//! S1.1 done gate: append/read round-trip; snapshot + tail equals the full fold
//! on a 3-cycle sim; a seq collision fails the transaction; the app role cannot
//! UPDATE or DELETE events. Runs against `DATABASE_URL` (`sqlx::test` makes a
//! throwaway database per test and applies the migrations).

use isms_core::event::{Actor, Event};
use isms_core::householder::run_round;
use isms_core::kinds::ClientKind;
use isms_core::rules::Rules;
use isms_core::tick::{TickInput, start_epoch, tick};
use isms_core::world::World;
use isms_core::{WORKSPACE_PRESETS_DIR, apply, load_preset_with_overrides};
use isms_store::{
    EventMeta, EventStore, NewEvent, PgEventStore, SocietyRow, StoreError, load_world, replay_hash,
    verify_latest_snapshot,
};
use sqlx::PgPool;
use std::path::Path;

const SOCIETY: i64 = 7;

fn society_row() -> SocietyRow {
    SocietyRow {
        id: SOCIETY,
        name: "freeport-test".into(),
        preset: "freeport".into(),
        class: "canonical".into(),
        status: "active".into(),
        seed: 7,
        epoch: 0,
        tick_seconds: 0,
        tick_origin: chrono::DateTime::UNIX_EPOCH,
        cycle_boundary_hour: 4,
    }
}

/// A householder-only Freeport for `cycles` cycles, as the simulator runs it,
/// returning the live world and every event with its metadata, batched per
/// tick (the shape the actor will persist).
fn simulate(cycles: u32) -> (World, Vec<Vec<NewEvent>>) {
    let overrides = vec![(
        "params.population.collapse_enabled".to_owned(),
        toml::Value::Boolean(false),
    )];
    let preset =
        load_preset_with_overrides(Path::new(WORKSPACE_PRESETS_DIR), "freeport", &overrides)
            .expect("preset loads");
    let society_id = u64::try_from(SOCIETY).unwrap();
    let mut world = World::new(society_id, 7, &preset);
    let mut batches = Vec::new();
    let created = Event::SocietyCreated {
        society_id,
        seed: 7,
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
    batches.push(batch);
    let ticks = cycles * world.ticks_per_cycle();
    for _ in 0..ticks {
        let mut batch = Vec::new();
        let now = world.meta.tick;
        let (round, _) = run_round(&mut world, &rules, now);
        for e in round {
            batch.push(NewEvent {
                meta: EventMeta::at(&world, None, Some(ClientKind::Householder)),
                event: e,
            });
        }
        let events = tick(&world, &rules, TickInput::next_for(&world)).expect("epoch not over");
        for e in &events {
            batch.push(NewEvent {
                meta: EventMeta::at(&world, None, None),
                event: e.clone(),
            });
        }
        for e in &events {
            apply(&mut world, e);
        }
        batches.push(batch);
    }
    (world, batches)
}

/// Append every batch in order; returns the last seq written.
async fn append_all(store: &PgEventStore, batches: &[Vec<NewEvent>]) -> i64 {
    let mut seq = 0i64;
    for b in batches {
        store.append_batch(SOCIETY, seq, b).await.expect("append");
        seq += i64::try_from(b.len()).unwrap();
    }
    seq - 1
}

#[sqlx::test]
async fn append_and_read_round_trip(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    store.create_society(&society_row()).await.unwrap();
    let (_, batches) = simulate(0);
    let last = append_all(&store, &batches).await;
    let read = store.read_from(SOCIETY, -1, 10_000).await.unwrap();
    let flat: Vec<&NewEvent> = batches.iter().flatten().collect();
    assert_eq!(read.len(), flat.len());
    assert_eq!(read.last().unwrap().seq, last);
    for (i, (r, n)) in read.iter().zip(&flat).enumerate() {
        assert_eq!(r.seq, i64::try_from(i).unwrap());
        assert_eq!(
            r.event, n.event,
            "event {i} differs after the JSON round trip"
        );
        assert_eq!(r.meta, n.meta);
    }
    // The first event carries who sent it; seeding events carry nothing.
    assert_eq!(read[0].meta.actor, Some(Actor::System));
    assert_eq!(read[0].meta.client_kind, Some(ClientKind::Sim));
    assert_eq!(read[1].meta.actor, None);
    // Paging: `after` is exclusive.
    let page = store.read_from(SOCIETY, 2, 3).await.unwrap();
    assert_eq!(
        page.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![3, 4, 5]
    );
    assert_eq!(store.list_societies().await.unwrap(), vec![society_row()]);
}

#[sqlx::test]
async fn snapshot_plus_tail_equals_full_fold(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    store.create_society(&society_row()).await.unwrap();
    let (live, batches) = simulate(3);
    // Persist the seeding batch and the first two cycles, snapshot, then the rest.
    let per_cycle = usize::try_from(live.ticks_per_cycle()).unwrap();
    let (head, tail) = batches.split_at(1 + 2 * per_cycle);
    let head_last = append_all(&store, head).await;
    let mut mid = World::new(u64::try_from(SOCIETY).unwrap(), 7, &head[0][0].preset());
    for e in head.iter().flatten() {
        apply(&mut mid, &e.event);
    }
    store
        .write_snapshot(SOCIETY, &mid, head_last)
        .await
        .unwrap();
    let mut seq = head_last + 1;
    for b in tail {
        store.append_batch(SOCIETY, seq, b).await.unwrap();
        seq += i64::try_from(b.len()).unwrap();
    }
    let total = i64::try_from(batches.iter().map(Vec::len).sum::<usize>()).unwrap();

    let loaded = load_world(&store, SOCIETY).await.unwrap();
    assert_eq!(loaded.last_seq, total - 1);
    assert_eq!(loaded.world.hash(), live.hash(), "snapshot + tail != live");
    assert_eq!(loaded.world, live);
    // The snapshot itself equals a replay of its own history.
    assert_eq!(
        verify_latest_snapshot(&store, SOCIETY).await.unwrap(),
        Some(head_last)
    );
    assert_eq!(
        replay_hash(&store, SOCIETY, total - 1).await.unwrap(),
        live.hash()
    );
    // A corrupted snapshot is refused rather than trusted.
    sqlx::query(
        "UPDATE snapshots SET world_hash = decode(repeat('00', 32), 'hex') WHERE society_id = $1",
    )
    .bind(SOCIETY)
    .execute(store.pool())
    .await
    .unwrap();
    assert!(matches!(
        load_world(&store, SOCIETY).await,
        Err(StoreError::SnapshotHashMismatch { .. })
    ));
}

/// A snapshot written before `World` changed shape: honest bytes (they match
/// their hash) that today's `World` cannot decode. The log is the truth, so the
/// load folds it from the start instead of refusing the society.
#[sqlx::test]
async fn stale_snapshot_falls_back_to_the_log(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    store.create_society(&society_row()).await.unwrap();
    let (live, batches) = simulate(1);
    let last = append_all(&store, &batches).await;
    store.write_snapshot(SOCIETY, &live, last).await.unwrap();
    // An older layout, as far as postcard can tell: the same bytes cut short.
    let bytes = live.canonical_bytes();
    let old = &bytes[..bytes.len() / 2];
    sqlx::query("UPDATE snapshots SET world_hash = $2, world = $3 WHERE society_id = $1")
        .bind(SOCIETY)
        .bind(blake3::hash(old).as_bytes().to_vec())
        .bind(zstd::encode_all(old, 3).unwrap())
        .execute(store.pool())
        .await
        .unwrap();

    let snap = store.latest_snapshot(SOCIETY).await.unwrap().unwrap();
    assert!(matches!(
        snap.world(SOCIETY),
        Err(StoreError::SnapshotStale { seq, .. }) if seq == last
    ));
    let loaded = load_world(&store, SOCIETY).await.unwrap();
    assert_eq!(loaded.last_seq, last);
    assert_eq!(loaded.world, live);
    // Nothing to verify: a stale snapshot is not a lying one.
    assert_eq!(verify_latest_snapshot(&store, SOCIETY).await.unwrap(), None);
}

#[sqlx::test]
async fn load_without_snapshot_folds_from_society_created(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    store.create_society(&society_row()).await.unwrap();
    let (live, batches) = simulate(1);
    let last = append_all(&store, &batches).await;
    let loaded = load_world(&store, SOCIETY).await.unwrap();
    assert_eq!(loaded.last_seq, last);
    assert_eq!(loaded.world.hash(), live.hash());
    assert!(matches!(
        load_world(&store, 99).await,
        Err(StoreError::NoEvents(99))
    ));
}

#[sqlx::test]
async fn seq_collision_fails_the_whole_transaction(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    store.create_society(&society_row()).await.unwrap();
    let (_, batches) = simulate(0);
    let last = append_all(&store, &batches).await;
    // A second writer that thinks the log ends one event earlier.
    let dup: Vec<NewEvent> = batches[0].iter().take(3).cloned().collect();
    let err = store.append_batch(SOCIETY, last, &dup).await.unwrap_err();
    assert!(matches!(err, StoreError::SeqCollision { society: SOCIETY, seq } if seq == last));
    // Nothing from the failed batch landed, not even the non-colliding rows.
    let read = store.read_from(SOCIETY, -1, 10_000).await.unwrap();
    assert_eq!(read.last().unwrap().seq, last);
    assert_eq!(read.len(), usize::try_from(last + 1).unwrap());
}

#[sqlx::test]
async fn app_role_cannot_update_or_delete_events(pool: PgPool) {
    let store = PgEventStore::from_pool(pool.clone());
    store.create_society(&society_row()).await.unwrap();
    let (_, batches) = simulate(0);
    append_all(&store, &batches).await;
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("SET ROLE isms_app")
        .execute(&mut *conn)
        .await
        .unwrap();
    let update = sqlx::query("UPDATE events SET kind = 'x' WHERE society_id = $1 AND seq = 0")
        .bind(SOCIETY)
        .execute(&mut *conn)
        .await;
    assert!(
        update
            .unwrap_err()
            .to_string()
            .contains("permission denied")
    );
    let delete = sqlx::query("DELETE FROM events WHERE society_id = $1")
        .bind(SOCIETY)
        .execute(&mut *conn)
        .await;
    assert!(
        delete
            .unwrap_err()
            .to_string()
            .contains("permission denied")
    );
    // The role can still read and append.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM events WHERE society_id = $1")
        .bind(SOCIETY)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert!(count > 0);
    sqlx::query("RESET ROLE").execute(&mut *conn).await.unwrap();
    // Even the owner is stopped by the trigger.
    let owner_update =
        sqlx::query("UPDATE events SET kind = 'x' WHERE society_id = $1 AND seq = 0")
            .bind(SOCIETY)
            .execute(&mut *conn)
            .await;
    assert!(
        owner_update
            .unwrap_err()
            .to_string()
            .contains("append-only")
    );
    let owner_delete = sqlx::query("DELETE FROM events WHERE society_id = $1")
        .bind(SOCIETY)
        .execute(&mut *conn)
        .await;
    assert!(
        owner_delete
            .unwrap_err()
            .to_string()
            .contains("append-only")
    );
}

/// Test helper: the preset carried by a `SocietyCreated` event.
trait CreatedPreset {
    fn preset(&self) -> isms_core::Preset;
}
impl CreatedPreset for NewEvent {
    fn preset(&self) -> isms_core::Preset {
        match &self.event {
            Event::SocietyCreated { preset, .. } => (**preset).clone(),
            _ => panic!("first event is not SocietyCreated"),
        }
    }
}

/// Ticks restart at 0 with every epoch, so a since-tick read names its epoch:
/// without it the second epoch's price history was the first epoch's opening days.
#[sqlx::test]
async fn since_tick_reads_stay_inside_their_epoch(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    store.create_society(&society_row()).await.unwrap();
    let (_, batches) = simulate(2);
    let last = append_all(&store, &batches).await;
    // The same ticks again as a second epoch (the payloads do not matter to the read).
    let again: Vec<NewEvent> = batches[1..]
        .iter()
        .flatten()
        .map(|e| NewEvent {
            meta: EventMeta {
                epoch: 1,
                ..e.meta.clone()
            },
            event: e.event.clone(),
        })
        .collect();
    store.append_batch(SOCIETY, last + 1, &again).await.unwrap();

    for epoch in [0, 1] {
        let ticks = store
            .read_kind_since_tick(SOCIETY, epoch, "TickResolved", 24, 1_000)
            .await
            .unwrap();
        assert_eq!(
            ticks.len(),
            24,
            "one day of ticks from tick 24 on, epoch {epoch}"
        );
        assert!(
            ticks
                .iter()
                .all(|e| e.meta.epoch == epoch && e.meta.tick >= 24)
        );

        let all = store
            .read_since_tick(SOCIETY, epoch, 40, 100_000)
            .await
            .unwrap();
        assert!(!all.is_empty());
        assert!(
            all.iter()
                .all(|e| e.meta.epoch == epoch && e.meta.tick >= 40)
        );
    }
}

/// ADR-0009: `class` is one of three words; the database refuses a fourth.
#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn society_class_is_checked(pool: PgPool) {
    let store = PgEventStore::from_pool(pool);
    let mut row = society_row();
    row.class = "lab".into();
    store.create_society(&row).await.unwrap();
    let mut bad = society_row();
    bad.id = SOCIETY + 1;
    bad.name = "freeport-bad".into();
    bad.class = "sandbox".into();
    assert!(store.create_society(&bad).await.is_err());
}
