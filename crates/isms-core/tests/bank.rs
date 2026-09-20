#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
//! S0.17c done gate (the Public Investment Bank): the levy is machines times
//! the last price times the rate and conserves; the floor is paid from the
//! pool before lending; the bank lends in application order and never more
//! than it holds; a coop with an open loan is declined; installments flow back
//! to the pool and reduce the share-out; an admission proposal passes at a
//! majority and lapses at cycle end; peer credit is not in this society.

use isms_core::bank::bank_org;
use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::ids::{OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::Ownership;

/// `coops` cooperatives, each with a Mine holding `machines`, managed by
/// citizens 0..coops (first members); `extra` more humans; the bank founded.
fn society(coops: u32, machines: u32, extra: u32) -> Harness {
    let mut b = WorldBuilder::new("commonwealth")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(coops + extra);
    for i in 0..coops {
        b = b
            .org(OrgKind::Cooperative, &format!("Co-op {i}"))
            .workplace(WorkplaceKind::Mine, i as usize, machines);
    }
    let mut h = b.build();
    h.check_every_step = false;
    for i in 0..coops {
        let m = nth(&h, i as usize);
        h.apply(Event::ManagerAppointed {
            org: OrgId(i),
            citizen: Some(m),
        });
        h.apply(Event::MemberAdmitted {
            org: OrgId(i),
            citizen: m,
        });
        h.apply(Event::Assigned {
            workplace: WorkplaceId(i),
            citizen: m,
            contract: None,
        });
    }
    let bank = h.world.next.org;
    h.apply(Event::OrgFounded {
        org: bank,
        kind: OrgKind::Association,
        name: "Public Investment Bank".into(),
        founder: None,
        ownership: Ownership::Society,
        manager: None,
        fee_burned: Money::ZERO,
    });
    for i in 0..(coops + extra) as usize {
        let c = nth(&h, i);
        let mut plan = h.citizen(c).plan.clone();
        plan.keep_food_at_least = 0;
        plan.buy_wares_when = None;
        h.apply(Event::PlanChanged {
            citizen: c,
            plan: Box::new(plan),
        });
        h.apply(Event::Seeded {
            holder: Holder::Citizen(c),
            asset: Asset::Good(Good::Food, 48),
        });
    }
    h
}

fn pool(h: &Harness) -> Money {
    h.world.orgs[&bank_org(&h.world).unwrap()].treasury
}

fn seed_pool(h: &mut Harness, credits: i64) {
    let bank = bank_org(&h.world).unwrap();
    h.apply(Event::Seeded {
        holder: Holder::Org(bank),
        asset: Asset::Money(Money::credits(credits)),
    });
}

fn apply_for(h: &mut Harness, coop: u32, credits: i64) -> Result<(), RejectCode> {
    let m = nth(h, coop as usize);
    h.cmd(
        Envelope::citizen(
            m,
            Command::RequestBankLoan {
                org: OrgId(coop),
                principal: Money::credits(credits),
                term_cycles: 5,
            },
            h.world.meta.tick,
        )
        .on_behalf_of(OrgId(coop)),
    )
    .map(|_| ())
    .map_err(|e| e.code)
}

#[test]
fn levy_is_machines_times_last_price_times_rate_and_conserves() {
    let mut h = society(1, 4, 0);
    h.apply(Event::Seeded {
        holder: Holder::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(100)),
    });
    let price = h.world.params.money.start_prices[&Good::Machines];
    let rate = h.world.policy.capital_levy.unwrap();
    let expected = Money(((price.0 * 4) as f64 * rate).floor() as i64);
    assert_eq!(isms_core::bank::levy_due(&h.world, OrgId(0)), expected);
    let events = h.run_cycle();
    let levy = events.iter().find_map(|e| match e {
        Event::LevyPaid {
            org: OrgId(0),
            amount,
            explain,
            ..
        } => {
            assert_eq!(explain.rule, RuleId::CapitalLevy);
            Some(*amount)
        }
        _ => None,
    });
    assert_eq!(levy, Some(expected));
    assert_eq!(pool(&h), expected);
    assert_eq!(
        h.world.orgs[&OrgId(0)].treasury,
        Money::credits(100) - expected
    );
    h.check();
    // No machines, no levy.
    let mut none = society(1, 0, 0);
    assert_eq!(
        isms_core::bank::levy_due(&none.world, OrgId(0)),
        Money::ZERO
    );
    assert!(
        !none
            .run_cycle()
            .iter()
            .any(|e| matches!(e, Event::LevyPaid { .. }))
    );
}

#[test]
fn floor_is_paid_from_the_pool_before_lending() {
    let mut h = society(1, 0, 1);
    // The second human is broke; the pool holds 10; the coop applies for 10.
    let broke = nth(&h, 1);
    let have = h.citizen(broke).household.balance;
    h.apply(Event::Transferred {
        from: Party::Citizen(broke),
        to: Party::Citizen(nth(&h, 0)),
        asset: Asset::Money(have),
        memo: String::new(),
    });
    h.apply(Event::Transferred {
        from: Party::Citizen(broke),
        to: Party::Citizen(nth(&h, 0)),
        asset: Asset::Good(Good::Food, 48),
        memo: String::new(),
    });
    seed_pool(&mut h, 10);
    apply_for(&mut h, 0, 10).unwrap();
    let events = h.run_cycle();
    let floor = events.iter().find_map(|e| match e {
        Event::NeedFloorPaid {
            citizen,
            amount,
            from,
            ..
        } if *citizen == broke => Some((*amount, *from)),
        _ => None,
    });
    let bank = bank_org(&h.world).unwrap();
    assert_eq!(floor, Some((Money::credits(10), Holder::Org(bank))));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::BankLoanDecided { .. })),
        "the application waits: the floor emptied the pool"
    );
    assert!(isms_core::bank::pending_application(&h.world, OrgId(0)));
    h.check();
}

#[test]
fn bank_lends_in_application_order_and_never_more_than_it_holds() {
    let mut h = society(3, 0, 0);
    seed_pool(&mut h, 25);
    apply_for(&mut h, 2, 10).unwrap();
    apply_for(&mut h, 0, 10).unwrap();
    apply_for(&mut h, 1, 10).unwrap();
    let events = h.run_cycle();
    let granted: Vec<OrgId> = events
        .iter()
        .filter_map(|e| match e {
            Event::BankLoanDecided {
                org, granted: true, ..
            } => Some(*org),
            _ => None,
        })
        .collect();
    assert_eq!(granted, vec![OrgId(2), OrgId(0)], "first come first served");
    assert!(isms_core::bank::pending_application(&h.world, OrgId(1)));
    assert!(pool(&h) >= Money::ZERO);
    let bank = bank_org(&h.world).unwrap();
    assert!(isms_core::bank::open_bank_loan(&h.world, bank, OrgId(2)));
    assert!(isms_core::bank::open_bank_loan(&h.world, bank, OrgId(0)));
    // The first installment falls at 8c of the same cycle end (Q94).
    let rate = h.world.params.bank.rate_per_cycle_bp;
    let (_, first, _) = isms_core::credit::schedule(Money::credits(10), rate, 5);
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::credits(10) - first);
    assert_eq!(
        h.world.orgs[&OrgId(0)].surplus_base,
        Money::credits(10) - first,
        "principal is capital, the installment paid is not"
    );
    assert_eq!(pool(&h), Money::credits(5) + first + first);
    // Every event kept the bank's treasury at or above zero.
    let mut w = isms_core::test_support::fold(&h.log[..h.log.len() - events.len()]);
    for e in &events {
        isms_core::apply(&mut w, e);
        assert!(w.orgs[&bank].treasury >= Money::ZERO, "{}", e.kind());
    }
    h.check();
}

#[test]
fn a_coop_with_an_open_bank_loan_is_declined_and_limits_hold() {
    let mut h = society(1, 0, 0);
    seed_pool(&mut h, 100);
    apply_for(&mut h, 0, 10).unwrap();
    h.run_cycle();
    // Already borrowing: a second application is rejected outright.
    assert_eq!(apply_for(&mut h, 0, 10), Err(RejectCode::AlreadyExists));
    let max = h.world.params.bank.max_loan;
    let mut fresh = society(1, 0, 0);
    seed_pool(&mut fresh, 100);
    assert_eq!(
        apply_for(&mut fresh, 0, max.as_credits_f64() as i64 + 1),
        Err(RejectCode::InvalidQuantity)
    );
    let m = nth(&fresh, 0);
    let err = fresh
        .cmd(
            Envelope::citizen(
                m,
                Command::RequestBankLoan {
                    org: OrgId(0),
                    principal: Money::credits(5),
                    term_cycles: fresh.world.params.bank.max_term_cycles + 1,
                },
                0,
            )
            .on_behalf_of(OrgId(0)),
        )
        .unwrap_err();
    assert_eq!(err.code, RejectCode::InvalidTerm);
}

#[test]
fn installments_flow_back_to_the_pool_and_reduce_share_out() {
    let mut h = society(1, 0, 0);
    seed_pool(&mut h, 20);
    apply_for(&mut h, 0, 20).unwrap();
    h.cmd(Envelope::citizen(
        nth(&h, 0),
        Command::SetLabor {
            allocations: vec![isms_core::world::Allocation {
                workplace: WorkplaceId(0),
                hours: 8,
                effort: Effort::Normal,
            }],
        },
        0,
    ))
    .unwrap();
    h.run_cycle();
    let rate = h.world.params.bank.rate_per_cycle_bp;
    let (_, installment, _) = isms_core::credit::schedule(Money::credits(20), rate, 5);
    assert_eq!(
        pool(&h),
        installment,
        "the loan went out and the first installment came back"
    );
    // Revenue of 30 next cycle: the installment is held back from the share.
    h.apply(Event::Transferred {
        from: Party::Citizen(nth(&h, 0)),
        to: Party::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(30)),
        memo: "sale".into(),
    });
    let events = h.run_cycle();
    let surplus = events.iter().find_map(|e| match e {
        Event::SurplusDeclared { surplus, .. } => Some(*surplus),
        _ => None,
    });
    assert_eq!(surplus, Some(Money::credits(30) - installment));
    assert_eq!(
        pool(&h),
        installment + installment,
        "the second installment came back to the pool"
    );
    h.check();
}

#[test]
fn admission_proposal_passes_at_majority_and_lapses_at_cycle_end() {
    let mut h = society(1, 0, 4);
    // Members: 0 (manager), 1, 2 admitted by the manager; 3 is the candidate.
    for i in 1..=2 {
        h.cmd(Envelope::citizen(
            nth(&h, 0),
            Command::AdmitMember {
                org: OrgId(0),
                citizen: nth(&h, i),
            },
            0,
        ))
        .unwrap();
    }
    let cand = nth(&h, 3);
    let events = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::ProposeAdmission {
                org: OrgId(0),
                citizen: cand,
            },
            0,
        ))
        .unwrap();
    let Event::AdmissionProposed { proposal, .. } = events[0] else {
        panic!()
    };
    assert_eq!(events.len(), 1, "one yes of three is no majority");
    // A ballot is replaceable until the close (S2.1): the proposer casting
    // yes again changes nothing; a non-member cannot vote.
    let events = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::VoteAdmission {
                proposal,
                approve: true,
            },
            0,
        ))
        .unwrap();
    assert_eq!(events.len(), 1, "still one yes of three");
    assert_eq!(h.world.proposals[&proposal].ballots.len(), 1);
    let err = h
        .cmd(Envelope::citizen(
            cand,
            Command::VoteAdmission {
                proposal,
                approve: true,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotParty);
    let events = h
        .cmd(Envelope::citizen(
            nth(&h, 2),
            Command::VoteAdmission {
                proposal,
                approve: true,
            },
            0,
        ))
        .unwrap();
    assert!(matches!(
        events[1],
        Event::ProposalClosed { passed: true, .. }
    ));
    assert!(h.world.orgs[&OrgId(0)].members.contains(&cand));
    assert!(isms_core::labor::has_position(&h.world, cand));
    assert!(h.world.proposals.is_empty());
    // A second proposal left open lapses at cycle end.
    let cand2 = nth(&h, 4);
    h.cmd(Envelope::citizen(
        nth(&h, 0),
        Command::ProposeAdmission {
            org: OrgId(0),
            citizen: cand2,
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.proposals.len(), 1);
    let events = h.run_cycle();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ProposalClosed { passed: false, .. }))
    );
    assert!(h.world.proposals.is_empty());
    assert!(!h.world.orgs[&OrgId(0)].members.contains(&cand2));
    // A lone member's proposal is its own majority.
    let mut solo = society(1, 0, 1);
    let events = solo
        .cmd(Envelope::citizen(
            nth(&solo, 0),
            Command::ProposeAdmission {
                org: OrgId(0),
                citizen: nth(&solo, 1),
            },
            0,
        ))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::MemberAdmitted { .. }))
    );
}

#[test]
fn peer_credit_is_not_in_this_society() {
    let mut h = society(1, 0, 1);
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::OfferCredit {
                to: None,
                principal: Money::credits(10),
                rate_per_cycle_bp: 100,
                term_cycles: 2,
                collateral: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    // And the bank does not exist in Freeport.
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .org(OrgKind::Firm, "F")
        .build();
    f.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(nth(&f, 0)),
    });
    let err = f
        .cmd(
            Envelope::citizen(
                nth(&f, 0),
                Command::RequestBankLoan {
                    org: OrgId(0),
                    principal: Money::credits(10),
                    term_cycles: 2,
                },
                0,
            )
            .on_behalf_of(OrgId(0)),
        )
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
}
