#![allow(clippy::many_single_char_names)]
//! S0.16a done gate (the state store): a stock-out produces a queue served in
//! arrival order; ration cards cap each citizen per cycle; sales move money to
//! the till and conserve; provision assigns a dwelling and issues the minimum
//! Food ration; a joiner is assigned the least-staffed workplace with room; a
//! good off the price list is rejected.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::ids::CitizenId;
use isms_core::kinds::{CitizenKind, Good, OrgKind};
use isms_core::labor::has_position;
use isms_core::ledger::{Asset, Holder};
use isms_core::money::Money;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::Owner;
use std::collections::BTreeMap;

fn directorate_humans(n: u32) -> Harness {
    WorldBuilder::new("directorate")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n)
        .build()
}

fn seeded_directorate() -> Harness {
    WorldBuilder::new("directorate")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build()
}

fn stock_food(h: &mut Harness, qty: u32) {
    h.apply(Event::Seeded {
        holder: Holder::StateStock,
        asset: Asset::Good(Good::Food, qty),
    });
}

fn quiet_plan(h: &mut Harness, id: CitizenId) {
    let mut plan = h.citizen(id).plan.clone();
    plan.keep_food_at_least = 0;
    plan.buy_wares_when = None;
    h.apply(Event::PlanChanged {
        citizen: id,
        plan: Box::new(plan),
    });
}

fn request(h: &mut Harness, id: CitizenId, good: Good, qty: u32) -> Result<(), RejectCode> {
    let tick = h.world.meta.tick;
    h.cmd(Envelope::citizen(
        id,
        Command::RequestStateStore { good, qty },
        tick,
    ))
    .map(|_| ())
    .map_err(|e| e.code)
}

fn sold(events: &[Event]) -> BTreeMap<CitizenId, u32> {
    let mut out = BTreeMap::new();
    for e in events {
        if let Event::StateStoreSold {
            citizen,
            good: Good::Food,
            qty,
            ..
        } = e
        {
            *out.entry(*citizen).or_insert(0) += qty;
        }
    }
    out
}

#[test]
fn stock_out_produces_a_queue_served_in_arrival_order() {
    let mut h = directorate_humans(3);
    let (a, b, c) = (CitizenId(0), CitizenId(1), CitizenId(2));
    for id in [a, b, c] {
        quiet_plan(&mut h, id);
    }
    stock_food(&mut h, 5);
    // Requests arrive b, a, c: b is served in full, a gets what is left, c nothing.
    request(&mut h, b, Good::Food, 3).unwrap();
    request(&mut h, a, Good::Food, 3).unwrap();
    request(&mut h, c, Good::Food, 3).unwrap();
    let events = h.tick();
    let got = sold(&events);
    assert_eq!(got.get(&b), Some(&3));
    assert_eq!(got.get(&a), Some(&2));
    assert_eq!(got.get(&c), None);
    let shortage = events.iter().find_map(|e| match e {
        Event::StateStoreShortage { unfilled, .. } => Some(unfilled.clone()),
        _ => None,
    });
    assert_eq!(shortage.unwrap().get(&Good::Food), Some(&4));
    let price = h.world.policy.price_list.as_ref().unwrap()[&Good::Food];
    assert_eq!(
        h.world.state_stock.as_ref().unwrap().till,
        Money(price.0 * 5),
        "the till took the price of every unit sold"
    );
    assert!(h.world.state_stock.as_ref().unwrap().requests.is_empty());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StateStoreSold { explain, .. }
                if explain.rule == RuleId::StateStorePrice))
    );
}

#[test]
fn ration_cards_cap_each_citizen_per_cycle() {
    let mut h = directorate_humans(1);
    let a = CitizenId(0);
    quiet_plan(&mut h, a);
    stock_food(&mut h, 100);
    let mut policy = h.world.policy.clone();
    policy.ration_caps = Some(BTreeMap::from([(Good::Food, 10)]));
    h.cmd(Envelope::system(
        Command::SetPolicy {
            policy: Box::new(policy),
        },
        0,
    ))
    .unwrap();
    request(&mut h, a, Good::Food, 12).unwrap();
    let got = sold(&h.tick());
    assert_eq!(got.get(&a), Some(&10), "the card caps the sale at 10");
    // Nothing more this cycle.
    request(&mut h, a, Good::Food, 1).unwrap();
    let events = h.tick();
    assert_eq!(sold(&events).get(&a), None);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StateStoreShortage { .. }))
    );
    // The card resets with the cycle.
    h.run_cycle();
    request(&mut h, a, Good::Food, 1).unwrap();
    assert_eq!(sold(&h.tick()).get(&a), Some(&1));
}

#[test]
fn sales_move_money_to_the_till_and_conserve() {
    let mut h = directorate_humans(2);
    let a = CitizenId(0);
    quiet_plan(&mut h, a);
    quiet_plan(&mut h, CitizenId(1));
    stock_food(&mut h, 20);
    let balance = h.citizen(a).household.balance;
    request(&mut h, a, Good::Food, 4).unwrap();
    h.tick();
    let price = h.world.policy.price_list.as_ref().unwrap()[&Good::Food];
    assert_eq!(h.citizen(a).household.balance, balance - Money(price.0 * 4));
    assert_eq!(
        h.world.state_stock.as_ref().unwrap().till,
        Money(price.0 * 4)
    );
    // Paid for out of a nearly empty purse: only what the balance covers.
    let broke = CitizenId(1);
    let have = h.citizen(broke).household.balance;
    h.apply(Event::Transferred {
        from: isms_core::ledger::Party::Citizen(broke),
        to: isms_core::ledger::Party::Citizen(a),
        asset: Asset::Money(have - Money(price.0 * 2)),
        memo: String::new(),
    });
    request(&mut h, broke, Good::Food, 5).unwrap();
    let got = sold(&h.tick());
    assert_eq!(got.get(&broke), Some(&2), "balance / price units");
    h.check();
}

#[test]
fn provision_assigns_a_dwelling_and_issues_the_minimum_food_ration() {
    let mut h = seeded_directorate();
    h.check_every_step = false;
    assert!(
        h.world
            .dwellings
            .values()
            .all(|d| d.owner == Owner::Society)
    );
    assert!(
        h.world
            .citizens
            .values()
            .all(|c| c.household.dwelling.is_some() && has_position(&h.world, c.id))
    );
    // The legacy budget went to the till, not to the enterprises.
    let n = u32::try_from(h.world.orgs.len()).unwrap();
    assert_eq!(
        h.world.state_stock.as_ref().unwrap().till,
        Money(h.world.params.money.legacy_treasury.0 * i64::from(n))
    );
    assert!(h.world.orgs.values().all(|o| o.treasury == Money::ZERO));
    // No scripts run here, so nothing is milled: stock the store for the cycle
    // and let the cycle-end ration top every pantry up to the minimum.
    stock_food(&mut h, 2000);
    let events = h.run_cycle();
    let ration = h.world.policy.minimum_food_ration.unwrap();
    let issued: Vec<(CitizenId, u32)> = events
        .iter()
        .filter_map(|e| match e {
            Event::RationIssued {
                citizen,
                good: Good::Food,
                qty,
                explain,
            } => {
                assert_eq!(explain.rule, RuleId::ProvisionRation);
                Some((*citizen, *qty))
            }
            _ => None,
        })
        .collect();
    assert!(!issued.is_empty(), "provision fired at cycle end");
    for (citizen, _) in &issued {
        assert_eq!(
            h.citizen(*citizen)
                .household
                .pantry
                .get(&Good::Food)
                .copied()
                .unwrap_or(0),
            ration,
            "topped up to the minimum ration"
        );
    }
    h.check();
}

#[test]
fn join_assigns_the_least_staffed_workplace_with_room() {
    let mut h = seeded_directorate();
    let expected = isms_core::orgs::least_staffed(&h.world).unwrap();
    let tick = h.world.meta.tick;
    let events = h
        .cmd(Envelope::system(
            Command::Join {
                handle: "dee".into(),
                kind: CitizenKind::Human,
            },
            tick,
        ))
        .unwrap();
    let assigned = events.iter().find_map(|e| match e {
        Event::Assigned {
            workplace,
            citizen,
            contract: None,
        } => Some((*workplace, *citizen)),
        _ => None,
    });
    let (workplace, citizen) = assigned.expect("a joiner is assigned at once");
    assert_eq!(workplace, expected);
    assert!(has_position(&h.world, citizen));
    // Every seeded enterprise is a state enterprise with a householder manager.
    assert!(
        h.world
            .orgs
            .values()
            .all(|o| o.kind == OrgKind::StateEnterprise && o.manager.is_some())
    );
    // A cycle-end fill assigns the newcomer householder too.
    h.check_every_step = false;
    h.run_cycle();
    assert!(
        h.world
            .citizens
            .values()
            .filter(|c| !c.dormant)
            .all(|c| has_position(&h.world, c.id))
    );
}

#[test]
fn a_good_off_the_price_list_is_rejected() {
    let mut h = directorate_humans(1);
    assert_eq!(
        request(&mut h, CitizenId(0), Good::Machines, 1),
        Err(RejectCode::NotOnPriceList)
    );
    assert_eq!(
        request(&mut h, CitizenId(0), Good::Food, 0),
        Err(RejectCode::InvalidQuantity)
    );
    let cap = h.world.params.pantry[&Good::Food];
    assert_eq!(
        request(&mut h, CitizenId(0), Good::Food, cap + 1),
        Err(RejectCode::PantryFull)
    );
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .build();
    assert_eq!(
        request(&mut f, CitizenId(0), Good::Food, 1),
        Err(RejectCode::NotInThisSociety)
    );
    let mut c = WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .build();
    assert_eq!(
        request(&mut c, CitizenId(0), Good::Food, 1),
        Err(RejectCode::NotInThisSociety)
    );
}
