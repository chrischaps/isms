#![allow(clippy::many_single_char_names)]
//! S0.16c done gate (the Directorate householder and the System planner):
//! forty householders run three Directorate cycles with zero rejected
//! commands, everyone is assigned, housed and paid by cycle one, the planner
//! raises targets by five percent, and the 3-cycle event log is byte-stable.

use isms_core::event::Event;
use isms_core::householder::run_round;
use isms_core::ids::WorkplaceId;
use isms_core::labor::has_position;
use isms_core::money::Money;
use isms_core::planner::run_system_round;
use isms_core::test_support::{Harness, WorldBuilder, check_golden};
use isms_core::world::LaborPlan;

fn seeded(seed: u64) -> Harness {
    WorldBuilder::new("directorate")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed(seed)
        .seed_epoch()
        .build()
}

/// The System's round, the householders' round, then one tick.
fn step(h: &mut Harness) -> (Vec<Event>, usize) {
    let rules = h.rules();
    let tick = h.world.meta.tick;
    let mut scratch = h.world.clone();
    let (system, sys_rejected) = run_system_round(&mut scratch, &rules, tick);
    assert!(sys_rejected.is_empty(), "{sys_rejected:#?}");
    let (events, rejected) = run_round(&mut scratch, &rules, tick);
    assert!(rejected.is_empty(), "{rejected:#?}");
    let mut all = system;
    all.extend(events);
    h.apply_all(all.iter().cloned());
    all.extend(h.tick());
    (all, 0)
}

#[test]
fn forty_householders_run_three_directorate_cycles_without_a_rejection() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mut paid = 0usize;
    for _ in 0..72 {
        let (events, _) = step(&mut h);
        paid += events
            .iter()
            .filter(|e| matches!(e, Event::Paid { contract: None, .. }))
            .count();
    }
    h.check();
    assert!(paid > 0, "state wages were paid");
    for c in h.world.citizens.values() {
        assert!(has_position(&h.world, c.id), "{} has no position", c.handle);
        assert!(c.household.dwelling.is_some(), "{} is unhoused", c.handle);
        assert_eq!(c.plan.labor, LaborPlan::AcceptAssignment);
        assert!(c.wages_total > Money::ZERO, "{} was never paid", c.handle);
    }
    assert_eq!(
        h.world
            .citizens
            .values()
            .filter(|c| c.flags.in_hardship)
            .count(),
        0
    );
    assert!(h.world.orgs.values().all(|o| o.treasury == Money::ZERO));
}

#[test]
fn everyone_is_assigned_housed_and_paid_by_cycle_one() {
    let mut h = seeded(2);
    h.check_every_step = false;
    for _ in 0..24 {
        step(&mut h);
    }
    for c in h.world.citizens.values() {
        assert!(has_position(&h.world, c.id));
        assert!(c.household.dwelling.is_some());
        assert!(
            c.wages_total > Money::ZERO,
            "{} unpaid after cycle 0",
            c.handle
        );
    }
    h.check();
}

#[test]
fn the_system_planner_raises_targets_by_five_percent() {
    let mut h = seeded(3);
    h.check_every_step = false;
    // No targets in cycle 0: nothing to plan from yet.
    for _ in 0..24 {
        step(&mut h);
    }
    assert!(h.world.workplaces.values().all(|w| w.target.is_none()));
    let last: Vec<(WorkplaceId, f64)> = h
        .world
        .workplaces
        .values()
        .map(|w| (w.id, w.last_cycle_output))
        .collect();
    assert!(last.iter().any(|(_, o)| *o > 0.0));
    // The first tick of cycle 1 publishes last cycle's output x growth.
    let (events, _) = step(&mut h);
    let published = events.iter().find_map(|e| match e {
        Event::PlanPublished { targets, .. } => Some(targets.clone()),
        _ => None,
    });
    let targets = published.expect("the planner published targets");
    let growth = h.world.params.governance.sim_planner_target_growth;
    for (wp, out) in &last {
        if *out > 0.0 {
            assert_eq!(targets.get(wp), Some(&(out * growth)), "{wp}");
        } else {
            assert_eq!(targets.get(wp), None);
        }
    }
    h.check();
}

#[test]
fn golden_three_cycle_directorate() {
    let mut h = seeded(7);
    h.check_every_step = false;
    let mut log = h.log.clone();
    for _ in 0..72 {
        let (events, _) = step(&mut h);
        log.extend(events);
    }
    h.check();
    check_golden("s0_16_directorate_3_cycles", &log);
}
