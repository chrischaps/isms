#![allow(clippy::many_single_char_names)]
//! S0.12b done gate: a 40-householder Freeport runs 3 cycles with zero rejected
//! householder commands; every householder is employed and housed by cycle 2;
//! legacy firms have positive treasuries at cycle 3; a human buying a legacy
//! firm becomes controlling owner and the householder manager steps down; the
//! 3-cycle event log is byte-stable.

use isms_core::command::{Command, Envelope};
use isms_core::employment::employed;
use isms_core::event::Event;
use isms_core::householder::{ask_price, cost_plus, run_round};
use isms_core::kinds::{CitizenKind, Good, WorkplaceKind};
use isms_core::ledger::Party;
use isms_core::money::Money;
use isms_core::test_support::{Harness, WorldBuilder, check_golden};
use isms_core::world::{Instrument, OfferBody, SaleAsset, Shelf, Side};

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

/// The legacy org that runs a workplace of `kind`.
fn legacy_org(h: &Harness, kind: WorkplaceKind) -> isms_core::ids::OrgId {
    h.world
        .workplaces
        .values()
        .find(|w| w.kind == kind)
        .map(|w| w.org)
        .expect("a seeded workplace of that kind")
}

/// E-1: the ask is cost-plus at the shelf's markup, one step per step the
/// shelf has taken, clamped to the band; a good never closed sits at the
/// legacy markup.
#[test]
fn a_shelf_moves_the_ask_by_a_step_within_the_band() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mill = legacy_org(&h, WorkplaceKind::Mill);
    assert_eq!(
        ask_price(&h.world, mill, WorkplaceKind::Mill),
        Money::cents(131)
    );
    // 114.33 cents of cost: x1.10 = 126, x1.00 = 114 (the floor, markup 0),
    // x1.45 = 166 (the ceiling, markup 0.45)
    for (step, cents) in [(-1, 126), (-3, 114), (-10, 114), (6, 166), (20, 166)] {
        h.apply(Event::ShelfClosed {
            org: mill,
            cycle: 0,
            shelf: [(
                Good::Food,
                Shelf {
                    last_close: 0,
                    step,
                },
            )]
            .into(),
        });
        assert_eq!(
            ask_price(&h.world, mill, WorkplaceKind::Mill),
            Money::cents(cents),
            "step {step}"
        );
    }
    // the base price the tests above anchor on has not moved
    assert_eq!(cost_plus(&h.world, WorkplaceKind::Mill), Money::cents(131));
    h.check();
}

/// E-1: the first close only takes the reading; a shelf that closes fuller
/// than the close before steps down and the next hour's ask follows; an org
/// that produced nothing holds even with an empty shelf.
#[test]
fn a_shelf_that_closed_fuller_steps_down_and_the_ask_follows() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mill = legacy_org(&h, WorkplaceKind::Mill);
    let mut events = Vec::new();
    for _ in 0..24 {
        events.extend(step(&mut h).0);
    }
    let closes: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::ShelfClosed { .. }))
        .collect();
    assert!(!closes.is_empty(), "every market org closes its shelf once");
    for o in h.world.orgs.values() {
        for (g, s) in &o.shelf {
            assert_eq!(s.step, 0, "{}'s {g:?}: a first close takes no step", o.name);
            assert_eq!(s.last_close, o.inventory.get(g).copied().unwrap_or(0));
        }
    }
    // Pretend the mill's shelf was bare at the last close: whatever Food it
    // holds at the next close is growth.
    h.apply(Event::ShelfClosed {
        org: mill,
        cycle: 0,
        shelf: [(
            Good::Food,
            Shelf {
                last_close: 0,
                step: 0,
            },
        )]
        .into(),
    });
    for _ in 0..24 {
        step(&mut h);
    }
    let held = h.world.orgs[&mill]
        .inventory
        .get(&Good::Food)
        .copied()
        .unwrap_or(0);
    assert!(held > 0, "the mill holds Food at the close");
    assert_eq!(h.world.orgs[&mill].shelf[&Good::Food].step, -1);
    assert_eq!(
        ask_price(&h.world, mill, WorkplaceKind::Mill),
        Money::cents(126)
    );
    // The next hour the manager re-posts at the new price, and the plans'
    // bids resting at the old one cross it: the cut sells (at the resting
    // bid's price, ADR-0001).
    let (events, _) = step(&mut h);
    let asks: Vec<Money> = events
        .iter()
        .filter_map(|e| match e {
            Event::OrderPlaced { order, .. }
                if order.owner == Party::Org(mill)
                    && order.side == Side::Ask
                    && order.instrument == Instrument::Good(Good::Food) =>
            {
                Some(order.limit_price)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        asks,
        vec![Money::cents(126)],
        "the mill's ask at the new price"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Trade { seller, instrument, .. }
            if *seller == Party::Org(mill) && *instrument == Instrument::Good(Good::Food))),
        "the cheaper Food sold"
    );
    h.check();
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
