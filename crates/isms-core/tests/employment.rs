#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]
//! S0.10a done gate (TDD §18.2 S0.10, employment part): hourly vs piece-rate
//! payslips, piece-rate on attributed output, pro-rata misses, two contracts,
//! manager-only offers, contract max hours, conservation with 3 firms.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::constitution::Monitoring;
use isms_core::employment::employed;
use isms_core::event::Event;
use isms_core::explain::Num;
use isms_core::ids::{ContractId, OfferId, OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::needs::FULL;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Allocation, ContractStatus, Pay};

/// One firm with a Mine, a manager (citizen 0), `n` other humans fed for a cycle.
fn firm(n: u32, treasury: i64) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n + 1)
        .org(OrgKind::Firm, "Iron & Sons")
        .workplace(WorkplaceKind::Mine, 0, 0);
    for i in 0..=n as usize {
        b = b.pantry(i, Good::Food, 48);
    }
    let mut h = b.build();
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(nth(&h, 0)),
    });
    if treasury > 0 {
        h.apply(Event::Seeded {
            holder: Holder::Org(OrgId(0)),
            asset: Asset::Money(Money::credits(treasury)),
        });
    }
    h
}

fn offer(pay: Pay, places: u32) -> Command {
    Command::OfferEmployment {
        org: OrgId(0),
        workplace: WorkplaceId(0),
        pay,
        max_hours: 8,
        term_cycles: Some(10),
        notice_cycles: 1,
        places,
    }
}

fn work(h: &mut Harness, who: usize, hours: u8) {
    let me = nth(h, who);
    h.cmd(Envelope::citizen(
        me,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: WorkplaceId(0),
                hours,
                effort: Effort::Normal,
            }],
        },
        h.world.meta.tick,
    ))
    .unwrap();
}

fn payslips(events: &[Event]) -> Vec<(u32, Money)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Paid {
                citizen, amount, ..
            } => Some((citizen.0, *amount)),
            _ => None,
        })
        .collect()
}

#[test]
fn offer_accept_and_hourly_payslip() {
    let mut h = firm(1, 1000);
    let (mgr, w) = (nth(&h, 0), nth(&h, 1));
    let r = h.cmd_dry(Envelope::citizen(
        w,
        offer(Pay::Hourly(Money::cents(780)), 1),
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotManager);
    let ev = h
        .cmd(Envelope::citizen(
            mgr,
            offer(Pay::Hourly(Money::cents(780)), 1),
            0,
        ))
        .unwrap();
    assert!(matches!(
        ev[0],
        Event::EmploymentOffered {
            offer: OfferId(0),
            ..
        }
    ));
    let ev = h
        .cmd(Envelope::citizen(
            w,
            Command::AcceptEmployment { offer: OfferId(0) },
            0,
        ))
        .unwrap();
    assert_eq!(
        ev.iter().map(Event::kind).collect::<Vec<_>>(),
        ["EmploymentAccepted", "Assigned"]
    );
    assert!(h.world.offers.is_empty(), "the only place was taken");
    assert!(employed(&h.world, w) && !employed(&h.world, mgr));
    assert_eq!(h.world.orgs[&OrgId(0)].employees.len(), 1);
    // 9 h exceeds the budget, and a contract for 8 h caps at 8 anyway
    let r = h.cmd_dry(Envelope::citizen(
        w,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: WorkplaceId(0),
                hours: 9,
                effort: Effort::Normal,
            }],
        },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::OverContractHours,
        "the contract check comes first"
    );
    work(&mut h, 1, 8);
    h.check_every_step = false;
    let events = h.run_cycle();
    assert_eq!(payslips(&events), vec![(w.0, Money::cents(780 * 8))]);
    let Some(Event::Paid { explain, .. }) = events.iter().find(|e| matches!(e, Event::Paid { .. }))
    else {
        unreachable!()
    };
    assert_eq!(explain.rule, isms_core::explain::RuleId::PayHourly);
    assert_eq!(
        h.citizen(w).household.balance,
        Money::credits(1000) + Money::cents(6240)
    );
    assert_eq!(
        h.world.orgs[&OrgId(0)].treasury,
        Money::credits(1000) - Money::cents(6240)
    );
    h.check();
}

#[test]
fn piece_rate_pays_on_attributed_output_even_when_noisy() {
    for (monitoring, sigma) in [(Monitoring::High, 0.0), (Monitoring::Medium, 0.25)] {
        let mut h = WorldBuilder::new("freeport")
            .with_preset(|p| {
                p.params.population.collapse_enabled = false;
                p.constitution.monitoring = monitoring;
            })
            .humans(2)
            .pantry(1, Good::Food, 48)
            .org(OrgKind::Firm, "x")
            .workplace(WorkplaceKind::Mine, 0, 0)
            .build();
        let (mgr, w) = (nth(&h, 0), nth(&h, 1));
        h.apply(Event::ManagerAppointed {
            org: OrgId(0),
            citizen: Some(mgr),
        });
        h.apply(Event::Seeded {
            holder: Holder::Org(OrgId(0)),
            asset: Asset::Money(Money::credits(1000)),
        });
        h.cmd(Envelope::citizen(
            mgr,
            offer(Pay::PieceRate(Money::cents(85)), 1),
            0,
        ))
        .unwrap();
        h.cmd(Envelope::citizen(
            w,
            Command::AcceptEmployment { offer: OfferId(0) },
            0,
        ))
        .unwrap();
        work(&mut h, 1, 8);
        h.check_every_step = false;
        let events = h.run_cycle();
        let (true_total, attributed_total) = events.iter().fold((0.0, 0.0), |acc, e| match e {
            Event::Produced { per_worker, .. } => (
                acc.0 + per_worker[0].true_output,
                acc.1 + per_worker[0].attributed_output,
            ),
            _ => acc,
        });
        let paid = payslips(&events)[0].1;
        assert_eq!(
            paid,
            Money((85.0 * attributed_total).floor() as i64),
            "sigma {sigma}"
        );
        if sigma == 0.0 {
            assert!((true_total - attributed_total).abs() < 1e-9);
        } else {
            assert!(
                (true_total - attributed_total).abs() > 1e-6,
                "noise should move the payslip"
            );
        }
        h.check();
    }
}

#[test]
fn hourly_and_piece_rate_differ_on_the_same_production() {
    // Same worker, same hours: hourly pays 62.40; piece-rate 0.85 x ~73 Ore (skill grows).
    let run = |pay: Pay| {
        let mut h = firm(1, 1000);
        let (mgr, w) = (nth(&h, 0), nth(&h, 1));
        h.cmd(Envelope::citizen(mgr, offer(pay, 1), 0)).unwrap();
        h.cmd(Envelope::citizen(
            w,
            Command::AcceptEmployment { offer: OfferId(0) },
            0,
        ))
        .unwrap();
        work(&mut h, 1, 8);
        h.check_every_step = false;
        let events = h.run_cycle();
        h.check();
        payslips(&events)[0].1
    };
    let hourly = run(Pay::Hourly(Money::cents(780)));
    let piece = run(Pay::PieceRate(Money::cents(85)));
    assert_eq!(hourly, Money::cents(6240));
    assert!(
        piece > Money::cents(5000) && piece < Money::cents(7000) && piece != hourly,
        "{piece}"
    );
}

#[test]
fn an_empty_treasury_pays_pro_rata_and_ends_the_contracts() {
    let mut h = firm(2, 0);
    let (mgr, a, b) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    h.apply(Event::Seeded {
        holder: Holder::Org(OrgId(0)),
        asset: Asset::Money(Money::cents(6240)),
    });
    h.cmd(Envelope::citizen(
        mgr,
        offer(Pay::Hourly(Money::cents(780)), 2),
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        a,
        Command::AcceptEmployment { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptEmployment { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    work(&mut h, 1, 8);
    work(&mut h, 2, 8);
    h.check_every_step = false;
    let events = h.run_cycle();
    // owed 2 x 62.40 = 124.80 against 62.40: each gets half
    assert_eq!(
        payslips(&events),
        vec![(a.0, Money::cents(3120)), (b.0, Money::cents(3120))]
    );
    let missed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::PaymentMissed { .. }))
        .collect();
    assert_eq!(missed.len(), 2);
    assert!(
        matches!(missed[0], Event::PaymentMissed { owed, paid, .. } if *owed == Money::cents(6240) && *paid == Money::cents(3120))
    );
    let org = &h.world.orgs[&OrgId(0)];
    assert!(org.payment_missed);
    assert_eq!(org.treasury, Money::ZERO);
    assert!(org.employees.is_empty(), "breached contracts ended");
    assert!(
        h.world
            .contracts
            .values()
            .all(|k| k.status == ContractStatus::Ended)
    );
    assert!(
        h.world.workplaces[&WorkplaceId(0)].workers.is_empty(),
        "and the workers were unassigned"
    );
    assert!(!employed(&h.world, a));
    h.check();
}

#[test]
fn a_worker_with_two_contracts_is_paid_by_both() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(2)
        .pantry(1, Good::Food, 48)
        .org(OrgKind::Firm, "A")
        .org(OrgKind::Firm, "B")
        .workplace(WorkplaceKind::Mine, 0, 0)
        .workplace(WorkplaceKind::Farm, 1, 0)
        .build();
    let (mgr, w) = (nth(&h, 0), nth(&h, 1));
    for org in [OrgId(0), OrgId(1)] {
        h.apply(Event::ManagerAppointed {
            org,
            citizen: Some(mgr),
        });
        h.apply(Event::Seeded {
            holder: Holder::Org(org),
            asset: Asset::Money(Money::credits(500)),
        });
    }
    h.cmd(Envelope::citizen(
        mgr,
        Command::OfferEmployment {
            org: OrgId(0),
            workplace: WorkplaceId(0),
            pay: Pay::Hourly(Money::cents(800)),
            max_hours: 4,
            term_cycles: None,
            notice_cycles: 1,
            places: 1,
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        mgr,
        Command::OfferEmployment {
            org: OrgId(1),
            workplace: WorkplaceId(1),
            pay: Pay::Hourly(Money::cents(600)),
            max_hours: 4,
            term_cycles: None,
            notice_cycles: 1,
            places: 1,
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        w,
        Command::AcceptEmployment { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        w,
        Command::AcceptEmployment { offer: OfferId(1) },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        w,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: WorkplaceId(0),
                hours: 5,
                effort: Effort::Normal,
            }],
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::OverContractHours);
    h.cmd(Envelope::citizen(
        w,
        Command::SetLabor {
            allocations: vec![
                Allocation {
                    workplace: WorkplaceId(0),
                    hours: 4,
                    effort: Effort::Normal,
                },
                Allocation {
                    workplace: WorkplaceId(1),
                    hours: 4,
                    effort: Effort::Normal,
                },
            ],
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    let events = h.run_cycle();
    assert_eq!(
        payslips(&events),
        vec![(w.0, Money::cents(3200)), (w.0, Money::cents(2400))]
    );
    h.check();
}

#[test]
fn termination_rules_follow_t18() {
    // Employer terminating: notice wages (1 cycle x 8 h x 7.80) plus accrued; worker leaving forfeits accrued.
    let setup = || {
        let mut h = firm(1, 1000);
        let (mgr, w) = (nth(&h, 0), nth(&h, 1));
        h.cmd(Envelope::citizen(
            mgr,
            offer(Pay::Hourly(Money::cents(780)), 1),
            0,
        ))
        .unwrap();
        h.cmd(Envelope::citizen(
            w,
            Command::AcceptEmployment { offer: OfferId(0) },
            0,
        ))
        .unwrap();
        work(&mut h, 1, 8);
        h.check_every_step = false;
        for _ in 0..6 {
            h.tick();
        }
        (h, mgr, w)
    };
    let (mut h, mgr, w) = setup();
    let accrued = Money::cents(780 * 8 * 6 / 24);
    let events = h
        .cmd(Envelope::citizen(
            mgr,
            Command::TerminateEmployment {
                contract: ContractId(0),
            },
            6,
        ))
        .unwrap();
    assert!(
        matches!(events[0], Event::EmploymentTerminated { notice_pay, forfeited, .. } if notice_pay == Money::cents(6240) + accrued && forfeited == Money::ZERO)
    );
    assert_eq!(
        h.citizen(w).household.balance,
        Money::credits(1000) + Money::cents(6240) + accrued
    );
    assert!(h.world.workplaces[&WorkplaceId(0)].workers.is_empty());
    assert!(!employed(&h.world, w));
    let (mut h2, _, w2) = setup();
    let events = h2
        .cmd(Envelope::citizen(
            w2,
            Command::TerminateEmployment {
                contract: ContractId(0),
            },
            6,
        ))
        .unwrap();
    assert!(
        matches!(events[0], Event::EmploymentTerminated { notice_pay, forfeited, .. } if notice_pay == Money::ZERO && forfeited == accrued)
    );
    assert_eq!(h2.citizen(w2).household.balance, Money::credits(1000));
    let r = h2.cmd_dry(Envelope::citizen(
        w2,
        Command::TerminateEmployment {
            contract: ContractId(0),
        },
        6,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::UnknownContract,
        "already ended"
    );
    // payday after termination pays nothing further
    let events = h2.run_cycle();
    assert!(payslips(&events).is_empty());
    h.check();
    h2.check();
}

#[test]
fn a_term_contract_ends_at_its_last_cycle_and_a_destitute_cannot_sign_long_ones() {
    let mut h = firm(1, 1000);
    let (mgr, w) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(
        mgr,
        Command::OfferEmployment {
            org: OrgId(0),
            workplace: WorkplaceId(0),
            pay: Pay::Hourly(Money::cents(100)),
            max_hours: 2,
            term_cycles: Some(2),
            notice_cycles: 1,
            places: 3,
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        w,
        Command::AcceptEmployment { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    h.run_cycle();
    assert!(employed(&h.world, w));
    h.run_cycle();
    assert!(
        !employed(&h.world, w),
        "term of 2 cycles ended after cycle 1"
    );
    assert!(h.world.workplaces[&WorkplaceId(0)].workers.is_empty());
    // destitute: long contracts refused, short ones allowed
    h.apply(Event::DestitutionBegan {
        citizen: w,
        cycle: 1,
    });
    let r = h.cmd_dry(Envelope::citizen(
        w,
        Command::AcceptEmployment { offer: OfferId(0) },
        48,
    ));
    assert!(r.is_ok(), "term 2 is short");
    h.cmd(Envelope::citizen(
        mgr,
        Command::OfferEmployment {
            org: OrgId(0),
            workplace: WorkplaceId(0),
            pay: Pay::Hourly(Money::cents(100)),
            max_hours: 2,
            term_cycles: Some(6),
            notice_cycles: 1,
            places: 1,
        },
        48,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        w,
        Command::AcceptEmployment { offer: OfferId(1) },
        48,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::OptionsNarrowed);
    h.check();
}

#[test]
fn three_firms_and_twelve_workers_conserve_over_a_cycle() {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(13)
        .org(OrgKind::Firm, "Farm Co")
        .org(OrgKind::Firm, "Mine Co")
        .org(OrgKind::Firm, "Mill Co")
        .workplace(WorkplaceKind::Farm, 0, 2)
        .workplace(WorkplaceKind::Mine, 1, 1)
        .workplace(WorkplaceKind::Mill, 2, 0)
        .org_inventory(2, Good::Grain, 500);
    for i in 0..13 {
        b = b.pantry(i, Good::Food, 48);
    }
    let mut h = b.build();
    let mgr = nth(&h, 0);
    for org in 0..3u32 {
        h.apply(Event::ManagerAppointed {
            org: OrgId(org),
            citizen: Some(mgr),
        });
        h.apply(Event::Seeded {
            holder: Holder::Org(OrgId(org)),
            asset: Asset::Money(Money::credits(300)),
        });
        let pay = if org == 1 {
            Pay::PieceRate(Money::cents(85))
        } else {
            Pay::Hourly(Money::cents(800))
        };
        h.cmd(Envelope::citizen(
            mgr,
            Command::OfferEmployment {
                org: OrgId(org),
                workplace: WorkplaceId(org),
                pay,
                max_hours: 8,
                term_cycles: None,
                notice_cycles: 1,
                places: 4,
            },
            0,
        ))
        .unwrap();
    }
    for i in 1..13usize {
        let me = nth(&h, i);
        let offer = OfferId(u32::try_from((i - 1) % 3).unwrap());
        h.cmd(Envelope::citizen(
            me,
            Command::AcceptEmployment { offer },
            0,
        ))
        .unwrap();
        let wp = WorkplaceId(u32::try_from((i - 1) % 3).unwrap());
        h.cmd(Envelope::citizen(
            me,
            Command::SetLabor {
                allocations: vec![Allocation {
                    workplace: wp,
                    hours: 8,
                    effort: Effort::Normal,
                }],
            },
            0,
        ))
        .unwrap();
        let mut needs = h.citizen(me).needs.clone();
        needs.food = FULL;
        h.set_needs(me, needs);
    }
    h.check_every_step = false;
    let events = h.run_cycle();
    let slips = payslips(&events);
    assert_eq!(slips.len(), 12);
    // Farm Co and Mill Co owe 4 x 64 = 256 <= 300; every hourly slip is 64.00
    assert!(
        slips
            .iter()
            .filter(|(_, m)| *m == Money::cents(6400))
            .count()
            >= 8
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PaymentMissed { .. }))
    );
    h.check();
    let _ = Party::Citizen(mgr);
}

/// D8: a payslip's `hours` is the day's tick-hours over the hours in a day,
/// and the Explain says so, so a worker who changed their allocation mid-day
/// can see where 2.625 came from.
#[test]
fn a_payslip_names_its_tick_hours_when_the_allocation_changed_mid_day() {
    let mut h = firm(1, 1000);
    let (mgr, w) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(
        mgr,
        offer(Pay::Hourly(Money::cents(100)), 1),
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        w,
        Command::AcceptEmployment { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    let tpc = h.world.params.time.ticks_per_cycle;
    work(&mut h, 1, 2);
    h.check_every_step = false;
    for _ in 0..tpc / 2 {
        h.tick();
    }
    work(&mut h, 1, 4);
    let mut events = Vec::new();
    for _ in 0..tpc / 2 {
        events.extend(h.tick());
    }
    let Some(Event::Paid {
        amount, explain, ..
    }) = events.iter().find(|e| matches!(e, Event::Paid { .. }))
    else {
        panic!("no payslip in {:?}", events.iter().map(Event::kind).collect::<Vec<_>>())
    };
    let input = |name: &str| {
        explain
            .inputs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("no input {name} in {:?}", explain.inputs))
    };
    let half = tpc / 2;
    assert_eq!(explain.formula, "tick_hours / ticks_per_cycle x rate");
    assert_eq!(input("tick_hours"), Num::Int(i64::from(2 * half + 4 * half)));
    assert_eq!(input("ticks_per_cycle"), Num::Int(i64::from(tpc)));
    assert_eq!(input("hours"), Num::Float(3.0), "half a day at 2 h, half at 4 h");
    assert_eq!(input("rate"), Num::Money(Money::cents(100)));
    assert_eq!(*amount, Money::cents(300));
    h.check();
}
