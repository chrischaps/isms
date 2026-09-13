#![allow(clippy::many_single_char_names)]
//! S0.12b done gate: a 40-householder Freeport runs 3 cycles with zero rejected
//! householder commands; every householder is employed and housed by cycle 2;
//! legacy firms have positive treasuries at cycle 3; a human buying a legacy
//! firm becomes controlling owner and the householder manager steps down; the
//! 3-cycle event log is byte-stable.

use isms_core::command::{Command, Envelope};
use isms_core::employment::employed;
use isms_core::event::Event;
use isms_core::householder::{cost_plus, run_round};
use isms_core::kinds::{CitizenKind, Good, WorkplaceKind};
use isms_core::ledger::Party;
use isms_core::money::Money;
use isms_core::test_support::{Harness, WorldBuilder, check_golden};
use isms_core::world::{OfferBody, SaleAsset};

fn seeded(seed: u64) -> Harness {
    WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed(seed)
        .seed_epoch()
        .build()
}

/// One round of scripts, then one tick; returns all events and the rejection count.
fn step(h: &mut Harness) -> (Vec<Event>, Vec<isms_core::householder::Rejection>) {
    let rules = h.rules();
    let tick = h.world.meta.tick;
    let mut scratch = h.world.clone();
    let (events, rejected) = run_round(&mut scratch, &rules, tick);
    h.apply_all(events.iter().cloned());
    let mut all = events;
    all.extend(h.tick());
    (all, rejected)
}

#[test]
fn cost_plus_prices_start_at_the_anchors() {
    let h = seeded(1);
    // labor 8.00 / 15 = 0.533 + 0 inputs -> x1.15 = 0.61 vs start 0.60
    assert_eq!(cost_plus(&h.world, WorkplaceKind::Farm), Money::cents(61));
    // Mill: 53.33 + reference Grain 61 = 114.33 cents x 1.15 = 131.5 -> 1.31
    assert_eq!(cost_plus(&h.world, WorkplaceKind::Mill), Money::cents(131));
    assert!(cost_plus(&h.world, WorkplaceKind::MachineShop) >= Money::credits(9));
}

#[test]
fn forty_householders_run_three_cycles_without_a_rejection() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mut rejected = Vec::new();
    for _ in 0..72 {
        let (_, r) = step(&mut h);
        rejected.extend(r);
    }
    assert!(
        rejected.is_empty(),
        "every scripted command must be valid: {rejected:#?}"
    );
    // payday just fired anyone a broke firm could not pay; the next round rehires them
    let rules = h.rules();
    let tick = h.world.meta.tick;
    let mut scratch = h.world.clone();
    let (events, r) = run_round(&mut scratch, &rules, tick);
    assert!(r.is_empty(), "{r:#?}");
    h.apply_all(events);
    h.check();
    for c in h.world.citizens.values() {
        assert!(employed(&h.world, c.id), "{} is unemployed", c.handle);
        assert!(c.household.dwelling.is_some(), "{} is unhoused", c.handle);
    }
    for o in h.world.orgs.values() {
        assert!(
            o.treasury > Money::ZERO,
            "{} has treasury {}",
            o.name,
            o.treasury
        );
    }
    // the economy actually moved: food was produced and bought
    assert!(
        h.world
            .ledger_meta
            .produced
            .get(&Good::Food)
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(h.world.books.values().any(|b| b.last_trade_tick.is_some()));
}

#[test]
fn everyone_is_employed_and_housed_by_cycle_two() {
    let mut h = seeded(2);
    h.check_every_step = false;
    for _ in 0..24 {
        step(&mut h);
    }
    let unemployed = h
        .world
        .citizens
        .values()
        .filter(|c| !employed(&h.world, c.id))
        .count();
    let unhoused = h
        .world
        .citizens
        .values()
        .filter(|c| c.household.dwelling.is_none())
        .count();
    assert_eq!((unemployed, unhoused), (0, 0));
    h.check();
}

#[test]
fn a_human_buying_a_legacy_firm_takes_control_and_the_householder_steps_down() {
    let mut h = seeded(3);
    h.check_every_step = false;
    step(&mut h); // managers list their firms
    h.cmd(Envelope::system(
        Command::Join {
            handle: "marlow".into(),
            kind: CitizenKind::Human,
        },
        1,
    ))
    .unwrap();
    let marlow = *h.citizen_ids().last().unwrap();
    let (offer, org, price) = h
        .world
        .offers
        .values()
        .find_map(|o| match o.body {
            OfferBody::Sale {
                asset: SaleAsset::Shares(org, 100),
                price: isms_core::world::Price::Money(m),
                ..
            } => Some((o.id, org, m)),
            _ => None,
        })
        .expect("a legacy firm is for sale at book value");
    assert!(
        price <= Money::credits(1000),
        "book value {price} is within the endowment"
    );
    let before = h.world.orgs[&org].manager.unwrap();
    assert_eq!(h.citizen(before).kind, CitizenKind::Householder);
    let events = h
        .cmd(Envelope::citizen(marlow, Command::AcceptSale { offer }, 1))
        .unwrap();
    assert!(
        events.iter().any(
            |e| matches!(e, Event::ManagerAppointed { citizen, .. } if *citizen == Some(marlow))
        )
    );
    assert_eq!(
        isms_core::orgs::controlling_owner(&h.world.orgs[&org]),
        Some(marlow)
    );
    assert_eq!(h.world.orgs[&org].manager, Some(marlow));
    // the householder scripts keep running for the other firms
    let (_, rejected) = step(&mut h);
    assert!(rejected.is_empty(), "{rejected:#?}");
    assert!(
        h.world.orgs[&org].manager == Some(marlow),
        "the script does not touch a human-run firm"
    );
    h.check();
    let _ = Party::Citizen(marlow);
}

#[test]
fn golden_three_cycle_freeport() {
    let mut h = seeded(7);
    h.check_every_step = false;
    let mut log = h.log.clone();
    for _ in 0..72 {
        let (events, rejected) = step(&mut h);
        assert!(rejected.is_empty(), "{rejected:#?}");
        log.extend(events);
    }
    h.check();
    check_golden("s0_12_freeport_3_cycles", &log);
}
