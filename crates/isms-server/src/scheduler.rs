//! The tick scheduler (TDD 9.2). Tick `n` is due at `tick_origin + n *
//! tick_seconds`; missed ticks after downtime run back-to-back, in order,
//! because each is a pure function of state. `tick_seconds = 0` runs ticks
//! as fast as the actor can resolve them (tests, the load test).

use crate::actor::{ActorError, SocietyHandle};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use isms_core::ids::Tick;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Notify;
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

/// The operator's hold on a society's clock (S1.13c, ADR-0007): pause and
/// resume without catch-up, a new tick length, and a wake-up for the
/// scheduler so a change takes effect at once.
#[derive(Debug)]
pub struct Control {
    paused: AtomicBool,
    schedule: RwLock<Schedule>,
    notify: Notify,
}

impl Control {
    #[must_use]
    pub fn new(schedule: Schedule, paused: bool) -> Self {
        Control {
            paused: AtomicBool::new(paused),
            schedule: RwLock::new(schedule),
            notify: Notify::new(),
        }
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn schedule(&self) -> Schedule {
        *self.schedule.read().expect("schedule lock")
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Wake the scheduler to look at the world again (after a rollover).
    pub fn wake(&self) {
        self.notify.notify_waiters();
    }

    /// Resume with the clock re-anchored so `next_tick` is due one tick length
    /// from `now`: a pause is not downtime, so nothing is caught up.
    pub fn resume(&self, next_tick: Tick, now: DateTime<Utc>) -> Schedule {
        let re = self.re_anchor(next_tick, now, None);
        self.paused.store(false, Ordering::SeqCst);
        self.notify.notify_waiters();
        re
    }

    /// A new tick length, re-anchored the same way; takes effect at once.
    pub fn set_tick_seconds(
        &self,
        tick_seconds: u32,
        next_tick: Tick,
        now: DateTime<Utc>,
    ) -> Schedule {
        let re = self.re_anchor(next_tick, now, Some(tick_seconds));
        self.notify.notify_waiters();
        re
    }

    fn re_anchor(
        &self,
        next_tick: Tick,
        now: DateTime<Utc>,
        tick_seconds: Option<u32>,
    ) -> Schedule {
        let mut s = self.schedule.write().expect("schedule lock");
        if let Some(t) = tick_seconds {
            s.tick_seconds = t;
        }
        // due_at(next_tick) == now + tick_seconds  <=>  origin == now - (next_tick - 1) * tick_seconds
        s.tick_origin = now
            - ChronoDuration::seconds(
                i64::from(next_tick.saturating_sub(1)) * i64::from(s.tick_seconds),
            );
        *s
    }
}

/// Drive one society's ticks until cancelled, the epoch ends, or the actor fails.
pub async fn run(
    handle: SocietyHandle,
    control: std::sync::Arc<Control>,
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
            // An ended society waits for an operator to start the next epoch
            // (S1.13d) or for S1.15's rollover sequence; either wakes the loop.
            tracing::info!(society = handle.id, "epoch ended; scheduler idle");
            tokio::select! {
                () = cancel.cancelled() => return Ok(()),
                () = control.notify.notified() => {}
            }
            continue;
        }
        if control.is_paused() {
            tokio::select! {
                () = cancel.cancelled() => return Ok(()),
                () = control.notify.notified() => {}
            }
            continue;
        }
        let schedule = control.schedule();
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
            () = control.notify.notified() => {}
            () = tokio::time::sleep(wait) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_re_anchors_so_nothing_is_caught_up() {
        use chrono::TimeZone;
        let origin = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let c = Control::new(
            Schedule {
                tick_seconds: 60,
                tick_origin: origin,
            },
            false,
        );
        c.pause();
        assert!(c.is_paused());
        // An hour later, tick 10 is next: it must be due one tick from now, not ten hours ago.
        let now = origin + ChronoDuration::hours(1);
        let s = c.resume(10, now);
        assert!(!c.is_paused());
        assert_eq!(s.due_at(10), now + ChronoDuration::seconds(60));
        assert!(!s.is_due(10, now));
        let s = c.set_tick_seconds(5, 10, now);
        assert_eq!(s.due_at(10), now + ChronoDuration::seconds(5));
    }

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
