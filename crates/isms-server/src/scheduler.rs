//! The tick scheduler (TDD 9.2). Tick `n` is due at `tick_origin + n *
//! tick_seconds`; missed ticks after downtime run back-to-back, in order,
//! because each is a pure function of state. `tick_seconds = 0` runs ticks
//! as fast as the actor can resolve them (tests, the load test).

use crate::actor::{ActorError, SocietyHandle};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use isms_core::ids::Tick;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// A society's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub tick_seconds: u32,
    pub tick_origin: DateTime<Utc>,
}

/// Longest sleep between scheduler wake-ups (TDD 9.2: "wakes each minute").
pub const MAX_SLEEP: Duration = Duration::from_secs(60);

impl Schedule {
    /// When tick `n` becomes due.
    #[must_use]
    pub fn due_at(&self, tick: Tick) -> DateTime<Utc> {
        self.tick_origin + ChronoDuration::seconds(i64::from(tick) * i64::from(self.tick_seconds))
    }

    #[must_use]
    pub fn is_due(&self, tick: Tick, now: DateTime<Utc>) -> bool {
        self.tick_seconds == 0 || now >= self.due_at(tick)
    }

    /// How long to wait before tick `n` is due, capped at [`MAX_SLEEP`].
    #[must_use]
    pub fn wait_for(&self, tick: Tick, now: DateTime<Utc>) -> Duration {
        let until = (self.due_at(tick) - now).to_std().unwrap_or(Duration::ZERO);
        until.min(MAX_SLEEP)
    }
}

/// Drive one society's ticks until cancelled, the epoch ends, or the actor fails.
pub async fn run(
    handle: SocietyHandle,
    schedule: Schedule,
    cancel: CancellationToken,
) -> Result<(), ActorError> {
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let (next_tick, ended) = {
            let world = handle.world.read().await;
            (world.meta.tick, world.meta.epoch_ended.is_some())
        };
        if ended {
            // Epoch rollover is S1.15's; until then an ended society just waits.
            tracing::info!(society = handle.id, "epoch ended; scheduler idle");
            cancel.cancelled().await;
            return Ok(());
        }
        let now = Utc::now();
        if schedule.is_due(next_tick, now) {
            if let Err(e) = handle.tick().await {
                tracing::error!(society = handle.id, tick = next_tick, "tick failed: {e}");
                return Err(e);
            }
            // Let commands interleave between back-to-back ticks.
            tokio::task::yield_now().await;
            continue;
        }
        let wait = schedule.wait_for(next_tick, now);
        tokio::select! {
            () = cancel.cancelled() => return Ok(()),
            () = tokio::time::sleep(wait) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_times_and_catch_up() {
        let origin = DateTime::parse_from_rfc3339("2026-09-13T04:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let s = Schedule {
            tick_seconds: 3600,
            tick_origin: origin,
        };
        assert_eq!(s.due_at(0), origin);
        assert_eq!(s.due_at(24), origin + ChronoDuration::hours(24));
        let now = origin + ChronoDuration::hours(3) + ChronoDuration::minutes(1);
        assert!(s.is_due(3, now));
        assert!(!s.is_due(4, now));
        assert_eq!(
            s.wait_for(4, now),
            Duration::from_secs(59 * 60).min(MAX_SLEEP)
        );
        assert_eq!(s.wait_for(3, now), Duration::ZERO);
        let fast = Schedule {
            tick_seconds: 0,
            tick_origin: origin,
        };
        assert!(fast.is_due(1_000, origin - ChronoDuration::days(1)));
    }
}
