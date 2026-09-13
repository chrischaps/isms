//! S0.13a done gate (metrics and rollover): a hand-built 4-citizen fixture yields
//! the hand-computed Gini; aggregate columns are populated; the epoch rollover
//! keeps the roster and resets material state.

use isms_core::event::Event;
use isms_core::ids::OrgId;
use isms_core::kinds::{CitizenKind, Effort, Good, OrgKind, WorkplaceKind};
use isms_core::metrics::{consumption_score, gini};
use isms_core::money::Money;
use isms_core::rules::Rules;
use isms_core::test_support::WorldBuilder;
use isms_core::test_support::strategies::nth;
use isms_core::tick::start_epoch;
use isms_core::world::EpochEndReason;

#[test]
fn gini_matches_hand_computed_values() {
    assert!((gini(&[1.0, 1.0, 1.0, 1.0])).abs() < 1e-12);
    assert!((gini(&[0.0, 0.0, 0.0, 4.0]) - 0.75).abs() < 1e-12);
    assert!((gini(&[1.0, 2.0, 3.0, 4.0]) - 0.25).abs() < 1e-12);
    assert!((gini(&[])).abs() < 1e-12);
}

#[test]
fn a_four_citizen_cycle_yields_the_hand_computed_metrics() {
    // Food: 48, 24, 0, 0 in the pantry; nobody housed; unhoused -> housed_ticks 0.
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(4)
        .pantry(0, Good::Food, 48)
        .pantry(1, Good::Food, 24)
        .build();
    for who in h.citizen_ids() {
        let mut p = h.citizen(who).plan.clone();
        p.keep_food_at_least = 0;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(p),
        });
    }
    h.check_every_step = false;
    let events = h.run_cycle();
    let Some(Event::CycleClosed { aggregates: a, .. }) = events
        .iter()
        .find(|e| matches!(e, Event::CycleClosed { .. }))
    else {
        panic!()
    };
    assert_eq!((a.population, a.active_humans, a.householders), (4, 4, 0));
    // consumption scores: 24, 24, 0, 0 -> Gini 0.5
    assert!(
        (a.consumption_gini - 0.5).abs() < 1e-9,
        "{}",
        a.consumption_gini
    );
    // the two who ate stayed above 20 in Food but everyone is unhoused: Shelter 100 -> 52, above 20
    // the two who did not eat fell to 4 by tick 23: breached
    assert!((a.need_fulfillment_rate - 0.5).abs() < 1e-9);
    assert_eq!(a.unemployed, 4);
    assert_eq!(a.firm_count, 0);
    assert!(a.real_output.abs() < 1e-12);
    assert!(
        a.median_wellbeing > 40.0 && a.median_wellbeing < 90.0,
        "{}",
        a.median_wellbeing
    );
    assert_eq!(a.hardship_count, 0, "hardship needs a full cycle below 20");
    assert_eq!(a.credit_outstanding, Money::ZERO);
    // accumulators were reset
    assert!(h.world.citizens.values().all(|c| c.cycle.ticks == 0));
    let score = consumption_score(
        &h.world,
        &isms_core::metrics::CitizenCycle {
            food_eaten: 10,
            wares_consumed: 2,
            housed_ticks: 4,
            ..Default::default()
        },
    );
    assert!((score - (10.0 + 2.0 + 2.0)).abs() < 1e-12);
    h.check();
}

#[test]
fn real_output_and_investment_share_follow_production() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(2)
        .pantry(0, Good::Food, 48)
        .pantry(1, Good::Food, 48)
        .org(OrgKind::Firm, "x")
        .workplace(WorkplaceKind::Foundry, 0, 0)
        .workplace(WorkplaceKind::MachineShop, 0, 0)
        .org_inventory(0, Good::Ore, 1000)
        .assign(0, 0, 8, Effort::Normal)
        .assign(1, 1, 8, Effort::Normal)
        .build();
    h.check_every_step = false;
    let events = h.run_cycle();
    let Some(Event::CycleClosed { aggregates: a, .. }) = events
        .iter()
        .find(|e| matches!(e, Event::CycleClosed { .. }))
    else {
        panic!()
    };
    let machines = h.world.orgs[&OrgId(0)]
        .inventory
        .get(&Good::Machines)
        .copied()
        .unwrap_or(0);
    assert!(machines > 0);
    assert!(
        (a.real_output - f64::from(machines) * 2.0).abs() < 1e-9,
        "only Machines are in the basket here"
    );
    assert!(
        a.investment_share > 0.0 && a.investment_share <= 1.0,
        "{}",
        a.investment_share
    );
    assert_eq!(a.firm_count, 1);
    h.check();
}

#[test]
fn the_epoch_rollover_keeps_the_roster_and_resets_material_state() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.time.epoch_cycles = 2;
        })
        .humans(2)
        .seed_epoch()
        .build();
    h.check_every_step = false;
    let humans: Vec<_> = h
        .world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Human)
        .map(|c| c.id)
        .collect();
    h.run_cycles(2);
    assert_eq!(h.world.meta.epoch_ended, Some(EpochEndReason::Scheduled));
    let rules = Rules::from_world(&h.world);
    let events = start_epoch(&h.world, &rules, 1);
    assert!(matches!(events[0], Event::EpochStarted { epoch: 1 }));
    h.apply_all(events);
    assert_eq!(h.world.meta.epoch, 1);
    assert_eq!(h.world.meta.tick, 0);
    assert!(h.world.meta.epoch_ended.is_none());
    for id in &humans {
        let c = h.citizen(*id);
        assert!(
            c.dormant,
            "humans start the new epoch dormant until they return"
        );
        assert_eq!(
            c.household.balance,
            Money::credits(1000),
            "a fresh endowment"
        );
        assert!(c.household.pantry.is_empty() && c.household.dwelling.is_none());
    }
    assert_eq!(
        h.world
            .citizens
            .values()
            .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
            .count(),
        40
    );
    assert_eq!(
        h.world.orgs.len(),
        18,
        "fresh legacy firms; the old ones are gone"
    );
    assert!(h.world.contracts.is_empty() && h.world.offers.is_empty() && h.world.escrow.is_empty());
    assert_eq!(h.world.dwellings.len(), 40);
    h.check();
    h.run_cycle();
    let _ = nth(&h, 0);
}
