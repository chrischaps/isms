//! S0.3b done gate: the harness itself, proptests over generated event
//! sequences, and the first golden.

use isms_core::event::Event;
use isms_core::kinds::Good;
use isms_core::ledger::{Asset, Party};
use isms_core::money::Money;
use isms_core::test_support::strategies::{arb_scenario, nth, run_scenario};
use isms_core::test_support::{
    WorldBuilder, assert_deterministic, assert_fold_equals_live, check_golden, fold,
};
use proptest::prelude::*;

#[test]
fn builder_makes_a_conserved_world_from_events() {
    let h = WorldBuilder::new("freeport")
        .humans(3)
        .householders(2)
        .pantry(0, Good::Food, 12)
        .build();
    assert_eq!(h.world.citizens.len(), 5);
    assert_eq!(h.log.len(), 2 + 5 + 1);
    assert_eq!(h.citizen(nth(&h, 0)).household.pantry[&Good::Food], 12);
    assert_eq!(
        h.citizen(nth(&h, 4)).household.balance,
        Money::credits(1000)
    );
    assert!(h.capabilities().order_books);
}

#[test]
fn commune_builder_mints_no_money() {
    let h = WorldBuilder::new("commune").humans(2).build();
    assert_eq!(h.world.ledger_meta.minted, Money::ZERO);
    assert!(h.world.store.is_some());
}

#[test]
fn fold_of_the_log_equals_live_after_transfers() {
    let mut h = WorldBuilder::new("freeport")
        .humans(2)
        .pantry(0, Good::Wares, 5)
        .build();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    h.apply(Event::Transferred {
        from: Party::Citizen(a),
        to: Party::Citizen(b),
        asset: Asset::Money(Money::credits(10)),
        memo: "x".into(),
    });
    h.apply(Event::Transferred {
        from: Party::Citizen(a),
        to: Party::Citizen(b),
        asset: Asset::Good(Good::Wares, 5),
        memo: String::new(),
    });
    assert_eq!(h.citizen(b).household.pantry[&Good::Wares], 5);
    assert!(!h.citizen(a).household.pantry.contains_key(&Good::Wares));
    assert_fold_equals_live(&h.log, &h.world);
    assert_eq!(fold(&h.log).hash(), h.world.hash());
}

#[test]
#[should_panic(expected = "fold != live")]
fn fold_check_catches_a_mutation_outside_apply() {
    let mut h = WorldBuilder::new("freeport").humans(1).build();
    h.world.citizens.values_mut().next().unwrap().needs.comfort = 3;
    assert_fold_equals_live(&h.log, &h.world);
}

#[test]
fn golden_of_the_builder_log_is_stable() {
    let h = WorldBuilder::new("freeport")
        .seed(7)
        .humans(2)
        .householders(1)
        .build();
    check_golden("s0_3b_builder", &h.log);
}

#[test]
fn builder_is_deterministic() {
    assert_deterministic(3, |seed| {
        WorldBuilder::new("freeport").seed(seed).humans(4).events()
    });
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// TDD S0.3 gate: `apply` never panics on well-formed events, conservation
    /// holds, and the fold equals the live world after every step.
    #[test]
    fn apply_never_panics_and_conserves(steps in arb_scenario(4, 40)) {
        let mut h = WorldBuilder::new("freeport").humans(3).householders(1).build();
        run_scenario(&mut h, &steps);
        h.check();
    }

    #[test]
    fn apply_never_panics_in_the_commune(steps in arb_scenario(3, 30)) {
        let mut h = WorldBuilder::new("commune").humans(3).build();
        run_scenario(&mut h, &steps);
        h.check();
        prop_assert_eq!(h.world.ledger_meta.minted, Money::ZERO);
    }

    #[test]
    fn meters_stay_in_range_and_balances_non_negative(steps in arb_scenario(3, 30)) {
        let mut h = WorldBuilder::new("freeport").humans(3).build();
        run_scenario(&mut h, &steps);
        for c in h.world.citizens.values() {
            prop_assert!(c.needs.food <= 100);
            prop_assert!(!c.household.balance.is_negative());
        }
    }
}
