#![allow(clippy::many_single_char_names)]
//! S0.12b done gate: a 40-householder Freeport runs 3 cycles with zero rejected
//! householder commands; every householder is employed and housed by cycle 2;
//! legacy firms have positive treasuries at cycle 3; a human buying a legacy
//! firm becomes controlling owner and the householder manager steps down; the
//! 3-cycle event log is byte-stable.

use isms_core::command::{Command, Envelope};
use isms_core::employment::employed;
use isms_core::event::Event;
use isms_core::householder::{ask_price, cost_plus, offer_wage, run_round};
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

/// E-7: the wage offer is the legacy wage moved a step per step the board
/// has taken, clamped to the band; a fresh workplace offers the legacy wage.
#[test]
fn a_board_moves_the_wage_offer_by_a_step_within_the_band() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mill = legacy_org(&h, WorkplaceKind::Mill);
    let wp = *h.world.orgs[&mill].workplaces.iter().next().unwrap();
    assert_eq!(offer_wage(&h.world, wp), Money::credits(8));
    // 8.00 x 1.05 = 8.40 a step; x2.00 = 16.00 is the ceiling (twenty steps);
    // the floor is the legacy wage itself.
    for (step, cents) in [(1, 840), (4, 960), (20, 1600), (25, 1600), (-3, 800)] {
        h.apply(Event::WageStepped {
            workplace: wp,
            cycle: 0,
            step,
        });
        assert_eq!(offer_wage(&h.world, wp), Money::cents(cents), "step {step}");
    }
    // the ask's cost basis has not moved with the board
    assert_eq!(cost_plus(&h.world, WorkplaceKind::Mill), Money::cents(131));
    h.check();
}

/// E-7: twelve householders spread one per workplace and nobody chooses the
/// mills; the mills' offers stand unfilled, so their wage steps up cycle by
/// cycle and the offer follows, while a firm whose shelf grows steps back
/// down and a firm that can no longer pay for a place withdraws its offer.
/// When the first hands are freed they take the best wage there is, and the
/// town has Food again.
#[test]
#[allow(clippy::too_many_lines)]
fn a_mill_nobody_chose_raises_its_wage_until_the_idle_hands_come() {
    // (`seed_epoch` restores the preset's floor, so the floor is set after it.)
    let mut h = WorldBuilder::new("freeport")
        .seed(1)
        .seed_epoch()
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.floor = 12;
        })
        .build();
    h.check_every_step = false;
    let open_offers = |h: &Harness| -> Vec<(isms_core::ids::WorkplaceId, Money)> {
        h.world
            .offers
            .values()
            .filter_map(|o| match o.body {
                OfferBody::Employment {
                    workplace,
                    pay: isms_core::world::Pay::Hourly(w),
                    places,
                    ..
                } if places > 0 => Some((workplace, w)),
                _ => None,
            })
            .collect()
    };
    let mut rejected = Vec::new();
    for _ in 0..24 {
        rejected.extend(step(&mut h).1);
    }
    assert!(rejected.is_empty(), "{rejected:#?}");
    // Twelve hands for eighteen workplaces: every board closed short, and a
    // first close reads no shelf.
    let idle: Vec<_> = h
        .world
        .workplaces
        .values()
        .filter(|w| w.workers.is_empty())
        .map(|w| w.id)
        .collect();
    assert!(idle.len() >= 6, "{} idle workplaces", idle.len());
    for w in h.world.workplaces.values() {
        assert_eq!(w.wage_step, 1, "{:?} closed short once", w.id);
    }
    // The next hour every manager withdraws its 8.00 offer and posts at 8.40.
    let (events, r) = step(&mut h);
    assert!(r.is_empty(), "{r:#?}");
    let withdrawn = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::OfferWithdrawn {
                    body: OfferBody::Employment { .. },
                    ..
                }
            )
        })
        .count();
    assert!(withdrawn >= idle.len(), "{withdrawn} offers withdrawn");
    for (wp, w) in open_offers(&h) {
        assert_eq!(w, Money::cents(840), "{wp:?}'s offer follows the board");
    }
    for _ in 0..23 {
        rejected.extend(step(&mut h).1);
    }
    assert!(rejected.is_empty(), "{rejected:#?}");
    // The idle boards closed short again, unless the org's shelf grew (a
    // seeded shelf coming back off the book counts); a staffed firm whose
    // shelf grew (more made than the town bought) stepped back down instead.
    for wp in &idle {
        let w = &h.world.workplaces[wp];
        let shelf_fell = h.world.params.recipes[&w.kind]
            .produces
            .as_good()
            .and_then(|g| h.world.orgs[&w.org].shelf.get(&g))
            .is_some_and(|s| s.step < 0);
        assert!(
            w.wage_step == 2 || (shelf_fell && w.wage_step == 0),
            "{wp:?} closed short twice: step {}",
            w.wage_step
        );
    }
    assert!(
        h.world.workplaces.values().any(|w| w.wage_step < 2),
        "some board answered its shelf: {:?}",
        h.world
            .workplaces
            .values()
            .map(|w| (w.kind, w.workers.len(), w.wage_step))
            .collect::<Vec<_>>()
    );
    // Eight more cycles: the workshops break, the hands they free take the
    // best wage, and the mills make Food.
    let mut food_made = 0;
    for _ in 0..(8 * 24) {
        let (events, r) = step(&mut h);
        rejected.extend(r);
        food_made += events
            .iter()
            .filter_map(|e| match e {
                Event::Produced { output, units, .. } if *output == Good::Food => Some(*units),
                _ => None,
            })
            .sum::<u32>();
    }
    assert!(rejected.is_empty(), "{rejected:#?}");
    assert!(food_made > 0, "the mills made Food once they had hands");
    // One more round, so the boards the last close moved have had their hour.
    let rules = h.rules();
    let tick = h.world.meta.tick;
    let mut scratch = h.world.clone();
    let (events, r) = run_round(&mut scratch, &rules, tick);
    assert!(r.is_empty(), "{r:#?}");
    h.apply_all(events);
    assert!(
        h.world
            .workplaces
            .values()
            .any(|w| w.kind == WorkplaceKind::Mill && !w.workers.is_empty()),
        "a mill has hands"
    );
    // No firm keeps an offer open that it cannot pay for.
    for o in h.world.offers.values() {
        if let OfferBody::Employment {
            org,
            workplace,
            pay: isms_core::world::Pay::Hourly(w),
            places,
            ..
        } = o.body
            && places > 0
        {
            let a_place = Money(w.0 * i64::from(h.world.params.labor.base_budget_hours));
            assert!(
                h.world.orgs[&org].treasury >= a_place,
                "{} keeps an offer at {w} open at {workplace} with {} in the treasury",
                h.world.orgs[&org].name,
                h.world.orgs[&org].treasury
            );
            assert_eq!(
                w,
                offer_wage(&h.world, workplace),
                "the offer is at the board's wage"
            );
        }
    }
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
