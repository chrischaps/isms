//! Read queries over the event log for the API (S1.4): one event by seq,
//! events since a tick, the last n of a kind.

use crate::{PgEventStore, Result, StoredEvent, row_to_event};

impl PgEventStore {
    pub async fn read_one(&self, society: i64, seq: i64) -> Result<Option<StoredEvent>> {
        let row = sqlx::query!(
            "SELECT seq, tick, cycle, epoch, actor, client_kind, payload, received_at
             FROM events WHERE society_id = $1 AND seq = $2",
            society,
            seq
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            row_to_event(
                r.seq,
                r.tick,
                r.cycle,
                r.epoch,
                r.actor,
                r.client_kind,
                r.payload,
                r.received_at,
            )
        })
        .transpose()
    }

    /// Events with `tick >= since`, in log order, at most `limit`.
    pub async fn read_since_tick(
        &self,
        society: i64,
        since: u32,
        limit: i64,
    ) -> Result<Vec<StoredEvent>> {
        let since = i32::try_from(since)?;
        let rows = sqlx::query!(
            "SELECT seq, tick, cycle, epoch, actor, client_kind, payload, received_at
             FROM events WHERE society_id = $1 AND tick >= $2 ORDER BY seq LIMIT $3",
            society,
            since,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                row_to_event(
                    r.seq,
                    r.tick,
                    r.cycle,
                    r.epoch,
                    r.actor,
                    r.client_kind,
                    r.payload,
                    r.received_at,
                )
            })
            .collect()
    }

    /// The last `n` events of one kind, oldest first.
    /// How a citizen's commands reached the engine: events by `client_kind`
    /// for the commands this citizen sent (tick-produced events carry none).
    /// Public telemetry (TDD 13); the Observatory reports it per society.
    pub async fn action_share(
        &self,
        society: i64,
        citizen: u32,
    ) -> Result<Vec<(serde_json::Value, i64)>> {
        let actor = serde_json::json!({ "citizen": citizen });
        let rows = sqlx::query!(
            r#"SELECT client_kind, COUNT(*) AS "n!" FROM events
               WHERE society_id = $1 AND actor = $2 AND client_kind <> 'null'::jsonb
               GROUP BY client_kind ORDER BY client_kind"#,
            society,
            actor
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.client_kind, r.n)).collect())
    }

    pub async fn read_last_of_kind(
        &self,
        society: i64,
        kind: &str,
        n: i64,
    ) -> Result<Vec<StoredEvent>> {
        let rows = sqlx::query!(
            "SELECT seq, tick, cycle, epoch, actor, client_kind, payload, received_at
             FROM events WHERE society_id = $1 AND kind = $2 ORDER BY seq DESC LIMIT $3",
            society,
            kind,
            n
        )
        .fetch_all(&self.pool)
        .await?;
        let mut events: Vec<StoredEvent> = rows
            .into_iter()
            .map(|r| {
                row_to_event(
                    r.seq,
                    r.tick,
                    r.cycle,
                    r.epoch,
                    r.actor,
                    r.client_kind,
                    r.payload,
                    r.received_at,
                )
            })
            .collect::<Result<_>>()?;
        events.reverse();
        Ok(events)
    }

    /// Events of one kind with `tick >= since`, in log order, at most `limit`.
    pub async fn read_kind_since_tick(
        &self,
        society: i64,
        kind: &str,
        since: u32,
        limit: i64,
    ) -> Result<Vec<StoredEvent>> {
        let since = i32::try_from(since)?;
        let rows = sqlx::query!(
            "SELECT seq, tick, cycle, epoch, actor, client_kind, payload, received_at
             FROM events WHERE society_id = $1 AND kind = $2 AND tick >= $3 ORDER BY seq LIMIT $4",
            society,
            kind,
            since,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                row_to_event(
                    r.seq,
                    r.tick,
                    r.cycle,
                    r.epoch,
                    r.actor,
                    r.client_kind,
                    r.payload,
                    r.received_at,
                )
            })
            .collect()
    }
}
