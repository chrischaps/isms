//! S0.6b done gate (TDD §18.2 S0.6, rest): founding, slots, worker cap,
//! machines, depreciation, manager checks.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::{OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::Party;
use isms_core::money::Money;
use isms_core::orgs::{check_room, controlling_owner};
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Ownership, ShareHolder};

fn founder_world() -> Harness {
    WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(2)
        .pantry(0, Good::Materials, 45)
        .build()
}

fn kinds(events: &[Event]) -> Vec<&'static str> {
    events.iter().map(Event::kind).collect()
}

#[test]
fn founding_a_firm_with_a_mine_costs_fee_and_materials() {
    let mut h = founder_world();
    let me = nth(&h, 0);
    let events = h
        .cmd(Envelope::citizen(
            me,
            Command::FoundOrg {
                kind: OrgKind::Firm,
                name: "Iron & Sons".into(),
                first_workplace: Some((WorkplaceKind::Mine, None)),
            },
            0,
        ))
        .unwrap();
    assert_eq!(
        kinds(&events),
        ["OrgFounded", "Transferred", "WorkplaceAdded", "Assigned"]
    );
    let org = &h.world.orgs[&OrgId(0)];
    assert_eq!(org.manager, Some(me));
    assert_eq!(controlling_owner(org), Some(me));
    assert!(
        matches!(&org.ownership, Ownership::Shares { issued: 100, holdings } if holdings[&ShareHolder::Citizen(me)] == 100)
    );
    assert_eq!(h.citizen(me).household.balance, Money::credits(800));
    assert_eq!(h.citizen(me).household.pantry[&Good::Materials], 25);
    assert_eq!(h.world.ledger_meta.burned_money, Money::credits(200));
    let wp = &h.world.workplaces[&WorkplaceId(0)];
    assert_eq!(wp.kind, WorkplaceKind::Mine);
    assert!(wp.slot.is_some());
    assert_eq!(
        h.world.land.slots[&wp.slot.unwrap()].workplace,
        Some(WorkplaceId(0))
    );
    assert_eq!(h.world.next.org, OrgId(1));
    assert_eq!(h.world.next.workplace, WorkplaceId(1));
}

#[test]
fn founding_is_refused_without_fee_or_materials_or_when_destitute() {
    let h = founder_world();
    let poor = nth(&h, 1); // no Materials
    let r = h.cmd_dry(Envelope::citizen(
        poor,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "x".into(),
            first_workplace: Some((WorkplaceKind::Mill, None)),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientGoods);
    // without a workplace only the fee applies
    let r = h.cmd_dry(Envelope::citizen(
        poor,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "x".into(),
            first_workplace: None,
        },
        0,
    ));
    assert!(r.is_ok());
    let mut h = founder_world();
    let me = nth(&h, 0);
    h.apply(Event::Transferred {
        from: Party::Citizen(me),
        to: Party::Citizen(nth(&h, 1)),
        asset: isms_core::ledger::Asset::Money(Money::credits(900)),
        memo: String::new(),
    });
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "x".into(),
            first_workplace: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientFunds);
    h.apply(Event::DestitutionBegan {
        citizen: me,
        cycle: 0,
    });
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::FoundOrg {
            kind: OrgKind::Association,
            name: "x".into(),
            first_workplace: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::OptionsNarrowed);
    let r = h.cmd_dry(Envelope::citizen(
        nth(&h, 1),
        Command::FoundOrg {
            kind: OrgKind::Collective,
            name: "x".into(),
            first_workplace: None,
        },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::NotInThisSociety,
        "no collectives in Freeport"
    );
}

#[test]
fn the_ninth_farm_is_rejected_and_mills_need_no_slot() {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .pantry(0, Good::Materials, 100)
        .org(OrgKind::Firm, "Legacy");
    for _ in 0..8 {
        b = b.workplace(WorkplaceKind::Farm, 0, 0);
    }
    let mut h = b.build();
    let me = nth(&h, 0);
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "Nine".into(),
            first_workplace: Some((WorkplaceKind::Farm, None)),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NoSlotAvailable);
    let ok = h
        .cmd(Envelope::citizen(
            me,
            Command::FoundOrg {
                kind: OrgKind::Firm,
                name: "Mill".into(),
                first_workplace: Some((WorkplaceKind::Mill, None)),
            },
            0,
        ))
        .unwrap();
    assert_eq!(
        ok.len(),
        4,
        "founded, Materials moved, added, the founder placed"
    );
    assert_eq!(h.world.workplaces[&WorkplaceId(8)].slot, None);
}

#[test]
fn add_workplace_needs_the_manager_and_org_materials() {
    let mut h = founder_world();
    let me = nth(&h, 0);
    h.cmd(Envelope::citizen(
        me,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "x".into(),
            first_workplace: None,
        },
        0,
    ))
    .unwrap();
    let other = nth(&h, 1);
    let r = h.cmd_dry(Envelope::citizen(
        other,
        Command::AddWorkplace {
            org: OrgId(0),
            kind: WorkplaceKind::Mill,
            slot: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotManager);
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::AddWorkplace {
            org: OrgId(0),
            kind: WorkplaceKind::Mill,
            slot: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientGoods);
    h.cmd(Envelope::citizen(
        me,
        Command::Transfer {
            to: Party::Org(OrgId(0)),
            asset: isms_core::ledger::Asset::Good(Good::Materials, 20),
            memo: String::new(),
        },
        0,
    ))
    .unwrap_or_else(|e| {
        // Transfer is S0.7; seed instead
        assert_eq!(e.code, RejectCode::NotImplemented);
        Vec::new()
    });
    if !h.world.orgs[&OrgId(0)]
        .inventory
        .contains_key(&Good::Materials)
    {
        h.apply(Event::Seeded {
            holder: isms_core::ledger::Holder::Org(OrgId(0)),
            asset: isms_core::ledger::Asset::Good(Good::Materials, 20),
        });
    }
    let events = h
        .cmd(Envelope::citizen(
            me,
            Command::AddWorkplace {
                org: OrgId(0),
                kind: WorkplaceKind::Mill,
                slot: None,
            },
            0,
        ))
        .unwrap();
    assert_eq!(kinds(&events), ["WorkplaceAdded"]);
    assert_eq!(
        h.world.orgs[&OrgId(0)].inventory.get(&Good::Materials),
        None
    );
    assert_eq!(h.world.ledger_meta.consumed[&Good::Materials], 20);
}

#[test]
fn the_seventh_worker_is_rejected() {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(7)
        .org(OrgKind::Firm, "x")
        .workplace(WorkplaceKind::Mine, 0, 0);
    for i in 0..6 {
        b = b.assign(i, 0, 8, Effort::Normal);
    }
    let h = b.build();
    assert_eq!(
        check_room(&h.world, WorkplaceId(0)).unwrap_err().code,
        RejectCode::WorkplaceFull
    );
    assert_eq!(
        check_room(&h.world, WorkplaceId(3)).unwrap_err().code,
        RejectCode::UnknownWorkplace
    );
}

#[test]
fn machines_install_uninstall_and_depreciate() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .org(OrgKind::Firm, "x")
        .workplace(WorkplaceKind::Mine, 0, 0)
        .org_inventory(0, Good::Machines, 100)
        .build();
    let me = nth(&h, 0);
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(me),
    });
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::InstallMachines {
            org: OrgId(0),
            workplace: WorkplaceId(0),
            qty: 101,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientGoods);
    h.cmd(Envelope::citizen(
        me,
        Command::InstallMachines {
            org: OrgId(0),
            workplace: WorkplaceId(0),
            qty: 100,
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.workplaces[&WorkplaceId(0)].machines, 100);
    assert_eq!(h.world.orgs[&OrgId(0)].inventory.get(&Good::Machines), None);
    h.check_every_step = false;
    let events = h.run_cycle();
    let dep = events
        .iter()
        .find(|e| matches!(e, Event::MachinesDepreciated { .. }));
    assert!(
        matches!(dep, Some(Event::MachinesDepreciated { qty: 2, .. })),
        "{dep:?}"
    );
    let wp = &h.world.workplaces[&WorkplaceId(0)];
    assert_eq!(wp.machines, 98);
    assert!(wp.machine_wear.abs() < 1e-9);
    assert_eq!(h.world.ledger_meta.depreciated[&Good::Machines], 2);
    h.check();
    // 98 * 0.02 = 1.96: one machine lost, 0.96 carried
    h.run_cycle();
    let wp = &h.world.workplaces[&WorkplaceId(0)];
    assert_eq!(wp.machines, 97);
    assert!((wp.machine_wear - 0.96).abs() < 1e-9);
    h.cmd(Envelope::citizen(
        me,
        Command::UninstallMachines {
            org: OrgId(0),
            workplace: WorkplaceId(0),
            qty: 97,
        },
        48,
    ))
    .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].inventory[&Good::Machines], 97);
    assert_eq!(h.world.workplaces[&WorkplaceId(0)].machines, 0);
    h.check();
}

#[test]
fn appoint_manager_needs_the_controlling_owner() {
    let mut h = founder_world();
    let (me, other) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(
        me,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "x".into(),
            first_workplace: None,
        },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        other,
        Command::AppointManager {
            org: OrgId(0),
            citizen: Some(other),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotControllingOwner);
    h.cmd(Envelope::citizen(
        me,
        Command::AppointManager {
            org: OrgId(0),
            citizen: Some(other),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].manager, Some(other));
}

#[test]
fn the_founder_holds_an_unpaid_position_at_its_first_workplace() {
    use isms_core::world::Allocation;
    let mut h = founder_world();
    let me = nth(&h, 0);
    h.cmd(Envelope::citizen(
        me,
        Command::FoundOrg {
            kind: OrgKind::Firm,
            name: "Iron & Sons".into(),
            first_workplace: Some((WorkplaceKind::Mine, None)),
        },
        0,
    ))
    .unwrap();
    // The owner-operator position (E-3, Q160): no contract, as a coop member holds.
    let a = &h.world.workplaces[&WorkplaceId(0)].workers[&me];
    assert_eq!(a.contract, None);
    assert!(isms_core::labor::has_position(&h.world, me));
    h.cmd(Envelope::citizen(
        me,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: WorkplaceId(0),
                hours: 8,
                effort: Effort::Normal,
            }],
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    let events = h.run_cycle();
    assert!(
        !events.iter().any(
            |e| matches!(e, Event::Paid { citizen, org, .. } if *citizen == me && *org == OrgId(0))
        ),
        "no wage: the output of the firm is the return"
    );
    assert!(
        events.iter().any(
            |e| matches!(e, Event::Produced { workplace, .. } if *workplace == WorkplaceId(0))
        ),
        "the hours of the founder produce"
    );
    assert!(
        h.world.orgs[&OrgId(0)]
            .inventory
            .get(&Good::Ore)
            .is_some_and(|q| *q > 0),
        "ore in the inventory of the firm"
    );
    assert!(
        h.world.workplaces[&WorkplaceId(0)]
            .workers
            .contains_key(&me),
        "the position survives payroll"
    );
    h.check();
}

#[test]
fn founding_without_a_workplace_grants_no_position() {
    let mut h = founder_world();
    let me = nth(&h, 0);
    let events = h
        .cmd(Envelope::citizen(
            me,
            Command::FoundOrg {
                kind: OrgKind::Firm,
                name: "Iron & Sons".into(),
                first_workplace: None,
            },
            0,
        ))
        .unwrap();
    assert_eq!(kinds(&events), ["OrgFounded"]);
    assert!(!isms_core::labor::has_position(&h.world, me));
}
