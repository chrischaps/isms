//! The society actor (TDD 9.1): one task per loaded society, sole owner of
//! its `World`. Commands and ticks arrive on a mailbox; every batch of events
//! is persisted before it is applied, then broadcast to subscribers.
//!
//! Householders (TDD 9.3) run inside the tick message: their scripts execute
//! on a scratch clone of the world so that the persist-then-apply contract
//! holds for their events too.

use isms_core::command::{Command, Envelope, Reject, RejectCode};
use isms_core::event::{Actor, Event};
use isms_core::householder::run_round;
use isms_core::ids::{Cycle, Epoch, Tick};
use isms_core::kinds::ClientKind;
use isms_core::rules::Rules;
use isms_core::tick::{TickError, TickInput, start_epoch, tick};
use isms_core::world::World;
use isms_core::{apply, handle};
use isms_store::{EventMeta, EventStore, Loaded, NewEvent, PgEventStore, StoreError};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

/// Events the actor has persisted and applied, in order, starting at `first_seq`.
#[derive(Debug)]
pub struct Batch {
    pub first_seq: i64,
    /// The clock the events were stored under (the tick they belong to).
    pub tick: Tick,
    pub cycle: Cycle,
    pub events: Vec<Event>,
}

/// What an accepted command produced.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandOk {
    /// Log position of the first event; `None` when nothing was produced.
    pub first_seq: Option<i64>,
    pub tick: Tick,
    pub cycle: Cycle,
    pub events: Vec<Event>,
}

/// What one tick did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickOutcome {
    pub tick: Tick,
    pub events: usize,
    pub householder_rejections: usize,
    pub cycle_closed: bool,
    pub epoch_ended: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Tick(#[from] TickError),
    #[error("the society actor has stopped")]
    Closed,
}

pub enum ActorMsg {
    Command {
        envelope: Envelope<Command>,
        reply: oneshot::Sender<Result<Result<CommandOk, Reject>, ActorError>>,
    },
    Tick {
        reply: oneshot::Sender<Result<TickOutcome, ActorError>>,
    },
    Snapshot {
        reply: oneshot::Sender<Result<(), ActorError>>,
    },
    /// Operator: start the next epoch once this one has ended (S1.13d).
    NewEpoch {
        reply: oneshot::Sender<Result<Result<Epoch, Reject>, ActorError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), ActorError>>,
    },
}

impl std::fmt::Debug for ActorMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ActorMsg::Command { .. } => "Command",
            ActorMsg::Tick { .. } => "Tick",
            ActorMsg::Snapshot { .. } => "Snapshot",
            ActorMsg::NewEpoch { .. } => "NewEpoch",
            ActorMsg::Shutdown { .. } => "Shutdown",
        })
    }
}

/// The caller's side of a society actor. Cheap to clone.
#[derive(Clone, Debug)]
pub struct SocietyHandle {
    pub id: i64,
    /// Readers never wait on the database; the actor takes the write lock only during `apply`.
    pub world: Arc<RwLock<World>>,
    mailbox: mpsc::Sender<ActorMsg>,
    events: broadcast::Sender<Arc<Batch>>,
}

impl SocietyHandle {
    async fn send<T>(
        &self,
        make: impl FnOnce(oneshot::Sender<Result<T, ActorError>>) -> ActorMsg,
    ) -> Result<T, ActorError> {
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .send(make(tx))
            .await
            .map_err(|_| ActorError::Closed)?;
        rx.await.map_err(|_| ActorError::Closed)?
    }

    /// Validate and execute a command; `Ok(Err(reject))` is the engine saying no.
    pub async fn command(
        &self,
        envelope: Envelope<Command>,
    ) -> Result<Result<CommandOk, Reject>, ActorError> {
        self.send(|reply| ActorMsg::Command { envelope, reply })
            .await
    }

    /// Resolve the next tick (householder round first).
    pub async fn tick(&self) -> Result<TickOutcome, ActorError> {
        self.send(|reply| ActorMsg::Tick { reply }).await
    }

    /// Start the next epoch; `Err(Reject)` while the current one is still running.
    pub async fn new_epoch(&self) -> Result<Result<Epoch, Reject>, ActorError> {
        self.send(|reply| ActorMsg::NewEpoch { reply }).await
    }

    pub async fn snapshot(&self) -> Result<(), ActorError> {
        self.send(|reply| ActorMsg::Snapshot { reply }).await
    }

    /// Final snapshot, then the task ends.
    pub async fn shutdown(&self) -> Result<(), ActorError> {
        self.send(|reply| ActorMsg::Shutdown { reply }).await
    }

    /// Every persisted batch, in order, from now on.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Batch>> {
        self.events.subscribe()
    }
}

/// Mailbox depth before senders wait.
pub const MAILBOX_CAPACITY: usize = 1024;
/// Batches kept for slow stream subscribers before they see `Lagged`.
const BROADCAST_CAPACITY: usize = 256;

/// Start the actor for a loaded society.
pub fn spawn(id: i64, store: PgEventStore, loaded: Loaded) -> (SocietyHandle, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(MAILBOX_CAPACITY);
    let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
    let world = Arc::new(RwLock::new(loaded.world));
    let handle = SocietyHandle {
        id,
        world: Arc::clone(&world),
        mailbox: tx,
        events: events.clone(),
    };
    let actor = SocietyActor {
        id,
        store,
        world,
        rules: None,
        next_seq: loaded.last_seq + 1,
        events,
    };
    let task = tokio::spawn(actor.run(rx));
    (handle, task)
}

struct SocietyActor {
    id: i64,
    store: PgEventStore,
    world: Arc<RwLock<World>>,
    /// Derived once per epoch (TDD 5.2); rebuilt after a rollover.
    rules: Option<(Epoch, Rules)>,
    next_seq: i64,
    events: broadcast::Sender<Arc<Batch>>,
}

fn is_fatal<T>(result: &Result<T, ActorError>) -> bool {
    matches!(
        result,
        Err(ActorError::Store(StoreError::SeqCollision { .. }))
    )
}

impl SocietyActor {
    async fn run(mut self, mut rx: mpsc::Receiver<ActorMsg>) {
        let society = self.id;
        tracing::info!(society, next_seq = self.next_seq, "society actor started");
        while let Some(msg) = rx.recv().await {
            metrics::gauge!("isms_actor_mailbox_depth", "society" => society.to_string())
                .set(u32::try_from(rx.len()).map_or(f64::MAX, f64::from));
            match msg {
                ActorMsg::Command { envelope, reply } => {
                    let result = self.command(envelope).await;
                    let fatal = is_fatal(&result);
                    let _ = reply.send(result);
                    if fatal {
                        break;
                    }
                }
                ActorMsg::Tick { reply } => {
                    let result = self.resolve_tick().await;
                    let fatal = is_fatal(&result);
                    let _ = reply.send(result);
                    if fatal {
                        break;
                    }
                }
                ActorMsg::Snapshot { reply } => {
                    let _ = reply.send(self.snapshot().await);
                }
                ActorMsg::NewEpoch { reply } => {
                    let result = self.new_epoch().await;
                    let fatal = is_fatal(&result);
                    let _ = reply.send(result);
                    if fatal {
                        break;
                    }
                }
                ActorMsg::Shutdown { reply } => {
                    let _ = reply.send(self.snapshot().await);
                    break;
                }
            }
        }
        tracing::info!(society, "society actor stopped");
    }

    fn rules(&mut self, world: &World) -> &Rules {
        let epoch = world.meta.epoch;
        if self.rules.as_ref().is_none_or(|(e, _)| *e != epoch) {
            self.rules = Some((epoch, Rules::from_world(world)));
        }
        &self.rules.as_ref().expect("just set").1
    }

    /// Persist, apply, broadcast: the single-writer contract (TDD 5.1).
    async fn commit(&mut self, batch: Vec<NewEvent>) -> Result<Option<i64>, ActorError> {
        if batch.is_empty() {
            return Ok(None);
        }
        let first_seq = self.next_seq;
        let (tick, cycle) = (batch[0].meta.tick, batch[0].meta.cycle);
        if let Err(e) = self.store.append_batch(self.id, first_seq, &batch).await {
            if matches!(e, StoreError::SeqCollision { .. }) {
                tracing::error!(
                    society = self.id,
                    seq = first_seq,
                    "{e}; stopping this actor"
                );
            }
            return Err(e.into());
        }
        let events: Vec<Event> = batch.into_iter().map(|e| e.event).collect();
        {
            let mut world = self.world.write().await;
            for e in &events {
                apply(&mut world, e);
            }
        }
        self.next_seq += i64::try_from(events.len()).expect("batch fits in i64");
        let _ = self.events.send(Arc::new(Batch {
            first_seq,
            tick,
            cycle,
            events,
        }));
        Ok(Some(first_seq))
    }

    async fn command(
        &mut self,
        mut envelope: Envelope<Command>,
    ) -> Result<Result<CommandOk, Reject>, ActorError> {
        let started = Instant::now();
        let kind = envelope.command.kind();
        let world_arc = Arc::clone(&self.world);
        let (outcome, meta) = {
            let world = world_arc.read().await;
            envelope.received_at_tick = world.meta.tick;
            let meta = EventMeta::at(&world, Some(envelope.actor), Some(envelope.client_kind));
            let rules = self.rules(&world);
            (handle(&world, rules, &envelope), meta)
        };
        let result = match outcome {
            Err(reject) => {
                metrics::counter!(
                    "isms_commands_rejected_total",
                    "society" => self.id.to_string(),
                    "code" => format!("{:?}", reject.code)
                )
                .increment(1);
                tracing::debug!(society = self.id, actor = ?envelope.actor, kind, code = ?reject.code, "rejected");
                Ok(Err(reject))
            }
            Ok(events) => {
                let batch: Vec<NewEvent> = events
                    .iter()
                    .cloned()
                    .map(|event| NewEvent {
                        meta: meta.clone(),
                        event,
                    })
                    .collect();
                let first_seq = self.commit(batch).await?;
                Ok(Ok(CommandOk {
                    first_seq,
                    tick: meta.tick,
                    cycle: meta.cycle,
                    events,
                }))
            }
        };
        metrics::counter!("isms_commands_total", "society" => self.id.to_string(), "kind" => kind)
            .increment(1);
        metrics::histogram!("isms_command_seconds", "society" => self.id.to_string())
            .record(started.elapsed().as_secs_f64());
        result
    }

    /// The rollover (TDD S0.13, driven by hand until S1.15's sequence): the
    /// engine's `start_epoch` for the next number, one batch, metas taken
    /// event by event against a scratch clone as the seeding at creation does.
    async fn new_epoch(&mut self) -> Result<Result<Epoch, Reject>, ActorError> {
        let mut scratch = self.world.read().await.clone();
        if scratch.meta.epoch_ended.is_none() {
            return Ok(Err(Reject::new(
                RejectCode::EpochEnded,
                "the epoch is still running; end it first",
            )));
        }
        let next = scratch.meta.epoch + 1;
        let rules = self.rules(&scratch).clone();
        let mut batch = Vec::new();
        for e in start_epoch(&scratch, &rules, next) {
            batch.push(NewEvent {
                meta: EventMeta::at(&scratch, None, None),
                event: e.clone(),
            });
            apply(&mut scratch, &e);
        }
        let n = batch.len();
        self.commit(batch).await?;
        tracing::warn!(
            society = self.id,
            epoch = next,
            events = n,
            "epoch started by operator"
        );
        Ok(Ok(next))
    }

    async fn resolve_tick(&mut self) -> Result<TickOutcome, ActorError> {
        let started = Instant::now();
        // Householders act on a scratch clone so their events are persisted
        // before the real world sees them (TDD 9.3, D8). The round and the
        // tick are one batch: one transaction, one commit per tick.
        let mut scratch = self.world.read().await.clone();
        let now = scratch.meta.tick;
        let rules = self.rules(&scratch).clone();
        let round_meta = EventMeta::at(&scratch, None, Some(ClientKind::Householder));
        let (round, rejected) = run_round(&mut scratch, &rules, now);
        for r in &rejected {
            tracing::warn!(society = self.id, citizen = r.citizen.0, code = ?r.reject.code, "householder command rejected");
        }
        metrics::counter!("isms_householder_rejections_total", "society" => self.id.to_string())
            .increment(u64::try_from(rejected.len()).unwrap_or(u64::MAX));
        let input = TickInput::next_for(&scratch);
        let events = tick(&scratch, &rules, input)?;
        let meta = EventMeta::at(&scratch, None, None);
        let cycle_closed = events
            .iter()
            .any(|e| matches!(e, Event::CycleClosed { .. }));
        let epoch_ended = events.iter().any(|e| matches!(e, Event::EpochEnded { .. }));
        let batch: Vec<NewEvent> = round
            .into_iter()
            .map(|event| NewEvent {
                meta: round_meta.clone(),
                event,
            })
            .chain(events.into_iter().map(|event| NewEvent {
                meta: meta.clone(),
                event,
            }))
            .collect();
        let count = batch.len();
        self.commit(batch).await?;
        if cycle_closed || epoch_ended {
            self.snapshot().await?;
        }
        let elapsed = started.elapsed();
        metrics::histogram!("isms_tick_seconds", "society" => self.id.to_string())
            .record(elapsed.as_secs_f64());
        tracing::info!(
            society = self.id,
            tick = now,
            events = count,
            rejected = rejected.len(),
            ms = elapsed.as_millis(),
            "tick resolved"
        );
        Ok(TickOutcome {
            tick: now,
            events: count,
            householder_rejections: rejected.len(),
            cycle_closed,
            epoch_ended,
        })
    }

    async fn snapshot(&mut self) -> Result<(), ActorError> {
        let last_seq = self.next_seq - 1;
        let world = self.world.read().await;
        self.store.write_snapshot(self.id, &world, last_seq).await?;
        tracing::debug!(
            society = self.id,
            seq = last_seq,
            tick = world.meta.tick,
            "snapshot written"
        );
        Ok(())
    }
}

/// Build the envelope the API will send: the actor overwrites `received_at_tick`.
#[must_use]
pub fn envelope(actor: Actor, client_kind: ClientKind, command: Command) -> Envelope<Command> {
    Envelope {
        actor,
        on_behalf_of: None,
        client_kind,
        received_at_tick: 0,
        command,
    }
}
