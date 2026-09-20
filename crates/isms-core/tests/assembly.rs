//! S2.4: the simulator's scripted assembly (`planner.rs`) as pure functions
//! of `World`: presence each cycle, the split nudged toward the scarcest sink,
//! a slate that fills every seat from cycle 1, and no rejected command.

use isms_core::command::Command;
use isms_core::constitution::OfficeKind;
use isms_core::event::Event;
use isms_core::planner::{
    Sink, decide_assembly, decide_presence, nudged_split, run_assembly_round, top_contributor,
};
use isms_core::policy::MaterialsSplit;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::ProposalKind;

fn commune(humans: u32) -> Harness {
    WorldBuilder::new("commune")
        .seed(4)
        .with_preset(|p| p.params.sim.assembly_size = humans)
        .humans(humans)
        .seed_epoch()
        .build()
}

/// One cycle with the assembly script before every tick.
fn run_cycle(h: &mut Harness) -> Vec<Event> {
    let rules = h.rules();
    let tpc = h.world.params.time.ticks_per_cycle;
    let mut events = Vec::new();
    for _ in 0..tpc {
        let tick = h.world.meta.tick;
        let (round, rejected) = run_assembly_round(&mut h.world, &rules, tick);
        assert!(rejected.is_empty(), "tick {tick}: {rejected:?}");
        h.log.extend(round.iter().cloned());
        events.extend(round);
        events.extend(h.tick());
    }
    events
}

#[test]
fn the_split_moves_toward_the_scarcest_sink_and_still_sums_to_one() {
    let split = MaterialsSplit {
        wares: 0.5,
        machines: 0.3,
        dwellings: 0.2,
    };
    let s = nudged_split(split, Some(Sink::Machines), 0.05);
    assert!(s.machines > split.machines && s.wares < split.wares && s.dwellings < split.dwellings);
    assert!((s.wares + s.machines + s.dwellings - 1.0).abs() < 1e-12);
    // Nothing goes below zero and a whole sink can be reached.
    let mut s = split;
    for _ in 0..100 {
        s = nudged_split(s, Some(Sink::Dwellings), 0.05);
        assert!(s.wares >= 0.0 && s.machines >= 0.0);
        assert!((s.wares + s.machines + s.dwellings - 1.0).abs() < 1e-12);
    }
    assert!(s.dwellings > 0.99);
    // With nothing short the split drifts back toward even thirds.
    for _ in 0..200 {
        s = nudged_split(s, None, 0.05);
        assert!((s.wares + s.machines + s.dwellings - 1.0).abs() < 1e-12);
    }
    assert!((s.wares - 1.0 / 3.0).abs() < 1e-3 && (s.dwellings - 1.0 / 3.0).abs() < 1e-3);
}

#[test]
fn six_humans_fill_three_seats_from_cycle_one_and_carry_the_split() {
    let mut h = commune(6);
    assert_eq!(top_contributor(&h.world), None, "nobody has worked yet");
    let humans: Vec<_> = (0..6).map(|i| nth(&h, i)).collect();
    // Tick 0: everyone is present and stands; the speaker moves the split.
    assert_eq!(decide_presence(&h.world, 0), humans);
    let speaker = decide_assembly(&h.world, humans[0], 0);
    assert!(speaker.iter().any(|c| matches!(
        c,
        Command::Propose {
            kind: ProposalKind::PolicyChange { patch },
            ..
        } if patch.materials_split.is_some()
    )));
    assert!(speaker.iter().any(|c| matches!(
        c,
        Command::Stand {
            office: OfficeKind::Coordinator
        }
    )));
    assert!(
        decide_assembly(&h.world, humans[1], 0)
            .iter()
            .all(|c| !matches!(c, Command::Propose { .. })),
        "only the speaker moves"
    );
    let before = h.world.policy.materials_split.unwrap();
    let events = run_cycle(&mut h);
    assert_eq!(h.world.offices.filled(OfficeKind::Coordinator), 3);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::PolicyChanged {
            proposal: Some(_),
            ..
        }
    )));
    assert_ne!(h.world.policy.materials_split.unwrap(), before);
    assert!(
        h.world.proposals.is_empty(),
        "the cycle's motion closed with it"
    );
    // The other three take over when the first terms end (no consecutive terms).
    let first: Vec<_> = h.world.offices.holders[&OfficeKind::Coordinator]
        .iter()
        .map(|s| s.citizen)
        .collect();
    for _ in 0..5 {
        run_cycle(&mut h);
        assert_eq!(h.world.offices.filled(OfficeKind::Coordinator), 3);
    }
    let second: Vec<_> = h.world.offices.holders[&OfficeKind::Coordinator]
        .iter()
        .map(|s| s.citizen)
        .collect();
    assert!(
        first.iter().all(|c| !second.contains(c)),
        "{first:?} vs {second:?}"
    );
    assert!(
        top_contributor(&h.world).is_some(),
        "the humans work like householders"
    );
    h.check();
}
