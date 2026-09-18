//! The tick scheduler (TDD 9.2). Tick `n` is due at `tick_origin + n *
//! tick_seconds`; missed ticks after downtime run back-to-back, in order,
//! because each is a pure function of state. `tick_seconds = 0` runs ticks
//! as fast as the actor can resolve them (tests, the load test).

use crate::actor::{ActorError, SocietyHandle};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use isms_core::ids::Tick;
use isms_store::PgEventStore;
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

/// Drive one society's ticks until cancelled or the actor fails. An ended
/// epoch waits out its closing-statements window, then rolls over (S1.15).
pub async fn run(
    handle: SocietyHandle,
    control: std::sync::Arc<Control>,
    store: PgEventStore,
    cancel: CancellationToken,
) -> Result<(), ActorError> {
    let mut failures: u32 = 0;
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let (next_tick, ended, epoch) = {
            let world = handle.world.read().await;
            (
                world.meta.tick,
                world.meta.epoch_ended.is_some(),
                world.meta.epoch,
            )
        };
        if ended {
            match wait_out_the_window(&handle, &control, &store, epoch, &cancel).await {
                Wait::Cancelled => return Ok(()),
                Wait::Again => continue,
                Wait::Closed => {}
            }
            match crate::runtime::start_next_epoch(&store, handle.id, &handle, &control).await {
                Ok(Ok(epoch)) => {
                    tracing::warn!(
                        society = handle.id,
                        epoch,
                        "closing statements closed; the next epoch starts"
                    );
                }
                // The operator started it first: nothing to do.
                Ok(Err(reject)) => {
                    tracing::info!(society = handle.id, "rollover not needed: {}", reject.message);
                }
                Err(ActorError::Closed) => return Err(ActorError::Closed),
                Err(e) => {
                    tracing::error!(society = handle.id, "rollover failed: {e}; retrying");
                    metrics::counter!("isms_rollover_failures_total", "society" => handle.id.to_string()).increment(1);
                    tokio::select! {
                        () = cancel.cancelled() => return Ok(()),
                        () = tokio::time::sleep(MAX_SLEEP) => {}
                    }
                }
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
            match handle.tick().await {
                Ok(_) => failures = 0,
                Err(ActorError::Closed) => {
                    tracing::error!(
                        society = handle.id,
                        tick = next_tick,
                        "tick failed: the actor is gone"
                    );
                    return Err(ActorError::Closed);
                }
                // D13: a failed tick (a database pool timed out after the host slept, a
                // transient store error) is retried with backoff, never the end of the
                // society's clock. The actor survives every error but a seq collision,
                // which closes it and lands in the arm above on the next attempt.
                Err(e) => {
                    failures += 1;
                    let wait = retry_delay(failures, schedule.tick_seconds);
                    tracing::error!(
                        society = handle.id,
                        tick = next_tick,
                        attempt = failures,
                        retry_in_secs = wait.as_secs(),
                        "tick failed: {e}; retrying"
                    );
                    metrics::counter!("isms_tick_failures_total", "society" => handle.id.to_string()).increment(1);
                    tokio::select! {
                        () = cancel.cancelled() => return Ok(()),
                        () = tokio::time::sleep(wait) => {}
                    }
                    continue;
                }
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

enum Wait {
    Cancelled,
    /// Look at the world again (a wake-up, a held clock, a sleep that ended).
    Again,
    /// The window has closed: roll over.
    Closed,
}

/// An ended society sleeps until its closing-statements window closes
/// (S1.15). A held clock holds the rollover too; an ended epoch with no
/// archive row (one that ended before S1.15) waits for an operator, as before.
async fn wait_out_the_window(
    handle: &SocietyHandle,
    control: &Control,
    store: &PgEventStore,
    epoch: u32,
    cancel: &CancellationToken,
) -> Wait {
    if control.is_paused() {
        tracing::info!(society = handle.id, "epoch ended and the clock is held");
        tokio::select! {
            () = cancel.cancelled() => return Wait::Cancelled,
            () = control.notify.notified() => {}
        }
        return Wait::Again;
    }
    let archive = match store.latest_archive(handle.id).await {
        Ok(row) => row.filter(|r| i32::try_from(epoch).is_ok_and(|e| e == r.epoch)),
        Err(e) => {
            tracing::error!(society = handle.id, "reading the epoch archive: {e}; retrying");
            tokio::select! {
                () = cancel.cancelled() => return Wait::Cancelled,
                () = tokio::time::sleep(MAX_SLEEP) => {}
            }
            return Wait::Again;
        }
    };
    let Some(archive) = archive else {
        tracing::info!(society = handle.id, "epoch ended with no archive; waiting for an operator");
        tokio::select! {
            () = cancel.cancelled() => return Wait::Cancelled,
            () = control.notify.notified() => {}
        }
        return Wait::Again;
    };
    let now = Utc::now();
    if now >= archive.closes_at {
        return Wait::Closed;
    }
    let wait = (archive.closes_at - now)
        .to_std()
        .unwrap_or(Duration::ZERO)
        .min(MAX_SLEEP);
    tokio::select! {
        () = cancel.cancelled() => return Wait::Cancelled,
        () = control.notify.notified() => {}
        () = tokio::time::sleep(wait) => {}
    }
    Wait::Again
}

/// How long to wait before trying a failed tick again: doubling from one
/// second, never longer than a tick or thirty seconds, whichever is less
/// (but at least one second).
#[must_use]
pub fn retry_delay(attempt: u32, tick_seconds: u32) -> std::time::Duration {
    let cap = u64::from(tick_seconds.clamp(1, 30));
    let secs = 1u64 << attempt.saturating_sub(1).min(5);
    std::time::Duration::from_secs(secs.min(cap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_tick_is_retried_with_a_backoff_that_never_outlasts_a_tick() {
        // D13: one second, two, four ..., capped by the tick length.
        assert_eq!(retry_delay(1, 3600).as_secs(), 1);
        assert_eq!(retry_delay(2, 3600).as_secs(), 2);
        assert_eq!(retry_delay(3, 3600).as_secs(), 4);
        assert_eq!(retry_delay(9, 3600).as_secs(), 30, "thirty seconds at most");
        assert_eq!(retry_delay(4, 10).as_secs(), 8);
        assert_eq!(
            retry_delay(5, 10).as_secs(),
            10,
            "a ten-second tick caps at ten"
        );
        assert_eq!(
            retry_delay(3, 0).as_secs(),
            1,
            "as-fast-as-possible ticks still wait a second"
        );
    }

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
