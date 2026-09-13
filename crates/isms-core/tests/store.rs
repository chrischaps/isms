#![allow(clippy::many_single_char_names)]
//! S0.15a done gate (Common Store): conservation holds with the store as a
//! holder; need-first serves the largest shortfall first (table test) and ties
//! deterministically; the draw entitlement is the meter shortfall; seeded
//! stock lands in the store; dwellings are assigned from the collective stock
//! at join and at cycle end; an emigrant's pantry returns to the store.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::{CitizenId, DwellingId};
use isms_core::kinds::{CitizenKind, Good, OrgKind};
use isms_core::ledger::{Asset, Holder};
use isms_core::money::Money;
use isms_core::needs::{FULL, TENTHS};
use isms_core::test_support::{Harness, WorldBuilder, assert_deterministic};
use isms_core::world::{Needs, Owner};
use std::collections::BTreeMap;

fn commune(n: u32) -> Harness {
    WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(n)
        .build()
}

fn seeded_commune() -> Harness {
    WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build()
}

fn food_meter(h: &mut Harness, id: CitizenId, tenths: u16) {
    let mut needs = h.citizen(id).needs.clone();
    needs.food = tenths;
    h.set_needs(id, needs);
}

fn store_food(h: &Harness) -> u32 {
    h.world
        .store
        .as_ref()
        .and_then(|s| s.stock.get(&Good::Food).copied())
        .unwrap_or(0)
}

fn drew(events: &[Event]) -> BTreeMap<CitizenId, u32> {
    let mut out = BTreeMap::new();
    for e in events {
        if let Event::Drew { citizen, goods, .. } = e {
            *out.entry(*citizen).or_insert(0) += goods.get(&Good::Food).copied().unwrap_or(0);
        }
    }
    out
}

#[test]
fn seeded_food_lands_in_the_store_not_the_org() {
    let h = seeded_commune();
    assert!(
        h.world
            .orgs
            .values()
            .all(|o| o.kind == OrgKind::Collective && o.inventory.is_empty())
    );
    assert_eq!(store_food(&h), 3 * 320, "three Mills' seeded Food");
    assert!(
        h.world
            .dwellings
            .values()
            .all(|d| d.owner == Owner::Society)
    );
    assert_eq!(h.world.dwellings.len(), 40);
    assert!(
        h.world
            .citizens
            .values()
            .all(|c| c.household.dwelling.is_some()),
        "every householder is housed from the collective stock at join"
    );
    assert!(h.world.dwellings.values().all(|d| d.occupant.is_some()));
}

#[test]
fn draw_entitlement_is_the_meter_shortfall() {
    let mut h = commune(1);
    let a = CitizenId(0);
    let per = u16::from(h.world.params.needs.food_meter_per_unit) * TENTHS;
    // 30 points short at 6 per unit = 5 units.
    food_meter(&mut h, a, FULL - 30 * TENTHS);
    assert_eq!(
        isms_core::store::entitlement(&h.world, h.citizen(a), Good::Food),
        u32::from((30 * TENTHS).div_ceil(per))
    );
    // Two on hand count against it.
    h.apply(Event::Seeded {
        holder: Holder::Citizen(a),
        asset: Asset::Good(Good::Food, 2),
    });
    assert_eq!(
        isms_core::store::entitlement(&h.world, h.citizen(a), Good::Food),
        3
    );
    // A full meter has no entitlement, and a draw above it is rejected.
    food_meter(&mut h, a, FULL);
    assert_eq!(
        isms_core::store::entitlement(&h.world, h.citizen(a), Good::Food),
        0
    );
    let tick = h.world.meta.tick;
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::RequestStoreDraw {
                good: Good::Food,
                qty: 1,
            },
            tick,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::OverEntitlement);
    // Materials have no need entitlement.
    assert_eq!(
        isms_core::store::entitlement(&h.world, h.citizen(a), Good::Materials),
        0
    );
    // And the command does not exist in a market society.
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(1)
        .build();
    let err = f
        .cmd(Envelope::citizen(
            a,
            Command::RequestStoreDraw {
                good: Good::Food,
                qty: 1,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
}

#[test]
fn need_first_serves_the_largest_shortfall_first() {
    let mut h = commune(3);
    let (a, b, c) = (CitizenId(0), CitizenId(1), CitizenId(2));
    // Shortfalls of 5, 3 and 2 units (6 points each) against a stock of 6.
    food_meter(&mut h, a, FULL - 30 * TENTHS);
    food_meter(&mut h, b, FULL - 18 * TENTHS);
    food_meter(&mut h, c, FULL - 12 * TENTHS);
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Food, 6),
    });
    let events = h.tick();
    let got = drew(&events);
    assert_eq!(
        got.get(&a),
        Some(&5),
        "the largest shortfall is served in full"
    );
    assert_eq!(got.get(&b), Some(&1), "the next gets what is left");
    assert_eq!(got.get(&c), None, "the smallest gets nothing");
    assert_eq!(store_food(&h), 0);
    assert!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Drew { .. }))
            .all(|e| matches!(e, Event::Drew { explain, .. }
                if explain.rule == isms_core::explain::RuleId::StoreDrawNeedFirst))
    );
    // Requests are cleared by the tick.
    assert!(h.world.store.as_ref().unwrap().requests.is_empty());
}

#[test]
fn need_first_ties_are_broken_by_the_seed_and_are_deterministic() {
    let run = |seed: u64| {
        let mut h = WorldBuilder::new("commune")
            .with_preset(|p| p.params.population.collapse_enabled = false)
            .seed(seed)
            .householders(2)
            .build();
        food_meter(&mut h, CitizenId(0), FULL - 18 * TENTHS);
        food_meter(&mut h, CitizenId(1), FULL - 18 * TENTHS);
        h.apply(Event::Seeded {
            holder: Holder::Store,
            asset: Asset::Good(Good::Food, 4),
        });
        h.tick()
    };
    assert_deterministic(7, run);
    let got = drew(&run(7));
    let mut served: Vec<u32> = got.values().copied().collect();
    served.sort_unstable();
    assert_eq!(
        served,
        vec![1, 3],
        "one full request, the other the remainder"
    );
}

#[test]
fn conservation_holds_with_the_store_as_a_holder() {
    // Forty householders draw the seeded Food down to a shortage over two
    // cycles; the harness re-checks conservation after every tick.
    let mut h = seeded_commune();
    let before = store_food(&h);
    h.run_cycles(2);
    assert!(store_food(&h) < before);
    assert!(
        h.log.iter().any(|e| matches!(e, Event::Drew { .. })),
        "householders drew from the store"
    );
    isms_core::ledger::conservation_check(&h.world).unwrap();
}

#[test]
fn dwellings_are_assigned_from_collective_stock_at_join_and_at_cycle_end() {
    // With a spare dwelling, a joiner is housed at once.
    let mut h = WorldBuilder::new("commune")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.initial_dwellings = 41;
        })
        .seed_epoch()
        .build();
    let tick = h.world.meta.tick;
    let events = h
        .cmd(Envelope::system(
            Command::Join {
                handle: "ada".into(),
                kind: CitizenKind::Human,
            },
            tick,
        ))
        .unwrap();
    let Event::CitizenJoined {
        citizen, dwelling, ..
    } = &events[0]
    else {
        panic!("expected CitizenJoined");
    };
    assert_eq!(*dwelling, Some(DwellingId(40)));
    assert_eq!(h.citizen(*citizen).household.dwelling, Some(DwellingId(40)));
    assert_eq!(h.world.dwellings[&DwellingId(40)].occupant, Some(*citizen));

    // With none spare, the joiner waits: the cycle-end fill emigrates one
    // householder (whose dwelling returns to the stock) and the next cycle end
    // houses the newcomer at step 8b.
    let mut h = seeded_commune();
    let tick = h.world.meta.tick;
    let events = h
        .cmd(Envelope::system(
            Command::Join {
                handle: "bo".into(),
                kind: CitizenKind::Human,
            },
            tick,
        ))
        .unwrap();
    let Event::CitizenJoined {
        citizen, dwelling, ..
    } = &events[0]
    else {
        panic!("expected CitizenJoined");
    };
    let human = *citizen;
    assert_eq!(*dwelling, None);
    h.run_cycle();
    assert!(
        h.log
            .iter()
            .any(|e| matches!(e, Event::HouseholderEmigrated { .. }))
    );
    assert_eq!(
        h.world
            .dwellings
            .values()
            .filter(|d| d.occupant.is_none())
            .count(),
        0,
        "the freed dwelling was assigned to the human at the same cycle end"
    );
    assert!(h.citizen(human).household.dwelling.is_some());
}

#[test]
fn emigration_returns_goods_to_the_store() {
    let mut h = seeded_commune();
    let last = CitizenId(39);
    h.apply(Event::Seeded {
        holder: Holder::Citizen(last),
        asset: Asset::Good(Good::Materials, 7),
    });
    let tick = h.world.meta.tick;
    h.cmd(Envelope::system(
        Command::Join {
            handle: "cy".into(),
            kind: CitizenKind::Human,
        },
        tick,
    ))
    .unwrap();
    let materials_before = h
        .world
        .store
        .as_ref()
        .and_then(|s| s.stock.get(&Good::Materials).copied())
        .unwrap_or(0);
    let events = h.run_cycle();
    let returned = events.iter().find_map(|e| match e {
        Event::StoreReturned {
            citizen,
            holder,
            goods,
            money,
        } if *citizen == last => Some((*holder, goods.clone(), *money)),
        _ => None,
    });
    let (holder, goods, money) = returned.expect("the emigrant returned their pantry");
    assert_eq!(holder, Holder::Store);
    assert_eq!(goods.get(&Good::Materials), Some(&7));
    assert_eq!(money, Money::ZERO);
    let gone = events.iter().find_map(|e| match e {
        Event::HouseholderEmigrated {
            citizen,
            burned_money,
            burned_goods,
            ..
        } if *citizen == last => Some((*burned_money, burned_goods.clone())),
        _ => None,
    });
    assert_eq!(gone, Some((Money::ZERO, BTreeMap::new())));
    assert_eq!(
        h.world
            .store
            .as_ref()
            .and_then(|s| s.stock.get(&Good::Materials).copied())
            .unwrap_or(0),
        materials_before + 7
    );
    assert!(
        h.citizen(last).household.pantry.is_empty(),
        "{:?}",
        h.citizen(last).household.pantry
    );
    assert!(
        h.citizen(last).household.dwelling.is_none(),
        "the dwelling was released"
    );
    assert_eq!(h.world.ledger_meta.burned.get(&Good::Materials), None);
    let _ = Needs::at_start(&h.world.params);
}
