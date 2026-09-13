//! S0.5 done gate (TDD §18.2): decay per effort, eating, clamps, the output
//! multiplier, hardship timing, destitution, fatigue debt, dormancy.

use isms_core::event::Event;
use isms_core::ids::WorkplaceId;
use isms_core::kinds::{Effort, Good};
use isms_core::needs::{FULL, TENTHS, output_multiplier};
use isms_core::test_support::strategies::{arb_scenario, nth, run_scenario};
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::Allocation;
use proptest::prelude::*;

fn fixture() -> Harness {
    WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .build()
}

fn food(h: &Harness) -> u16 {
    h.citizen(nth(h, 0)).needs.food
}

fn set_effort(h: &mut Harness, effort: Effort) {
    let me = nth(h, 0);
    h.apply(Event::LaborSet {
        citizen: me,
        allocations: vec![Allocation {
            workplace: WorkplaceId(0),
            hours: 8,
            effort,
        }],
    });
}

#[test]
fn food_decays_per_effort_level() {
    for (effort, per_tick) in [
        (Effort::Low, 32u16),
        (Effort::Normal, 40),
        (Effort::High, 52),
    ] {
        let mut h = fixture();
        set_effort(&mut h, effort);
        h.tick();
        assert_eq!(food(&h), FULL - per_tick, "{effort:?}");
        h.tick();
        assert_eq!(food(&h), FULL - 2 * per_tick, "{effort:?}");
    }
}

#[test]
fn one_food_per_tick_holds_steady_at_normal_and_loses_1_2_at_high() {
    // The S0.5 table test at the GDD value (one Food = +4). Q20 raised the
    // preset to 6 in S1.0 so a human can climb out of hardship; the mechanic
    // under test is unchanged, so the fixture pins the original constant.
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.needs.food_meter_per_unit = 4;
        })
        .humans(1)
        .pantry(0, Good::Food, 48)
        .build();
    // Start below full so the +4 is not clamped away.
    let me = nth(&h, 0);
    let mut needs = h.citizen(me).needs.clone();
    needs.food = 80 * TENTHS;
    h.set_needs(me, needs);
    h.tick();
    assert_eq!(food(&h), 80 * TENTHS);
    assert_eq!(h.citizen(me).household.pantry[&Good::Food], 47);
    set_effort(&mut h, Effort::High);
    h.tick();
    assert_eq!(food(&h), 80 * TENTHS - 12);
    h.tick();
    assert_eq!(food(&h), 80 * TENTHS - 24);
}

#[test]
fn meters_clamp_at_0_and_100() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .pantry(0, Good::Food, 48)
        .pantry(0, Good::Wares, 48)
        .build();
    h.tick();
    let me = nth(&h, 0);
    assert_eq!(h.citizen(me).needs.food, FULL);
    // unhoused: Comfort decays at twice the rate
    assert_eq!(h.citizen(me).needs.comfort, FULL - 20);
    let mut h = fixture();
    h.check_every_step = false;
    for _ in 0..40 {
        h.tick();
    }
    h.check();
    assert_eq!(food(&h), 0);
    assert_eq!(h.citizen(nth(&h, 0)).needs.shelter, FULL - 40 * 20);
}

#[test]
fn output_multiplier_matches_the_table() {
    let h = fixture();
    let p = &h.world.params;
    let mut needs = h.citizen(nth(&h, 0)).needs.clone();
    for (points, expected) in [(50u16, 1.0), (30, 0.76), (10, 0.52), (0, 0.4)] {
        needs.food = points * TENTHS;
        let (m, explain) = output_multiplier(&needs, true, false, p);
        assert!((m - expected).abs() < 1e-9, "food {points}: {m}");
        assert_eq!(explain.rule, isms_core::explain::RuleId::OutputMultiplier);
    }
    needs.food = 30 * TENTHS;
    let (m, _) = output_multiplier(&needs, false, false, p);
    assert!((m - 0.76 * 0.7).abs() < 1e-9);
    needs.food = 0;
    let (m, _) = output_multiplier(&needs, true, true, p);
    assert!((m - 0.25).abs() < 1e-9);
    // and the tick stores it for the next tick's labor
    let mut h = fixture();
    h.tick();
    assert!(
        (h.citizen(nth(&h, 0)).labor.output_mult - 0.7).abs() < 1e-9,
        "unhoused, full food"
    );
}

#[test]
fn hardship_needs_a_full_cycle_aligned_cycle_below_20() {
    // No food: 100 -> below 20 after 21 ticks, so cycle 0 is not a hardship cycle.
    let mut h = fixture();
    h.check_every_step = false;
    let c0 = h.run_cycle();
    assert!(!c0.iter().any(|e| matches!(e, Event::HardshipBegan { .. })));
    assert!(!h.citizen(nth(&h, 0)).flags.in_hardship);
    assert_eq!(h.citizen(nth(&h, 0)).labor.budget, 8);
    let c1 = h.run_cycle();
    assert!(
        c1.iter()
            .any(|e| matches!(e, Event::HardshipBegan { cycle: 1, .. }))
    );
    let me = nth(&h, 0);
    assert!(h.citizen(me).flags.in_hardship);
    assert_eq!(h.citizen(me).labor.fatigue_debt, 2);
    assert_eq!(h.citizen(me).labor.budget, 6);
    // Eating one Food per tick only holds the meter (Q20), so restore it and
    // feed them through cycle 2: hardship ends, budget returns to 8.
    let mut needs = h.citizen(me).needs.clone();
    needs.food = FULL;
    h.set_needs(me, needs);
    h.apply(Event::Seeded {
        holder: isms_core::ledger::Holder::Citizen(me),
        asset: isms_core::ledger::Asset::Good(Good::Food, 48),
    });
    let c2 = h.run_cycle();
    assert!(
        c2.iter()
            .any(|e| matches!(e, Event::HardshipEnded { cycle: 2, .. }))
    );
    assert!(!h.citizen(me).flags.in_hardship);
    assert_eq!(h.citizen(me).labor.budget, 8);
    h.check();
}

#[test]
fn destitution_at_exactly_the_third_consecutive_hardship_cycle() {
    let mut h = fixture();
    h.check_every_step = false;
    h.run_cycle(); // cycle 0: not hardship
    h.run_cycle(); // 1: hardship #1
    let c2 = h.run_cycle(); // 2: #2
    assert!(
        !c2.iter()
            .any(|e| matches!(e, Event::DestitutionBegan { .. }))
    );
    let c3 = h.run_cycle(); // 3: #3 -> destitute
    assert!(
        c3.iter()
            .any(|e| matches!(e, Event::DestitutionBegan { cycle: 3, .. }))
    );
    let me = nth(&h, 0);
    assert!(h.citizen(me).flags.destitute && h.citizen(me).flags.options_narrowed);
    h.tick();
    assert_eq!(h.citizen(me).needs.comfort, 0, "Comfort collapses");
    assert!((h.citizen(me).labor.output_mult - 0.25 * 0.7).abs() < 1e-9);
    h.check();

    // Three non-consecutive hardship cycles never make a citizen destitute.
    let mut h = fixture();
    h.check_every_step = false;
    let me = nth(&h, 0);
    for _ in 0..3 {
        let mut needs = h.citizen(me).needs.clone();
        needs.food = FULL;
        h.set_needs(me, needs);
        h.apply(Event::Seeded {
            holder: isms_core::ledger::Holder::Citizen(me),
            asset: isms_core::ledger::Asset::Good(Good::Food, 24),
        });
        h.run_cycle(); // fed: not hardship
        let mut needs = h.citizen(me).needs.clone();
        needs.food = 0;
        h.set_needs(me, needs);
        h.run_cycle(); // starved: hardship
        assert!(h.citizen(me).flags.in_hardship);
    }
    assert!(!h.citizen(me).flags.destitute);
    h.check();
}

#[test]
fn a_dormant_citizen_is_untouched() {
    let mut h = fixture();
    let me = nth(&h, 0);
    h.apply(Event::CitizenDormant { citizen: me });
    let before = h.citizen(me).needs.clone();
    h.run_cycle();
    assert_eq!(h.citizen(me).needs, before);
    assert!(!h.citizen(me).flags.in_hardship);
    h.apply(Event::CitizenReturned { citizen: me });
    h.tick();
    assert_ne!(h.citizen(me).needs, before);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn meters_stay_in_range_through_ticks(steps in arb_scenario(3, 30), ticks in 0u32..60) {
        let mut h = WorldBuilder::new("freeport")
            .with_preset(|p| p.params.population.collapse_enabled = false)
            .humans(3)
            .build();
        h.check_every_step = false;
        run_scenario(&mut h, &steps);
        for _ in 0..ticks {
            h.tick();
        }
        h.check();
        for c in h.world.citizens.values() {
            prop_assert!(c.needs.food <= FULL);
            prop_assert!(c.needs.shelter <= FULL);
            prop_assert!(c.needs.comfort <= FULL);
            prop_assert!(c.labor.budget <= 8);
        }
    }
}
