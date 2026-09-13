#![allow(clippy::many_single_char_names)]
//! S0.17a, the Republic householder economy: forty householders run three
//! Republic cycles with zero rejected commands, tax is collected every cycle
//! and conserves against the treasury, and the 3-cycle event log is
//! byte-stable.

use isms_core::event::Event;
use isms_core::householder::run_round;
use isms_core::money::Money;
use isms_core::test_support::{Harness, WorldBuilder, check_golden};

fn seeded(seed: u64) -> Harness {
    WorldBuilder::new("republic")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed(seed)
        .seed_epoch()
        .build()
}

fn step(h: &mut Harness) -> Vec<Event> {
    let rules = h.rules();
    let tick = h.world.meta.tick;
    let mut scratch = h.world.clone();
    let (events, rejected) = run_round(&mut scratch, &rules, tick);
    assert!(rejected.is_empty(), "{rejected:#?}");
    h.apply_all(events.iter().cloned());
    let mut all = events;
    all.extend(h.tick());
    all
}

#[test]
fn forty_householders_pay_tax_for_three_cycles_without_a_rejection() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mut collected = Money::ZERO;
    let mut floor = Money::ZERO;
    for _ in 0..72 {
        for e in step(&mut h) {
            match e {
                Event::TaxAssessed { tax, .. } => collected += tax,
                Event::NeedFloorPaid { amount, .. } => floor += amount,
                _ => {}
            }
        }
    }
    h.check();
    assert!(collected > Money::ZERO, "tax was collected");
    assert_eq!(
        h.world.treasury,
        collected - floor,
        "treasury = tax in - floor out"
    );
    let rate = h.world.policy.tax_rate.unwrap();
    assert!(rate > 0.0);
    assert!(
        h.world
            .citizens
            .values()
            .all(|c| c.wages_total > Money::ZERO),
        "everyone earned and was assessed"
    );
}

#[test]
fn golden_three_cycle_republic() {
    let mut h = seeded(7);
    h.check_every_step = false;
    let mut log = h.log.clone();
    for _ in 0..72 {
        log.extend(step(&mut h));
    }
    h.check();
    check_golden("s0_17_republic_3_cycles", &log);
}
