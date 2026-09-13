//! S1.2 done gate: 100 concurrent commands keep conservation; killing and
//! restarting after 5 ticks replays to the same `world_hash`; a 3-tick outage
//! catches up in order; `tick_seconds = 0` runs an epoch in seconds.

use chrono::{Duration as ChronoDuration, Utc};
use isms_core::command::Command;
use isms_core::event::{Actor, Event};
use isms_core::ids::CitizenId;
use isms_core::kinds::{CitizenKind, ClientKind};
use isms_core::ledger::{Asset, Party};
use isms_core::money::Money;
use isms_core::{WORKSPACE_PRESETS_DIR, conservation_check};
use isms_server::actor::{SocietyHandle, envelope, spawn};
use isms_server::runtime::{Runtime, SeedSpec, StartOptions, seed_society, start_society};
use isms_server::scheduler::{self, Schedule};
use isms_store::{EventStore, PgEventStore, SocietyRow, load_world};
use sqlx::PgPool;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

fn spec(tick_seconds: u32) -> SeedSpec {
    SeedSpec {
        name: "freeport-test".into(),
        preset: "freeport".into(),
        seed: 1,
        tick_seconds,
        cycle_boundary_hour: 4,
        overrides: vec![(
            "params.population.collapse_enabled".to_owned(),
            toml::Value::Boolean(false),
        )],
    }
}

/// Throwaway test servers need no durability: on Docker Desktop the WAL
/// flush behind each synchronous commit stalls for tens of milliseconds,
/// which turned a 3-second epoch into two minutes. Server-wide, because the
/// pool's connections predate any per-database setting. Production keeps
/// the default; the dev compose file sets the same flag.
async fn fast_commits(pool: &PgPool) {
    sqlx::query("ALTER SYSTEM SET synchronous_commit = 'off'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("SELECT pg_reload_conf()")
        .execute(pool)
        .await
        .unwrap();
}

async fn seeded(pool: PgPool, tick_seconds: u32) -> (PgEventStore, SocietyRow) {
    fast_commits(&pool).await;
    let store = PgEventStore::from_pool(pool);
    let row = seed_society(
        &store,
        Path::new(WORKSPACE_PRESETS_DIR),
        &spec(tick_seconds),
    )
    .await
    .expect("seed");
    (store, row)
}

async fn actor(store: &PgEventStore, id: i64) -> SocietyHandle {
    let loaded = load_world(store, id).await.unwrap();
    let (handle, _task) = spawn(id, store.clone(), loaded);
    handle
}

async fn wait_until(deadline: Duration, mut done: impl AsyncFnMut() -> bool) {
    let start = Instant::now();
    while !done().await {
        assert!(start.elapsed() < deadline, "timed out after {deadline:?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn hundred_concurrent_commands_keep_conservation(pool: PgPool) {
    let (store, row) = seeded(pool, 0).await;
    let handle = actor(&store, row.id).await;
    // 100 humans join at once (Q10: System sends Join for a not-yet citizen).
    let joins: Vec<_> = (0..100)
        .map(|i| {
            let h = handle.clone();
            tokio::spawn(async move {
                h.command(envelope(
                    Actor::System,
                    ClientKind::Sim,
                    Command::Join {
                        handle: format!("human-{i}"),
                        kind: CitizenKind::Human,
                    },
                ))
                .await
                .unwrap()
                .unwrap()
            })
        })
        .collect();
    let mut humans = Vec::new();
    for j in joins {
        for e in j.await.unwrap() {
            if let Event::CitizenJoined { citizen, .. } = e {
                humans.push(citizen);
            }
        }
    }
    assert_eq!(humans.len(), 100);
    // Then 100 concurrent transfers around a ring, from web sessions.
    let transfers: Vec<_> = (0..100)
        .map(|i| {
            let h = handle.clone();
            let from: CitizenId = humans[i];
            let to: CitizenId = humans[(i + 1) % 100];
            tokio::spawn(async move {
                h.command(envelope(
                    Actor::Citizen(from),
                    ClientKind::Web,
                    Command::Transfer {
                        to: Party::Citizen(to),
                        asset: Asset::Money(Money::cents(100)),
                        memo: "ring".into(),
                    },
                ))
                .await
                .unwrap()
            })
        })
        .collect();
    let mut accepted = 0;
    for t in transfers {
        if t.await.unwrap().is_ok() {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 100);
    let world = handle.world.read().await;
    conservation_check(&world).expect("conserved");
    assert_eq!(
        world
            .citizens
            .values()
            .filter(|c| c.kind == CitizenKind::Human)
            .count(),
        100
    );
    // Persist-then-apply: the log folds to exactly the live world, seqs contiguous.
    let loaded = load_world(&store, row.id).await.unwrap();
    assert_eq!(loaded.world.hash(), world.hash());
    let all = store.read_from(row.id, -1, 100_000).await.unwrap();
    assert_eq!(all.last().unwrap().seq, loaded.last_seq);
    assert!(
        all.iter()
            .enumerate()
            .all(|(i, e)| e.seq == i64::try_from(i).unwrap())
    );
    // A web command was stamped as such.
    let web = all
        .iter()
        .filter(|e| e.meta.client_kind == Some(ClientKind::Web))
        .count();
    assert_eq!(web, 100);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn kill_and_restart_after_five_ticks_replays_to_the_same_hash(pool: PgPool) {
    let (store, row) = seeded(pool, 0).await;
    let loaded = load_world(&store, row.id).await.unwrap();
    let (handle, task) = spawn(row.id, store.clone(), loaded);
    for _ in 0..5 {
        handle.tick().await.unwrap();
    }
    let live_hash = handle.world.read().await.hash();
    assert_eq!(handle.world.read().await.meta.tick, 5);
    // Kill: no shutdown, no final snapshot.
    task.abort();
    drop(handle);
    let reloaded = load_world(&store, row.id).await.unwrap();
    assert_eq!(reloaded.world.hash(), live_hash);
    assert_eq!(reloaded.world.meta.tick, 5);
    // The restarted actor continues from the same state and seq.
    let (handle, _task) = spawn(row.id, store.clone(), reloaded);
    let out = handle.tick().await.unwrap();
    assert_eq!(out.tick, 5);
    let again = load_world(&store, row.id).await.unwrap();
    assert_eq!(again.world.hash(), handle.world.read().await.hash());
    // Graceful shutdown writes a snapshot that loads to the same bytes.
    handle.shutdown().await.unwrap();
    let snap = store.latest_snapshot(row.id).await.unwrap().unwrap();
    assert_eq!(snap.last_seq, again.last_seq);
    assert_eq!(snap.world(row.id).unwrap().hash(), again.world.hash());
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn three_tick_outage_catches_up_in_order(pool: PgPool) {
    let (store, row) = seeded(pool, 1).await;
    let handle = actor(&store, row.id).await;
    // The server was "down" for three ticks: ticks 0, 1, 2 are already due,
    // tick 3 is due half a second from now.
    let schedule = Schedule {
        tick_seconds: 1,
        tick_origin: Utc::now() - ChronoDuration::milliseconds(2_500),
    };
    let cancel = CancellationToken::new();
    let task = tokio::spawn(scheduler::run(handle.clone(), schedule, cancel.clone()));
    let h = handle.clone();
    wait_until(Duration::from_secs(10), async || {
        h.world.read().await.meta.tick >= 4
    })
    .await;
    cancel.cancel();
    task.await.unwrap().unwrap();
    let ticks: Vec<u32> = store
        .read_from(row.id, -1, 100_000)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|e| match e.event {
            Event::TickResolved { tick, .. } => Some(tick),
            _ => None,
        })
        .collect();
    assert_eq!(&ticks[..4], &[0, 1, 2, 3]);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn tick_seconds_zero_runs_an_epoch(pool: PgPool) {
    let (store, row) = seeded(pool, 0).await;
    let cancel = CancellationToken::new();
    let running = start_society(&store, &row, StartOptions::default(), &cancel)
        .await
        .unwrap();
    let h = running.handle.clone();
    let started = Instant::now();
    wait_until(Duration::from_secs(300), async || {
        h.world.read().await.meta.epoch_ended.is_some()
    })
    .await;
    let elapsed = started.elapsed();
    let (tick, ticks_per_epoch) = {
        let w = h.world.read().await;
        (w.meta.tick, w.params.ticks_per_epoch())
    };
    assert_eq!(tick, ticks_per_epoch);
    running.shutdown(&cancel).await.unwrap();
    // Snapshot per cycle end plus the seeding one and the final one.
    let snapshots: i64 = sqlx::query_scalar("SELECT count(*) FROM snapshots WHERE society_id = $1")
        .bind(row.id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert!(snapshots >= 43, "snapshots = {snapshots}");
    let loaded = load_world(&store, row.id).await.unwrap();
    assert_eq!(loaded.world.hash(), h.world.read().await.hash());
    conservation_check(&loaded.world).unwrap();
    eprintln!("epoch of {ticks_per_epoch} ticks in {elapsed:?}");
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn runtime_starts_every_active_society(pool: PgPool) {
    let (store, a) = seeded(pool, 3600).await;
    let b = seed_society(
        &store,
        Path::new(WORKSPACE_PRESETS_DIR),
        &SeedSpec {
            name: "second".into(),
            ..spec(3600)
        },
    )
    .await
    .unwrap();
    let runtime = Runtime::start(
        &store,
        StartOptions {
            verify_replay: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        runtime.societies.keys().copied().collect::<Vec<_>>(),
        vec![a.id, b.id]
    );
    assert!(runtime.handle(a.id).is_some());
    runtime.shutdown().await.unwrap();
}
