//! Collapse ends an abandoned epoch, not one that never drew a crowd (ADR-0006).
//! The floor is 2 humans here; humans who are never seen go dormant after
//! `dormancy_absent_cycles`, which is how a society empties in a test.

use isms_core::event::Event;
use isms_core::test_support::WorldBuilder;
use isms_core::world::EpochEndReason;

fn ended(events: &[Event]) -> Option<(EpochEndReason, u32)> {
    events.iter().find_map(|e| match e {
        Event::EpochEnded { reason, cycle } => Some((*reason, *cycle)),
        _ => None,
    })
}

fn society(humans: u32) -> isms_core::test_support::Harness {
    WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.floor = 2;
            p.params.population.collapse_enabled = true;
        })
        .humans(humans)
        .build()
}

#[test]
fn a_society_that_never_reached_the_floor_keeps_running() {
    let mut h = society(1);
    let mut all = Vec::new();
    for _ in 0..20 {
        all.extend(h.run_cycle());
    }
    assert_eq!(
        ended(&all),
        None,
        "one human below a floor of two is not abandonment"
    );
    assert!(!h.world.meta.reached_floor);
    assert_eq!(h.world.meta.low_population_cycles, 0);
}

#[test]
fn a_society_that_reached_the_floor_and_emptied_collapses() {
    let mut h = society(2);
    let first = h.run_cycle();
    assert!(
        h.world.meta.reached_floor,
        "two active humans reach a floor of two"
    );
    assert_eq!(ended(&first), None);
    let absent = h.world.params.population.dormancy_absent_cycles;
    let grace = h.world.params.population.collapse_cycles;
    let mut all = Vec::new();
    let mut cycles = 1;
    while ended(&all).is_none() && cycles < absent + grace + 5 {
        all.extend(h.run_cycle());
        cycles += 1;
    }
    let (reason, cycle) = ended(&all).expect("the emptied society collapses");
    assert_eq!(reason, EpochEndReason::Collapse);
    // `cycle` is the 0-based index. Both humans go dormant at the end of the
    // `absent`th cycle (index absent - 1); the counter then reaches `grace` at
    // the end of index absent - 1 + grace - 1.
    assert_eq!(
        cycle,
        absent + grace - 2,
        "collapse lands after the grace cycles"
    );
    assert_eq!(h.world.meta.low_population_cycles, grace);
}
