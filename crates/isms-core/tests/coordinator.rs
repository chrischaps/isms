//! S2.3 done gate: the coordinator's three powers and the assembly's honors.
//! A non-holder's `SetPlan` is refused; closing a workplace with workers emits
//! their unassignments and conserves the Materials moved; a non-coordinator's
//! rationing proposal is refused; an honor lands on the record; the System's
//! `SetPolicy` is refused under a direct assembly outside the simulator.

#![allow(clippy::many_single_char_names, clippy::float_cmp)]

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::constitution::OfficeKind;
use isms_core::event::{Actor, Event};
use isms_core::ids::{CitizenId, ProposalId, SlotId, WorkplaceId};
use isms_core::kinds::{ClientKind, Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, conservation_check, goods_at};
use isms_core::policy::{MaterialsSplit, PolicyPatch, Rationing};
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Ballot, ProposalKind};
use std::collections::BTreeMap;

const FARM: WorkplaceId = WorkplaceId(0);
const MILL: WorkplaceId = WorkplaceId(1);

/// A Commune with its collective, a Farm (two machines, two workers), a Mill,
/// fifty Materials in the Common Store and citizen 0 seated as a coordinator
/// through the event an election emits (S2.2), so the fold check stays on.
fn commune() -> Harness {
    let mut h = WorldBuilder::new("commune")
        .seed(5)
        .humans(4)
        .org(OrgKind::Collective, "The Collective")
        .workplace(WorkplaceKind::Farm, 0, 2)
        .workplace(WorkplaceKind::Mill, 0, 0)
        .assign(1, 0, 6, Effort::Normal)
        .assign(2, 0, 6, Effort::Normal)
        .build();
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Materials, 50),
    });
    h.apply(Event::OfficeTaken {
        office: OfficeKind::Coordinator,
        citizen: nth(&h, 0),
        term_ends_cycle: 5,
        approvals: 1,
    });
    h
}

fn set_plan(
    h: &mut Harness,
    who: CitizenId,
    targets: &[(WorkplaceId, f64)],
) -> Result<Vec<Event>, RejectCode> {
    h.cmd(Envelope::citizen(
        who,
        Command::SetPlan {
            targets: targets.iter().copied().collect(),
            materials_split: None,
            price_list: None,
            wage_grades: None,
            ration_caps: None,
        },
        0,
    ))
    .map_err(|e| e.code)
}

fn open(
    h: &mut Harness,
    who: CitizenId,
    kind: WorkplaceKind,
    slot: Option<SlotId>,
) -> Result<Vec<Event>, RejectCode> {
    h.cmd(Envelope::citizen(
        who,
        Command::OpenWorkplace { kind, slot },
        0,
    ))
    .map_err(|e| e.code)
}

fn close(
    h: &mut Harness,
    who: CitizenId,
    workplace: WorkplaceId,
) -> Result<Vec<Event>, RejectCode> {
    h.cmd(Envelope::citizen(
        who,
        Command::CloseWorkplace { workplace },
        0,
    ))
    .map_err(|e| e.code)
}

fn store(h: &Harness, good: Good) -> u32 {
    goods_at(&h.world, Holder::Store, good)
}

fn propose_honor(h: &mut Harness, who: CitizenId, of: CitizenId) -> Result<ProposalId, RejectCode> {
    let events = h
        .cmd(Envelope::citizen(
            who,
            Command::Propose {
                title: "for the harvest".into(),
                text: "they carried the sacks".into(),
                kind: ProposalKind::Honor { citizen: of },
            },
            0,
        ))
        .map_err(|e| e.code)?;
    let Event::Proposed { proposal, .. } = events[0] else {
        panic!("{events:?}")
    };
    Ok(proposal)
}

#[test]
fn the_plan_is_a_coordinators_and_targets_only() {
    let mut h = commune();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    assert!(h.world.offices.holds(a, OfficeKind::Coordinator));
    // A citizen who does not sit is refused with the office code (S2.2).
    assert_eq!(
        set_plan(&mut h, b, &[(FARM, 100.0)]).unwrap_err(),
        RejectCode::NotAnOfficeHolder
    );
    // Nor is the System the Commune's planner any more.
    let err = h
        .cmd(Envelope::system(
            Command::SetPlan {
                targets: BTreeMap::from([(FARM, 100.0)]),
                materials_split: None,
                price_list: None,
                wage_grades: None,
                ration_caps: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    // The coordinator publishes targets, signed.
    let events = set_plan(&mut h, a, &[(FARM, 100.0), (MILL, 40.0)]).unwrap();
    assert!(matches!(
        &events[..],
        [Event::PlanPublished { by: Actor::Citizen(c), targets, cycle: 0 }]
            if *c == a && targets.len() == 2
    ));
    assert_eq!(h.world.workplaces[&FARM].target, Some(100.0));
    assert_eq!(h.world.workplaces[&MILL].target, Some(40.0));
    // Targets only: the split is the assembly's vote, not the Plan's.
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::SetPlan {
                targets: BTreeMap::new(),
                materials_split: Some(MaterialsSplit {
                    wares: 0.4,
                    machines: 0.4,
                    dwellings: 0.2,
                }),
                price_list: None,
                wage_grades: None,
                ration_caps: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    // Advisory: a cycle at target pays nobody a bonus and moves no target.
    let events = h.run_cycle();
    assert!(!events.iter().any(|e| matches!(e, Event::Paid { .. })));
    assert!(!events.iter().any(|e| matches!(e, Event::TargetSet { .. })));
    assert_eq!(h.world.workplaces[&FARM].target, Some(100.0));
    h.check();
}

#[test]
fn opening_spends_the_founding_materials_from_the_store() {
    let mut h = commune();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    let materials = h.world.params.founding.materials;
    assert_eq!(materials, 20);
    assert_eq!(
        open(&mut h, b, WorkplaceKind::Farm, None).unwrap_err(),
        RejectCode::NotAnOfficeHolder
    );
    let before = store(&h, Good::Materials);
    let events = open(&mut h, a, WorkplaceKind::Farm, None).unwrap();
    let [
        Event::WorkplaceOpened {
            workplace,
            org,
            kind,
            slot,
            materials_consumed,
            by,
        },
    ] = &events[..]
    else {
        panic!("{events:?}")
    };
    assert_eq!(
        (*kind, *by, *materials_consumed),
        (WorkplaceKind::Farm, a, materials)
    );
    assert_eq!(store(&h, Good::Materials), before - materials);
    let wp = &h.world.workplaces[workplace];
    assert_eq!((wp.org, wp.slot), (*org, *slot));
    assert_eq!(
        h.world.land.slots[&slot.unwrap()].workplace,
        Some(*workplace)
    );
    assert!(h.world.orgs[org].workplaces.contains(workplace));
    assert_eq!(h.world.orgs[org].kind, OrgKind::Collective);
    conservation_check(&h.world).unwrap();
    // An occupied slot is not on offer; the Store's floor is the refusal.
    assert_eq!(
        open(&mut h, a, WorkplaceKind::Farm, *slot).unwrap_err(),
        RejectCode::NoSlotAvailable
    );
    open(&mut h, a, WorkplaceKind::Mine, None).unwrap();
    assert_eq!(store(&h, Good::Materials), 10);
    assert_eq!(
        open(&mut h, a, WorkplaceKind::Mill, None).unwrap_err(),
        RejectCode::InsufficientGoods
    );
    // Freeport has no coordinators and no collective: the command does not exist.
    let mut f = WorldBuilder::new("freeport").humans(1).build();
    let err = f
        .cmd(Envelope::citizen(
            nth(&f, 0),
            Command::OpenWorkplace {
                kind: WorkplaceKind::Farm,
                slot: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    h.check();
}

#[test]
fn closing_unassigns_the_workers_and_returns_the_machines() {
    let mut h = commune();
    let (a, b, c, d) = (nth(&h, 0), nth(&h, 1), nth(&h, 2), nth(&h, 3));
    assert_eq!(h.world.workplaces[&FARM].workers.len(), 2);
    assert_eq!(h.world.workplaces[&FARM].machines, 2);
    let materials = store(&h, Good::Materials);
    let machines = store(&h, Good::Machines);
    assert_eq!(
        close(&mut h, d, FARM).unwrap_err(),
        RejectCode::NotAnOfficeHolder
    );
    assert_eq!(
        close(&mut h, a, WorkplaceId(9)).unwrap_err(),
        RejectCode::UnknownWorkplace
    );
    let farm_slot = h.world.workplaces[&FARM].slot.unwrap();
    let events = close(&mut h, a, FARM).unwrap();
    assert_eq!(
        events,
        vec![
            Event::Unassigned {
                workplace: FARM,
                citizen: b
            },
            Event::Unassigned {
                workplace: FARM,
                citizen: c
            },
            Event::WorkplaceClosed {
                workplace: FARM,
                org: isms_core::ids::OrgId(0),
                slot: Some(farm_slot),
                machines_returned: 2,
                by: a,
            },
        ]
    );
    assert!(!h.world.workplaces.contains_key(&FARM));
    assert!(h.citizen(b).labor.allocations.is_empty());
    assert!(h.citizen(c).labor.allocations.is_empty());
    assert_eq!(h.world.land.slots[&farm_slot].workplace, None);
    assert!(
        !h.world.orgs[&isms_core::ids::OrgId(0)]
            .workplaces
            .contains(&FARM)
    );
    // The Materials the opening consumed stay consumed; the machines come home.
    assert_eq!(store(&h, Good::Materials), materials);
    assert_eq!(store(&h, Good::Machines), machines + 2);
    conservation_check(&h.world).unwrap();
    // The slot is free again for the next opening, and the world still ticks.
    let events = open(&mut h, a, WorkplaceKind::Farm, Some(farm_slot)).unwrap();
    assert!(
        matches!(&events[..], [Event::WorkplaceOpened { slot: Some(s), .. }] if *s == farm_slot)
    );
    h.run_cycle();
    h.check();
    // Only the collective's workplaces are the coordinator's to close.
    let mut g = WorldBuilder::new("commune")
        .seed(6)
        .humans(1)
        .org(OrgKind::Association, "The Guild")
        .workplace(WorkplaceKind::Mill, 0, 0)
        .build();
    let me = nth(&g, 0);
    g.apply(Event::OfficeTaken {
        office: OfficeKind::Coordinator,
        citizen: me,
        term_ends_cycle: 5,
        approvals: 1,
    });
    assert_eq!(
        close(&mut g, me, FARM).unwrap_err(),
        RejectCode::NotAuthorized
    );
    assert_eq!(
        open(&mut g, me, WorkplaceKind::Farm, None).unwrap_err(),
        RejectCode::NotInThisSociety
    );
}

#[test]
fn the_rationing_rule_is_a_coordinators_to_propose() {
    let mut h = commune();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    let lottery = || Command::Propose {
        title: "lottery".into(),
        text: "fair is random".into(),
        kind: ProposalKind::PolicyChange {
            patch: PolicyPatch {
                rationing: Some(Rationing::Lottery),
                ..PolicyPatch::default()
            },
        },
    };
    let err = h.cmd(Envelope::citizen(b, lottery(), 0)).unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    let events = h.cmd(Envelope::citizen(a, lottery(), 0)).unwrap();
    assert!(matches!(&events[..], [Event::Proposed { by, .. }] if *by == a));
    // Any other field is anyone's to propose.
    let ok = h
        .cmd(Envelope::citizen(
            b,
            Command::Propose {
                title: "five hours".into(),
                text: "x".into(),
                kind: ProposalKind::PolicyChange {
                    patch: PolicyPatch {
                        work_norm_hours: Some(5),
                        ..PolicyPatch::default()
                    },
                },
            },
            0,
        ))
        .is_ok();
    assert!(ok);
}

#[test]
fn an_honor_lands_on_the_record() {
    let mut h = commune();
    let ids = h.citizen_ids();
    let (a, b) = (ids[0], ids[1]);
    assert_eq!(
        propose_honor(&mut h, a, CitizenId(99)).unwrap_err(),
        RejectCode::UnknownCitizen
    );
    let p = propose_honor(&mut h, a, b).unwrap();
    // One motion per subject per cycle (Q119).
    assert_eq!(
        propose_honor(&mut h, ids[2], b).unwrap_err(),
        RejectCode::AlreadyExists
    );
    // A motion that fails leaves no mark.
    let q = propose_honor(&mut h, b, a).unwrap();
    for id in &ids {
        h.cmd(Envelope::citizen(
            *id,
            Command::Vote {
                proposal: p,
                ballot: Ballot::Yes,
            },
            0,
        ))
        .unwrap();
        h.cmd(Envelope::citizen(
            *id,
            Command::Vote {
                proposal: q,
                ballot: Ballot::No,
            },
            0,
        ))
        .unwrap();
    }
    let events = h.run_cycle();
    let honored: Vec<(CitizenId, ProposalId, u32)> = events
        .iter()
        .filter_map(|e| match e {
            Event::Honored {
                citizen,
                proposal,
                cycle,
            } => Some((*citizen, *proposal, *cycle)),
            _ => None,
        })
        .collect();
    assert_eq!(honored, vec![(b, p, 0)]);
    assert_eq!(h.citizen(b).honors.len(), 1);
    assert_eq!(h.citizen(b).honors[0].proposal, p);
    assert!(h.citizen(a).honors.is_empty());
    assert!(h.world.proposals.is_empty());
    // The scoreboard reads the record.
    assert_eq!(isms_core::metrics::honors_of(h.citizen(b)), 1);
    let summary =
        isms_core::metrics::epoch_summary(&h.world, isms_core::metrics::aggregates(&h.world, 0));
    let row = summary.standings.iter().find(|s| s.citizen == b).unwrap();
    assert_eq!(row.honors, 1);
    // Never revoked, and a second honor next cycle is a second line.
    let p2 = propose_honor(&mut h, a, b).unwrap();
    for id in &ids {
        h.cmd(Envelope::citizen(
            *id,
            Command::Vote {
                proposal: p2,
                ballot: Ballot::Yes,
            },
            0,
        ))
        .unwrap();
    }
    h.run_cycle();
    assert_eq!(h.citizen(b).honors.len(), 2);
    h.check();
}

#[test]
fn the_systems_set_policy_is_refused_under_a_direct_assembly_outside_the_sim() {
    let mut h = commune();
    let mut policy = h.world.policy.clone();
    policy.rationing = Some(Rationing::Lottery);
    let cmd = Command::SetPolicy {
        policy: Box::new(policy),
    };
    for via in [ClientKind::Web, ClientKind::ApiKey] {
        let err = h
            .cmd(Envelope::system(cmd.clone(), 0).via(via))
            .unwrap_err();
        assert_eq!(err.code, RejectCode::NotAuthorized, "{via:?}");
    }
    assert_eq!(h.world.policy.rationing, Some(Rationing::NeedFirst));
    // The simulator's stand-in stays (TDD S0.15 "set by System in sim").
    h.cmd(Envelope::system(cmd, 0)).unwrap();
    assert_eq!(h.world.policy.rationing, Some(Rationing::Lottery));
    // A Committee's society keeps the System until Phase 4's Committee exists.
    let mut d = WorldBuilder::new("directorate").humans(1).build();
    let policy = d.world.policy.clone();
    d.cmd(
        Envelope::system(
            Command::SetPolicy {
                policy: Box::new(policy),
            },
            0,
        )
        .via(ClientKind::Web),
    )
    .unwrap();
}
