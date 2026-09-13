//! S0.4 done gate: determinism, seed-dependent shuffle, cycle boundaries, the
//! scheduled epoch end, and collapse.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::kinds::{CitizenKind, ClientKind};
use isms_core::rules::Rules;
use isms_core::test_support::{WorldBuilder, assert_deterministic};
use isms_core::tick::{TickError, TickInput, derive_seed, tick};
use isms_core::world::EpochEndReason;

fn kinds(events: &[Event]) -> Vec<&'static str> {
    events.iter().map(Event::kind).collect()
}

#[test]
fn a_hundred_ticks_are_deterministic() {
    assert_deterministic(11, |seed| {
        let mut h = WorldBuilder::new("freeport")
            .seed(seed)
            .humans(5)
            .householders(3)
            .build();
        let mut all = h.log.clone();
        for _ in 0..100 {
            all.extend(h.tick());
        }
        all
    });
}

#[test]
fn the_seed_changes_the_shuffle_order() {
    let mut orders = Vec::new();
    for seed in [1u64, 2, 3] {
        let h = WorldBuilder::new("freeport").seed(seed).humans(12).build();
        let rules = Rules::from_world(&h.world);
        let input = TickInput::next_for(&h.world);
        let order = isms_core::tick::shuffle_order_for_test(&h.world, &rules, input);
        orders.push(order);
    }
    assert_ne!(orders[0], orders[1]);
    assert_ne!(orders[1], orders[2]);
    // and the same seed gives the same order
    let h = WorldBuilder::new("freeport").seed(1).humans(12).build();
    let rules = Rules::from_world(&h.world);
    let again =
        isms_core::tick::shuffle_order_for_test(&h.world, &rules, TickInput::next_for(&h.world));
    assert_eq!(orders[0], again);
}

#[test]
fn seeds_differ_by_tick_and_epoch() {
    assert_ne!(derive_seed(1, 0, 0), derive_seed(1, 0, 1));
    assert_ne!(derive_seed(1, 0, 0), derive_seed(1, 1, 0));
    assert_ne!(derive_seed(1, 0, 0), derive_seed(2, 0, 0));
    assert_eq!(derive_seed(9, 3, 7), derive_seed(9, 3, 7));
}

#[test]
fn every_tick_ends_with_tick_resolved_and_cycles_close_at_23_47() {
    let mut h = WorldBuilder::new("freeport").humans(2).build();
    for t in 0..48u32 {
        let events = h.tick();
        assert_eq!(
            events.last().map(Event::kind),
            Some("TickResolved"),
            "tick {t}"
        );
        let closed = events
            .iter()
            .any(|e| matches!(e, Event::CycleClosed { .. }));
        assert_eq!(closed, t % 24 == 23, "tick {t}");
        if let Some(Event::CycleClosed { cycle, .. }) = events
            .iter()
            .find(|e| matches!(e, Event::CycleClosed { .. }))
        {
            assert_eq!(*cycle, t / 24);
        }
    }
    assert_eq!(h.world.meta.tick, 48);
}

#[test]
fn tick_input_must_match_the_world() {
    let h = WorldBuilder::new("freeport").humans(1).build();
    let rules = Rules::from_world(&h.world);
    let wrong = TickInput {
        tick: 5,
        seed: [0; 32],
    };
    assert_eq!(
        tick(&h.world, &rules, wrong),
        Err(TickError::WrongTick {
            expected: 0,
            got: 5
        })
    );
}

#[test]
fn scheduled_epoch_end_at_tick_1007_and_no_tick_1008() {
    // One human is below the floor; the simulator turns collapse off (TDD 5.5).
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .build();
    h.check_every_step = false;
    for _ in 0..1007 {
        let events = h.tick();
        assert!(!kinds(&events).contains(&"EpochEnded"));
    }
    let last = h.tick();
    let k = kinds(&last);
    assert!(k.contains(&"CycleClosed"));
    assert!(k.contains(&"EpochEnded"));
    assert_eq!(k.last(), Some(&"TickResolved"));
    assert!(matches!(
        last.iter().find(|e| matches!(e, Event::EpochEnded { .. })),
        Some(Event::EpochEnded {
            reason: EpochEndReason::Scheduled,
            cycle: 41
        })
    ));
    assert_eq!(h.world.meta.epoch_ended, Some(EpochEndReason::Scheduled));
    assert_eq!(h.try_tick(), Err(TickError::EpochEnded));
    h.check();
    // commands are refused too
    let r = h.cmd(Envelope::citizen(h.citizen_ids()[0], Command::Seen, 1008));
    assert_eq!(r.unwrap_err().code, RejectCode::EpochEnded);
}

#[test]
fn collapse_fires_after_five_low_cycles_only_when_enabled() {
    // Householders only: zero active humans, below the floor of 40.
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = true;
            p.params.population.floor = 40;
        })
        .householders(3)
        .build();
    h.check_every_step = false;
    let mut ended_at = None;
    for t in 0..(24 * 6) {
        let events = h.tick();
        if let Some(Event::EpochEnded { reason, cycle }) = events
            .iter()
            .find(|e| matches!(e, Event::EpochEnded { .. }))
        {
            ended_at = Some((t, *reason, *cycle));
            break;
        }
    }
    assert_eq!(ended_at, Some((24 * 5 - 1, EpochEndReason::Collapse, 4)));
    h.check();

    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(3)
        .build();
    h.check_every_step = false;
    for _ in 0..(24 * 8) {
        let events = h.tick();
        assert!(!kinds(&events).contains(&"EpochEnded"));
    }
    h.check();
}

#[test]
fn handle_gates_on_capabilities_before_anything_else() {
    let h = WorldBuilder::new("commune").humans(1).build();
    let me = h.citizen_ids()[0];
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::PlaceOrder {
            instrument: isms_core::world::Instrument::Good(isms_core::kinds::Good::Food),
            side: isms_core::world::Side::Bid,
            qty: 1,
            limit_price: isms_core::money::Money::cents(1),
            expires_tick: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotInThisSociety);
    // a goods transfer is a primitive everywhere; it passes the gate and fails on its merits
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::Transfer {
            to: isms_core::ledger::Party::Citizen(me),
            asset: isms_core::ledger::Asset::Good(isms_core::kinds::Good::Food, 1),
            memo: String::new(),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::SelfDeal);
}

#[test]
fn join_and_seen_are_real_commands() {
    let mut h = WorldBuilder::new("freeport").humans(1).build();
    let events = h
        .cmd(Envelope::system(
            Command::Join {
                handle: "ada".into(),
                kind: CitizenKind::Human,
            },
            0,
        ))
        .unwrap();
    assert_eq!(kinds(&events), ["CitizenJoined"]);
    let ada = h.citizen_ids()[1];
    assert_eq!(h.citizen(ada).handle, "ada");
    assert_eq!(
        h.citizen(ada).household.balance,
        isms_core::money::Money::credits(1000)
    );
    let dup = h.cmd_dry(Envelope::system(
        Command::Join {
            handle: "ada".into(),
            kind: CitizenKind::Human,
        },
        0,
    ));
    assert_eq!(dup.unwrap_err().code, RejectCode::HandleTaken);

    h.tick();
    let seen = h
        .cmd(Envelope::citizen(ada, Command::Seen, 1).via(ClientKind::Web))
        .unwrap();
    assert_eq!(kinds(&seen), ["CitizenSeen"]);
    assert_eq!(h.citizen(ada).last_seen_tick, 1);
    // throttled: a second Seen in the same tick emits nothing
    let again = h.cmd(Envelope::citizen(ada, Command::Seen, 1)).unwrap();
    assert!(again.is_empty());
}
