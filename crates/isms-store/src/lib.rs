//! `isms-store`: Postgres persistence for societies (TDD 8).
//!
//! The event log is append-only and the society actor assigns `seq`; a
//! collision is the guard against two writers. Snapshots are postcard bytes
//! of `World` (zstd-compressed) with the blake3 hash of the uncompressed
//! bytes, so a loaded snapshot can be verified before any event is folded
//! onto it.

use chrono::{DateTime, Utc};
use isms_core::event::{Actor, Event};
use isms_core::ids::{Cycle, Epoch, Tick};
use isms_core::kinds::ClientKind;
use isms_core::world::World;
use isms_core::{Preset, apply};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub mod accounts;
pub mod archives;
pub mod projections;
pub mod reads;

/// Migrations embedded from `crates/isms-store/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("seq collision for society {society} at seq {seq}: another writer exists")]
    SeqCollision { society: i64, seq: i64 },
    #[error("snapshot for society {society} at seq {seq} does not match its stored hash")]
    SnapshotHashMismatch { society: i64, seq: i64 },
    #[error("snapshot for society {society} at seq {seq} was written by another World layout")]
    SnapshotStale { society: i64, seq: i64 },
    #[error("society {0} has no events")]
    NoEvents(i64),
    #[error("society {0}: first event is not SocietyCreated")]
    NotSocietyCreated(i64),
    #[error("event payload: {0}")]
    Json(#[from] serde_json::Error),
    #[error("snapshot bytes: {0}")]
    Postcard(#[from] postcard::Error),
    #[error("snapshot compression: {0}")]
    Zstd(#[from] std::io::Error),
    #[error("value out of range: {0}")]
    Range(#[from] std::num::TryFromIntError),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// The clock position and provenance stored next to each event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventMeta {
    pub tick: Tick,
    pub cycle: Cycle,
    pub epoch: Epoch,
    /// `None` for events produced by `tick` or `start_epoch`.
    pub actor: Option<Actor>,
    pub client_kind: Option<ClientKind>,
}

impl EventMeta {
    /// Metadata for an event emitted against `world` as it stands now.
    #[must_use]
    pub fn at(world: &World, actor: Option<Actor>, client_kind: Option<ClientKind>) -> Self {
        let tick = world.meta.tick;
        EventMeta {
            tick,
            cycle: world.cycle_of(tick),
            epoch: world.meta.epoch,
            actor,
            client_kind,
        }
    }
}

/// An event to append.
#[derive(Clone, Debug)]
pub struct NewEvent {
    pub meta: EventMeta,
    pub event: Event,
}

/// An event read back.
#[derive(Clone, Debug)]
pub struct StoredEvent {
    pub seq: i64,
    pub meta: EventMeta,
    pub event: Event,
    pub received_at: DateTime<Utc>,
}

/// A row of `societies`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocietyRow {
    pub id: i64,
    pub name: String,
    pub preset: String,
    pub class: String,
    pub status: String,
    pub seed: i64,
    pub epoch: i32,
    /// Wall-clock seconds per tick; 0 means as fast as possible (tests).
    pub tick_seconds: i32,
    /// Tick n is due at `tick_origin + n * tick_seconds`.
    pub tick_origin: DateTime<Utc>,
    /// Hour (UTC) at which cycles end (TDD 9.2); informational until S1.14.
    pub cycle_boundary_hour: i32,
}

/// A stored snapshot, still compressed.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub epoch: Epoch,
    pub tick: Tick,
    pub last_seq: i64,
    pub world_hash: [u8; 32],
    compressed: Vec<u8>,
}

impl Snapshot {
    /// Decompress, decode, and verify against the stored hash.
    ///
    /// Bytes that match their hash and still do not decode, or decode to a
    /// world that encodes differently, were written before `World` changed
    /// shape: `SnapshotStale`, which a loader answers by folding the log.
    pub fn world(&self, society: i64) -> Result<World> {
        let bytes = zstd::decode_all(self.compressed.as_slice())?;
        if *blake3::hash(&bytes).as_bytes() != self.world_hash {
            return Err(StoreError::SnapshotHashMismatch {
                society,
                seq: self.last_seq,
            });
        }
        match postcard::from_bytes::<World>(&bytes) {
            Ok(world) if world.hash() == self.world_hash => Ok(world),
            _ => Err(StoreError::SnapshotStale {
                society,
                seq: self.last_seq,
            }),
        }
    }
}

/// The persistence contract the society actor depends on (TDD 8, S1.1).
#[allow(async_fn_in_trait)]
pub trait EventStore {
    async fn create_society(&self, row: &SocietyRow) -> Result<()>;
    async fn list_societies(&self) -> Result<Vec<SocietyRow>>;
    /// `active`, `paused` or `archived` (S1.13c).
    async fn set_society_status(&self, society: i64, status: &str) -> Result<()>;
    /// The row's epoch after a rollover (S1.13d); the world is the authority.
    async fn set_society_epoch(&self, society: i64, epoch: i32) -> Result<()>;
    /// The clock after an operator re-anchors it (S1.13c).
    async fn set_society_schedule(
        &self,
        society: i64,
        tick_seconds: i32,
        tick_origin: DateTime<Utc>,
    ) -> Result<()>;
    /// One more than the highest society id (1 for an empty table).
    async fn next_society_id(&self) -> Result<i64>;
    /// Append `batch` as `first_seq..first_seq + batch.len()` in one transaction.
    async fn append_batch(&self, society: i64, first_seq: i64, batch: &[NewEvent]) -> Result<()>;
    /// `append_batch`, plus the epoch archive row when the batch ends an epoch
    /// (S1.15), in the same transaction: a crash leaves neither or both.
    async fn append_batch_archiving(
        &self,
        society: i64,
        first_seq: i64,
        batch: &[NewEvent],
        archive: Option<&archives::NewArchive>,
    ) -> Result<()>;
    /// Events with `seq > after`, in order, at most `limit`.
    async fn read_from(&self, society: i64, after: i64, limit: i64) -> Result<Vec<StoredEvent>>;
    async fn latest_snapshot(&self, society: i64) -> Result<Option<Snapshot>>;
    async fn write_snapshot(&self, society: i64, world: &World, last_seq: i64) -> Result<()>;
}

/// Postgres implementation.
#[derive(Clone, Debug)]
pub struct PgEventStore {
    pool: PgPool,
}

/// zstd level for snapshots; 3 is the library default and fast enough per cycle.
const SNAPSHOT_ZSTD_LEVEL: i32 = 3;
/// Page size used by the loader when streaming the tail.
const LOAD_PAGE: i64 = 5_000;

impl PgEventStore {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(8).connect(url).await?;
        Ok(PgEventStore { pool })
    }

    #[must_use]
    pub fn from_pool(pool: PgPool) -> Self {
        PgEventStore { pool }
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<()> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn row_to_event(
    seq: i64,
    tick: i32,
    cycle: i32,
    epoch: i32,
    actor: serde_json::Value,
    client_kind: serde_json::Value,
    payload: serde_json::Value,
    received_at: DateTime<Utc>,
) -> Result<StoredEvent> {
    Ok(StoredEvent {
        seq,
        meta: EventMeta {
            tick: Tick::try_from(tick)?,
            cycle: Cycle::try_from(cycle)?,
            epoch: Epoch::try_from(epoch)?,
            actor: serde_json::from_value(actor)?,
            client_kind: serde_json::from_value(client_kind)?,
        },
        event: serde_json::from_value(payload)?,
        received_at,
    })
}

impl EventStore for PgEventStore {
    async fn create_society(&self, row: &SocietyRow) -> Result<()> {
        sqlx::query!(
            "INSERT INTO societies (id, name, preset, class, status, seed, epoch,
                                    tick_seconds, tick_origin, cycle_boundary_hour)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            row.id,
            row.name,
            row.preset,
            row.class,
            row.status,
            row.seed,
            row.epoch,
            row.tick_seconds,
            row.tick_origin,
            row.cycle_boundary_hour
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_society_epoch(&self, society: i64, epoch: i32) -> Result<()> {
        sqlx::query!(
            "UPDATE societies SET epoch = $2 WHERE id = $1",
            society,
            epoch
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_society_status(&self, society: i64, status: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE societies SET status = $2 WHERE id = $1",
            society,
            status
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_society_schedule(
        &self,
        society: i64,
        tick_seconds: i32,
        tick_origin: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE societies SET tick_seconds = $2, tick_origin = $3 WHERE id = $1",
            society,
            tick_seconds,
            tick_origin
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_societies(&self) -> Result<Vec<SocietyRow>> {
        let rows = sqlx::query_as!(
            SocietyRow,
            "SELECT id, name, preset, class, status, seed, epoch,
                    tick_seconds, tick_origin, cycle_boundary_hour
             FROM societies ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn next_society_id(&self) -> Result<i64> {
        let row = sqlx::query!("SELECT COALESCE(MAX(id), 0) + 1 AS next FROM societies")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.next.unwrap_or(1))
    }

    async fn append_batch(&self, society: i64, first_seq: i64, batch: &[NewEvent]) -> Result<()> {
        self.append_batch_archiving(society, first_seq, batch, None)
            .await
    }

    async fn append_batch_archiving(
        &self,
        society: i64,
        first_seq: i64,
        batch: &[NewEvent],
        archive: Option<&archives::NewArchive>,
    ) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        let n = i64::try_from(batch.len())?;
        let seqs: Vec<i64> = (first_seq..first_seq + n).collect();
        let mut ticks = Vec::with_capacity(batch.len());
        let mut cycles = Vec::with_capacity(batch.len());
        let mut epochs = Vec::with_capacity(batch.len());
        let mut kinds = Vec::with_capacity(batch.len());
        let mut actors = Vec::with_capacity(batch.len());
        let mut client_kinds = Vec::with_capacity(batch.len());
        let mut payloads = Vec::with_capacity(batch.len());
        for e in batch {
            ticks.push(i32::try_from(e.meta.tick)?);
            cycles.push(i32::try_from(e.meta.cycle)?);
            epochs.push(i32::try_from(e.meta.epoch)?);
            kinds.push(e.event.kind().to_owned());
            actors.push(serde_json::to_value(e.meta.actor)?);
            client_kinds.push(serde_json::to_value(e.meta.client_kind)?);
            payloads.push(serde_json::to_value(&e.event)?);
        }
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query!(
            "INSERT INTO events (society_id, seq, tick, cycle, epoch, kind, actor, client_kind, payload)
             SELECT $1, u.seq, u.tick, u.cycle, u.epoch, u.kind, u.actor, u.client_kind, u.payload
             FROM UNNEST($2::bigint[], $3::int[], $4::int[], $5::int[], $6::text[], $7::jsonb[], $8::jsonb[], $9::jsonb[])
                  AS u(seq, tick, cycle, epoch, kind, actor, client_kind, payload)",
            society,
            &seqs,
            &ticks,
            &cycles,
            &epochs,
            &kinds,
            &actors,
            &client_kinds,
            &payloads
        )
        .execute(&mut *tx)
        .await;
        match result {
            Ok(_) => {
                if let Some(a) = archive {
                    sqlx::query!(
                        "INSERT INTO epoch_archives (society_id, epoch, reason, final_cycle, ended_seq, summary, closes_at)
                         VALUES ($1, $2, $3, $4, $5, $6, $7)",
                        society,
                        a.epoch,
                        a.reason,
                        a.final_cycle,
                        a.ended_seq,
                        a.summary,
                        a.closes_at
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
                Ok(())
            }
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                tx.rollback().await?;
                Err(StoreError::SeqCollision {
                    society,
                    seq: first_seq,
                })
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e.into())
            }
        }
    }

    async fn read_from(&self, society: i64, after: i64, limit: i64) -> Result<Vec<StoredEvent>> {
        let rows = sqlx::query!(
            "SELECT seq, tick, cycle, epoch, actor, client_kind, payload, received_at
             FROM events WHERE society_id = $1 AND seq > $2 ORDER BY seq LIMIT $3",
            society,
            after,
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

    async fn latest_snapshot(&self, society: i64) -> Result<Option<Snapshot>> {
        let row = sqlx::query!(
            "SELECT epoch, tick, last_seq, world_hash, world FROM snapshots
             WHERE society_id = $1 ORDER BY last_seq DESC LIMIT 1",
            society
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            let mut world_hash = [0u8; 32];
            if r.world_hash.len() == 32 {
                world_hash.copy_from_slice(&r.world_hash);
            }
            Ok(Snapshot {
                epoch: Epoch::try_from(r.epoch)?,
                tick: Tick::try_from(r.tick)?,
                last_seq: r.last_seq,
                world_hash,
                compressed: r.world,
            })
        })
        .transpose()
    }

    async fn write_snapshot(&self, society: i64, world: &World, last_seq: i64) -> Result<()> {
        let bytes = world.canonical_bytes();
        let hash = blake3::hash(&bytes).as_bytes().to_vec();
        let compressed = zstd::encode_all(bytes.as_slice(), SNAPSHOT_ZSTD_LEVEL)?;
        sqlx::query!(
            "INSERT INTO snapshots (society_id, epoch, tick, last_seq, world_hash, world)
             VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (society_id, last_seq) DO NOTHING",
            society,
            i32::try_from(world.meta.epoch)?,
            i32::try_from(world.meta.tick)?,
            last_seq,
            hash,
            compressed
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// A `World` rebuilt from the store plus the seq of the last event folded.
#[derive(Debug)]
pub struct Loaded {
    pub world: World,
    pub last_seq: i64,
}

/// Fold every event after `after` onto `world`, returning the last seq folded.
async fn fold_tail<S: EventStore>(
    store: &S,
    society: i64,
    world: &mut World,
    mut after: i64,
) -> Result<i64> {
    loop {
        let page = store.read_from(society, after, LOAD_PAGE).await?;
        let Some(last) = page.last() else {
            return Ok(after);
        };
        after = last.seq;
        for e in &page {
            apply(world, &e.event);
        }
    }
}

/// Build the initial `World` from a society's first event.
fn world_from_created(society: i64, first: &StoredEvent) -> Result<World> {
    match &first.event {
        Event::SocietyCreated {
            society_id,
            seed,
            preset,
        } => {
            let preset: &Preset = preset;
            let mut world = World::new(*society_id, *seed, preset);
            apply(&mut world, &first.event);
            Ok(world)
        }
        _ => Err(StoreError::NotSocietyCreated(society)),
    }
}

/// The latest snapshot's world, or `None` when there is no snapshot or it is
/// stale. The log is the truth and a snapshot only a shortcut to it, so a
/// stale one costs a full fold and nothing else; a corrupt one is still refused.
fn usable(snap: Option<Snapshot>, society: i64) -> Result<Option<(World, i64)>> {
    let Some(snap) = snap else { return Ok(None) };
    match snap.world(society) {
        Ok(world) => Ok(Some((world, snap.last_seq))),
        Err(StoreError::SnapshotStale { seq, .. }) => {
            tracing::warn!(
                society,
                seq,
                "snapshot predates this World layout; folding the whole log"
            );
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Latest snapshot (verified against its hash) plus the tail of events after
/// it; the whole log when there is no usable snapshot.
pub async fn load_world<S: EventStore>(store: &S, society: i64) -> Result<Loaded> {
    let (mut world, from) =
        if let Some(found) = usable(store.latest_snapshot(society).await?, society)? {
            found
        } else {
            let first = store.read_from(society, -1, 1).await?;
            let first = first.first().ok_or(StoreError::NoEvents(society))?;
            (world_from_created(society, first)?, first.seq)
        };
    let last_seq = fold_tail(store, society, &mut world, from).await?;
    Ok(Loaded { world, last_seq })
}

/// Fold the whole log from `SocietyCreated` up to `last_seq` and return the
/// world hash: the check that a snapshot equals its own history (TDD 9.1).
pub async fn replay_hash<S: EventStore>(
    store: &S,
    society: i64,
    last_seq: i64,
) -> Result<[u8; 32]> {
    let first = store.read_from(society, -1, 1).await?;
    let first = first.first().ok_or(StoreError::NoEvents(society))?;
    let mut world = world_from_created(society, first)?;
    let mut after = first.seq;
    while after < last_seq {
        let limit = (last_seq - after).min(LOAD_PAGE);
        let page = store.read_from(society, after, limit).await?;
        let Some(last) = page.last() else { break };
        after = last.seq;
        for e in &page {
            apply(&mut world, &e.event);
        }
    }
    Ok(world.hash())
}

/// Verify the latest snapshot against a full replay. `Ok(None)` when there is
/// no snapshot, or only a stale one, which `load_world` will not use either.
/// Refuse to start a society on `Err(SnapshotHashMismatch)`.
pub async fn verify_latest_snapshot<S: EventStore>(store: &S, society: i64) -> Result<Option<i64>> {
    let Some(snap) = store.latest_snapshot(society).await? else {
        return Ok(None);
    };
    if matches!(snap.world(society), Err(StoreError::SnapshotStale { .. })) {
        return Ok(None);
    }
    let replayed = replay_hash(store, society, snap.last_seq).await?;
    if replayed == snap.world_hash {
        Ok(Some(snap.last_seq))
    } else {
        Err(StoreError::SnapshotHashMismatch {
            society,
            seq: snap.last_seq,
        })
    }
}
