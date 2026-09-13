#![allow(clippy::many_single_char_names)]
//! S0.15c done gate (the Commune householder): forty householders run three
//! Commune cycles with zero rejected commands, everyone works and is housed by
//! cycle one, Machines from the store get installed, `JoinWorkplace` and
//! `LeaveWorkplace` are gated, and the 3-cycle event log is byte-stable.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::householder::run_round;
use isms_core::ids::{CitizenId, WorkplaceId};
use isms_core::kinds::{Good, WorkplaceKind};
use isms_core::labor::has_position;
use isms_core::ledger::{Asset, Holder};
use isms_core::test_support::{Harness, WorldBuilder, check_golden};
use isms_core::world::LaborPlan;

fn seeded(seed: u64) -> Harness {
    WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed(seed)
        .seed_epoch()
        .build()
}

/// One round of scripts, then one tick; returns all events and the rejections.
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
fn forty_householders_run_three_commune_cycles_without_a_rejection() {
    let mut h = seeded(1);
    h.check_every_step = false;
    let mut rejected = Vec::new();
    let mut drew = 0usize;
    for _ in 0..72 {
        let (events, r) = step(&mut h);
        rejected.extend(r);
        drew += events
            .iter()
            .filter(|e| matches!(e, Event::Drew { .. }))
            .count();
    }
    assert!(
        rejected.is_empty(),
        "every scripted command must be valid: {rejected:#?}"
    );
    h.check();
    for c in h.world.citizens.values() {
        assert!(has_position(&h.world, c.id), "{} has no position", c.handle);
        assert!(c.household.dwelling.is_some(), "{} is unhoused", c.handle);
        assert_eq!(c.plan.labor, LaborPlan::FollowNorm);
        assert!(
            c.labor.allocations.iter().map(|a| a.hours).sum::<u8>() > 0,
            "{} works no hours",
            c.handle
        );
    }
    // The economy moved: Food was grown, milled and drawn; nobody is in hardship.
    let produced = |g: Good| h.world.ledger_meta.produced.get(&g).copied().unwrap_or(0);
    assert!(produced(Good::Grain) > 0 && produced(Good::Food) > 0);
    assert!(drew > 0);
    assert_eq!(
        h.world
            .citizens
            .values()
            .filter(|c| c.flags.in_hardship)
            .count(),
        0
    );
    // The ledger closed three times and everyone met the norm in the last cycle.
    assert!(
        h.world
            .citizens
            .values()
            .all(|c| c.contribution.cycles == 3 && c.contribution.norm_met_cycles >= 2)
    );
}

#[test]
fn everyone_works_and_is_housed_by_cycle_one() {
    let mut h = seeded(2);
    h.check_every_step = false;
    for _ in 0..24 {
        step(&mut h);
    }
    let idle = h
        .world
        .citizens
        .values()
        .filter(|c| !has_position(&h.world, c.id))
        .count();
    let unhoused = h
        .world
        .citizens
        .values()
        .filter(|c| c.household.dwelling.is_none())
        .count();
    assert_eq!((idle, unhoused), (0, 0));
    // The balancing rule spread forty citizens over the eighteen workplaces by weight.
    let farm_workers: usize = h
        .world
        .workplaces
        .values()
        .filter(|w| w.kind == WorkplaceKind::Farm)
        .map(|w| w.workers.len())
        .sum();
    let shop_workers: usize = h
        .world
        .workplaces
        .values()
        .filter(|w| w.kind == WorkplaceKind::MachineShop)
        .map(|w| w.workers.len())
        .sum();
    assert!(
        farm_workers > shop_workers,
        "{farm_workers} vs {shop_workers}"
    );
    h.check();
}

#[test]
fn machines_from_the_store_get_installed() {
    let mut h = seeded(3);
    h.check_every_step = false;
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Machines, 5),
    });
    let (events, rejected) = step(&mut h);
    assert!(rejected.is_empty(), "{rejected:#?}");
    let installed: Vec<WorkplaceId> = events
        .iter()
        .filter_map(|e| match e {
            Event::MachinesInstalled { workplace, qty: 1 } => Some(*workplace),
            _ => None,
        })
        .collect();
    assert_eq!(installed.len(), 5, "one per workplace, five workplaces");
    assert!(
        installed
            .iter()
            .all(|w| h.world.workplaces[w].kind != WorkplaceKind::MachineShop)
    );
    assert_eq!(
        h.world
            .store
            .as_ref()
            .unwrap()
            .stock
            .get(&Good::Machines)
            .copied()
            .unwrap_or(0),
        0
    );
    h.check();
}

#[test]
fn join_and_leave_workplace_are_gated() {
    let mut h = seeded(4);
    let a = CitizenId(0);
    // Not in a market society.
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build();
    let err = f
        .cmd(Envelope::citizen(
            a,
            Command::JoinWorkplace {
                workplace: WorkplaceId(0),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    // Leaving a position one does not hold.
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::LeaveWorkplace {
                workplace: WorkplaceId(0),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAssigned);
    // Joining, then joining again, then a full workplace.
    h.cmd(Envelope::citizen(
        a,
        Command::JoinWorkplace {
            workplace: WorkplaceId(0),
        },
        0,
    ))
    .unwrap();
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::JoinWorkplace {
                workplace: WorkplaceId(0),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::AlreadyExists);
    let max = h.world.params.labor.max_workers_per_workplace;
    for i in 1..max {
        h.cmd(Envelope::citizen(
            CitizenId(i),
            Command::JoinWorkplace {
                workplace: WorkplaceId(0),
            },
            0,
        ))
        .unwrap();
    }
    let err = h
        .cmd(Envelope::citizen(
            CitizenId(max),
            Command::JoinWorkplace {
                workplace: WorkplaceId(0),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::WorkplaceFull);
    // The least-staffed rule now avoids the full Farm.
    assert_ne!(
        isms_core::orgs::least_staffed(&h.world),
        Some(WorkplaceId(0))
    );
    // Leaving works and drops the allocation.
    h.cmd(Envelope::citizen(
        a,
        Command::LeaveWorkplace {
            workplace: WorkplaceId(0),
        },
        0,
    ))
    .unwrap();
    assert!(!has_position(&h.world, a));
}

#[test]
fn golden_three_cycle_commune() {
    let mut h = seeded(7);
    h.check_every_step = false;
    let mut log = h.log.clone();
    for _ in 0..72 {
        let (events, rejected) = step(&mut h);
        assert!(rejected.is_empty(), "{rejected:#?}");
        log.extend(events);
    }
    h.check();
    check_golden("s0_15_commune_3_cycles", &log);
}
