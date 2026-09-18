//! Epoch archives (S1.15, GDD 11.5): the row an ended epoch leaves behind and
//! the closing statements citizens add to it while the window is open. The
//! row is inserted by `append_batch_archiving` in the transaction that appends
//! `EpochEnded`; everything here is a read or an edit of that row.

use crate::{PgEventStore, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One archived epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveRow {
    pub society_id: i64,
    /// The engine's 0-based epoch number.
    pub epoch: i32,
    pub ended_at: DateTime<Utc>,
    /// `scheduled`, `collapse` or `operator`.
    pub reason: String,
    /// The engine's 0-based final cycle.
    pub final_cycle: i32,
    /// The `EpochEnded` event's seq.
    pub ended_seq: i64,
    /// The engine's `EpochSummary`, as serialized on the event.
    pub summary: serde_json::Value,
    /// When closing statements stop being accepted; the rollover closes them early.
    pub closes_at: DateTime<Utc>,
    /// `Vec<ClosingStatement>` as JSON, in arrival order.
    pub closing_statements: serde_json::Value,
}

impl ArchiveRow {
    /// The statements, oldest first; a malformed column reads as none.
    #[must_use]
    pub fn statements(&self) -> Vec<ClosingStatement> {
        serde_json::from_value(self.closing_statements.clone()).unwrap_or_default()
    }

    /// Whether a statement written at `now` would still be accepted.
    #[must_use]
    pub fn is_open(&self, now: DateTime<Utc>) -> bool {
        now < self.closes_at
    }
}

/// What the actor hands the store beside an `EpochEnded`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewArchive {
    pub epoch: i32,
    pub reason: String,
    pub final_cycle: i32,
    pub ended_seq: i64,
    pub summary: serde_json::Value,
    pub closes_at: DateTime<Utc>,
}

/// One citizen's closing statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosingStatement {
    pub citizen: u32,
    pub handle: String,
    pub text: String,
    pub written_at: DateTime<Utc>,
}

impl PgEventStore {
    /// Every archived epoch of a society, oldest first.
    pub async fn list_archives(&self, society: i64) -> Result<Vec<ArchiveRow>> {
        let rows = sqlx::query_as!(
            ArchiveRow,
            "SELECT society_id, epoch, ended_at, reason, final_cycle, ended_seq, summary,
                    closes_at, closing_statements
             FROM epoch_archives WHERE society_id = $1 ORDER BY epoch",
            society
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// One archived epoch (the engine's 0-based number).
    pub async fn archive(&self, society: i64, epoch: i32) -> Result<Option<ArchiveRow>> {
        let row = sqlx::query_as!(
            ArchiveRow,
            "SELECT society_id, epoch, ended_at, reason, final_cycle, ended_seq, summary,
                    closes_at, closing_statements
             FROM epoch_archives WHERE society_id = $1 AND epoch = $2",
            society,
            epoch
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// The most recently ended epoch, whose statements window may still be open.
    pub async fn latest_archive(&self, society: i64) -> Result<Option<ArchiveRow>> {
        let row = sqlx::query_as!(
            ArchiveRow,
            "SELECT society_id, epoch, ended_at, reason, final_cycle, ended_seq, summary,
                    closes_at, closing_statements
             FROM epoch_archives WHERE society_id = $1 ORDER BY epoch DESC LIMIT 1",
            society
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Write or replace one citizen's statement on an archive. The caller has
    /// checked the window; this only keeps one entry per citizen, in the order
    /// citizens first spoke. Returns the row as it now stands, or `None` when
    /// there is no such archive.
    pub async fn write_closing_statement(
        &self,
        society: i64,
        epoch: i32,
        statement: &ClosingStatement,
    ) -> Result<Option<ArchiveRow>> {
        let mut tx = self.pool.begin().await?;
        let current = sqlx::query_scalar!(
            "SELECT closing_statements FROM epoch_archives
             WHERE society_id = $1 AND epoch = $2 FOR UPDATE",
            society,
            epoch
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(current) = current else {
            tx.rollback().await?;
            return Ok(None);
        };
        let mut statements: Vec<ClosingStatement> =
            serde_json::from_value(current).unwrap_or_default();
        match statements
            .iter_mut()
            .find(|s| s.citizen == statement.citizen)
        {
            Some(mine) => *mine = statement.clone(),
            None => statements.push(statement.clone()),
        }
        let json = serde_json::to_value(&statements)?;
        sqlx::query!(
            "UPDATE epoch_archives SET closing_statements = $3
             WHERE society_id = $1 AND epoch = $2",
            society,
            epoch,
            json
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.archive(society, epoch).await
    }

    /// The window closes now if it has not already (the rollover, or an
    /// operator starting the next epoch early).
    pub async fn close_statements(
        &self,
        society: i64,
        epoch: i32,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE epoch_archives SET closes_at = LEAST(closes_at, $3)
             WHERE society_id = $1 AND epoch = $2",
            society,
            epoch,
            now
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
