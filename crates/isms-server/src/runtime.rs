//! Loading societies from the store, seeding new ones, and running actors
//! with their schedulers until shutdown (TDD 9.1, 9.2; `isms-server serve`).

use crate::actor::{self, ActorError, SocietyHandle};
use crate::chronicle::{Templates, spawn_projector};
use crate::scheduler::{self, Schedule};
use chrono::Utc;
use isms_core::config::ConfigError;
use isms_core::event::{Actor, Event};
use isms_core::kinds::ClientKind;
use isms_core::rules::Rules;
use isms_core::tick::start_epoch;
use isms_core::world::World;
use isms_core::{apply, load_preset_with_overrides};
use isms_store::{
    EventMeta, EventStore, Loaded, NewEvent, PgEventStore, SocietyRow, StoreError, load_world,
    verify_latest_snapshot,
};
use std::collections::BTreeMap;
use std::path::Path;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Actor(#[from] ActorError),
    #[error("society {0} is unknown")]
    UnknownSociety(i64),
    #[error("copy: {0}")]
    Copy(String),
}

/// What `isms-server seed` needs.
#[derive(Clone, Debug)]
pub struct SeedSpec {
    pub name: String,
    pub preset: String,
    pub seed: u64,
    pub tick_seconds: u32,
    pub cycle_boundary_hour: u32,
    /// `params.x.y = value` overrides applied before loading (tests, playtests).
    pub overrides: Vec<(String, toml::Value)>,
}

/// Create a society: its row, `SocietyCreated`, epoch 0 seeding, first snapshot.
pub async fn seed_society(
    store: &PgEventStore,
    presets_dir: &Path,
    spec: &SeedSpec,
) -> Result<SocietyRow, RuntimeError> {
    let preset = load_preset_with_overrides(presets_dir, &spec.preset, &spec.overrides)?;
    let id = store.next_society_id().await?;
    let row = SocietyRow {
        id,
        name: spec.name.clone(),
        preset: spec.preset.clone(),
        class: "canonical".into(),
        status: "active".into(),
        seed: i64::try_from(spec.seed).map_err(StoreError::from)?,
        epoch: 0,
        tick_seconds: i32::try_from(spec.tick_seconds).map_err(StoreError::from)?,
        tick_origin: Utc::now(),
        cycle_boundary_hour: i32::try_from(spec.cycle_boundary_hour).map_err(StoreError::from)?,
    };
    store.create_society(&row).await?;
    let society_id = u64::try_from(id).map_err(StoreError::from)?;
    let mut world = World::new(society_id, spec.seed, &preset);
    let created = Event::SocietyCreated {
        society_id,
        seed: spec.seed,
        preset: Box::new(preset),
    };
    let mut batch = vec![NewEvent {
        meta: EventMeta::at(&world, Some(Actor::System), Some(ClientKind::Sim)),
        event: created.clone(),
    }];
    apply(&mut world, &created);
    let rules = Rules::from_world(&world);
    for e in start_epoch(&world, &rules, 0) {
        batch.push(NewEvent {
            meta: EventMeta::at(&world, None, None),
            event: e.clone(),
        });
        apply(&mut world, &e);
    }
    let last_seq = i64::try_from(batch.len()).map_err(StoreError::from)? - 1;
    store.append_batch(id, 0, &batch).await?;
    store.write_snapshot(id, &world, last_seq).await?;
    tracing::info!(society = id, name = %spec.name, preset = %spec.preset, events = batch.len(), "society seeded");
    Ok(row)
}

/// Startup options.
#[derive(Clone, Debug, Default)]
pub struct StartOptions {
    /// Replay the whole log and compare with the latest snapshot before
    /// starting (TDD 9.1). Snapshot bytes are always verified against their hash.
    pub verify_replay: bool,
    /// Where the presets and their copy live; the Chronicle projector runs only when set.
    pub presets_dir: Option<std::path::PathBuf>,
}

/// A running society: its actor and scheduler tasks.
#[derive(Debug)]
pub struct Running {
    pub row: SocietyRow,
    pub handle: SocietyHandle,
    pub schedule: Schedule,
    actor_task: JoinHandle<()>,
    projector_task: Option<JoinHandle<()>>,
    scheduler_task: JoinHandle<Result<(), ActorError>>,
}

/// Load a society from the store and start its actor and scheduler.
pub async fn start_society(
    store: &PgEventStore,
    row: &SocietyRow,
    options: StartOptions,
    cancel: &CancellationToken,
) -> Result<Running, RuntimeError> {
    let templates = options
        .presets_dir
        .as_deref()
        .map(|dir| Templates::load(dir, &row.preset))
        .transpose()
        .map_err(RuntimeError::Copy)?;
    if options.verify_replay {
        if let Some(seq) = verify_latest_snapshot(store, row.id).await? {
            tracing::info!(society = row.id, seq, "snapshot verified against replay");
        } else {
            tracing::info!(society = row.id, "no snapshot to verify");
        }
    }
    let loaded: Loaded = load_world(store, row.id).await?;
    tracing::info!(
        society = row.id,
        name = %row.name,
        tick = loaded.world.meta.tick,
        seq = loaded.last_seq,
        "society loaded"
    );
    let schedule = Schedule {
        tick_seconds: u32::try_from(row.tick_seconds).unwrap_or(0),
        tick_origin: row.tick_origin,
    };
    let (handle, actor_task) = actor::spawn(row.id, store.clone(), loaded);
    let scheduler_task = tokio::spawn(scheduler::run(
        handle.clone(),
        schedule,
        cancel.child_token(),
    ));
    let projector_task =
        templates.map(|t| spawn_projector(store.clone(), handle.clone(), t, cancel.child_token()));
    Ok(Running {
        row: row.clone(),
        handle,
        schedule,
        actor_task,
        projector_task,
        scheduler_task,
    })
}

impl Running {
    /// Stop the scheduler, take a final snapshot, end the actor.
    pub async fn shutdown(self, cancel: &CancellationToken) -> Result<(), RuntimeError> {
        cancel.cancel();
        let _ = self.scheduler_task.await;
        if let Some(p) = self.projector_task {
            let _ = p.await;
        }
        let result = self.handle.shutdown().await;
        let _ = self.actor_task.await;
        result?;
        Ok(())
    }
}

/// Every active society, running.
#[derive(Debug)]
pub struct Runtime {
    pub societies: BTreeMap<i64, Running>,
    pub cancel: CancellationToken,
}

impl Runtime {
    pub async fn start(
        store: &PgEventStore,
        options: StartOptions,
    ) -> Result<Runtime, RuntimeError> {
        let cancel = CancellationToken::new();
        let mut societies = BTreeMap::new();
        for row in store.list_societies().await? {
            if row.status != "active" {
                tracing::info!(society = row.id, status = %row.status, "skipped");
                continue;
            }
            let running = start_society(store, &row, options.clone(), &cancel).await?;
            societies.insert(row.id, running);
        }
        Ok(Runtime { societies, cancel })
    }

    #[must_use]
    pub fn handle(&self, society: i64) -> Option<SocietyHandle> {
        self.societies.get(&society).map(|r| r.handle.clone())
    }

    /// Graceful shutdown: schedulers stop, every actor writes a final snapshot.
    pub async fn shutdown(self) -> Result<(), RuntimeError> {
        self.cancel.cancel();
        let mut first_err = None;
        for (id, running) in self.societies {
            if let Err(e) = running.shutdown(&self.cancel).await {
                tracing::error!(society = id, "shutdown: {e}");
                first_err.get_or_insert(e);
            }
        }
        first_err.map_or(Ok(()), Err)
    }
}
