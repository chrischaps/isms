//! Projection tables (TDD 8, S1.5): the Chronicle, regenerable from events,
//! and free-text messages (D12).

use crate::{PgEventStore, Result};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewHeadline {
    pub source_event_seq: i64,
    pub ordinal: i32,
    pub cycle: i32,
    pub tick: i32,
    pub headline: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadlineRow {
    pub source_event_seq: i64,
    pub ordinal: i32,
    pub cycle: i32,
    pub tick: i32,
    pub headline: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageRow {
    pub id: i64,
    pub channel: String,
    pub sender_citizen: i32,
    pub body: String,
    pub tick: i32,
    pub created_at: DateTime<Utc>,
}

impl PgEventStore {
    /// Idempotent: a headline already present for `(seq, ordinal)` is left alone.
    pub async fn insert_headlines(&self, society: i64, rows: &[NewHeadline]) -> Result<()> {
        for r in rows {
            sqlx::query!(
                "INSERT INTO chronicle (society_id, source_event_seq, ordinal, cycle, tick, headline)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (society_id, source_event_seq, ordinal) DO NOTHING",
                society,
                r.source_event_seq,
                r.ordinal,
                r.cycle,
                r.tick,
                r.headline
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn headlines_for_cycle(&self, society: i64, cycle: i32) -> Result<Vec<HeadlineRow>> {
        let rows = sqlx::query_as!(
            HeadlineRow,
            "SELECT source_event_seq, ordinal, cycle, tick, headline FROM chronicle
             WHERE society_id = $1 AND cycle = $2 ORDER BY source_event_seq, ordinal",
            society,
            cycle
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// The last `n` headlines, oldest first.
    pub async fn latest_headlines(&self, society: i64, n: i64) -> Result<Vec<HeadlineRow>> {
        let mut rows = sqlx::query_as!(
            HeadlineRow,
            "SELECT source_event_seq, ordinal, cycle, tick, headline FROM chronicle
             WHERE society_id = $1 ORDER BY source_event_seq DESC, ordinal DESC LIMIT $2",
            society,
            n
        )
        .fetch_all(&self.pool)
        .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn all_headlines(&self, society: i64) -> Result<Vec<HeadlineRow>> {
        let rows = sqlx::query_as!(
            HeadlineRow,
            "SELECT source_event_seq, ordinal, cycle, tick, headline FROM chronicle
             WHERE society_id = $1 ORDER BY source_event_seq, ordinal",
            society
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn clear_chronicle(&self, society: i64) -> Result<u64> {
        let done = sqlx::query!("DELETE FROM chronicle WHERE society_id = $1", society)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected())
    }

    pub async fn insert_message(
        &self,
        society: i64,
        channel: &str,
        sender: i32,
        body: &str,
        tick: i32,
    ) -> Result<MessageRow> {
        let row = sqlx::query_as!(
            MessageRow,
            "INSERT INTO messages (society_id, channel, sender_citizen, body, tick)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, channel, sender_citizen, body, tick, created_at",
            society,
            channel,
            sender,
            body,
            tick
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Messages with `id > after`, oldest first, at most `limit`.
    pub async fn messages(
        &self,
        society: i64,
        channel: &str,
        after: i64,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let rows = sqlx::query_as!(
            MessageRow,
            "SELECT id, channel, sender_citizen, body, tick, created_at FROM messages
             WHERE society_id = $1 AND channel = $2 AND id > $3 ORDER BY id LIMIT $4",
            society,
            channel,
            after,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
