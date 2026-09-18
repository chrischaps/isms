#![allow(clippy::many_single_char_names, clippy::too_many_lines)]
//! S0.11b done gate (TDD §18.2 S0.11, credit and associations): the hand-computed
//! schedule, conservation over a loan's life, default seizes collateral then
//! flags, association membership and pantry access.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::credit::schedule;
use isms_core::event::Event;
use isms_core::ids::{ContractId, DwellingId, OfferId, OrgId};
use isms_core::kinds::{Good, OrgKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Collateral, ContractStatus, Owner, Price, SaleAsset};

fn quiet(humans: u32) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000;
        })
        .humans(humans)
        .householders(1)
        .org(OrgKind::Firm, "Legacy Builders")
        .dwelling(0);
    for i in 0..humans as usize {
        b = b.pantry(i, Good::Food, 40);
    }
    let mut h = b.build();
    for who in h.citizen_ids() {
        let mut p = h.citizen(who).plan.clone();
        p.keep_food_at_least = 0;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(p),
        });
    }
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(nth(&h, humans as usize)),
    });
    h
}

fn loan(
    to: Option<Party>,
    credits: i64,
    bp: u32,
    term: u32,
    collateral: Option<Collateral>,
) -> Command {
    Command::OfferCredit {
        to,
        principal: Money::credits(credits),
        rate_per_cycle_bp: bp,
        term_cycles: term,
        collateral,
    }
}

#[test]
fn the_schedule_is_hand_computable() {
    // 100 credits, 5 cycles, 2% per cycle: interest 10, 22.00 per cycle
    let (total, inst, last) = schedule(Money::credits(100), 200, 5);
    assert_eq!(
        (total, inst, last),
        (Money::credits(110), Money::credits(22), Money::credits(22))
    );
    // remainder lands on the last installment
    let (total, inst, last) = schedule(Money::cents(1000), 100, 3);
    assert_eq!(total, Money::cents(1030));
    assert_eq!(inst, Money::cents(343));
    assert_eq!(last, Money::cents(344));
}

#[test]
fn a_loan_is_escrowed_repaid_in_installments_and_conserves() {
    let mut h = quiet(2);
    let (l, b) = (nth(&h, 0), nth(&h, 1));
    let r = h.cmd_dry(Envelope::citizen(
        l,
        loan(Some(Party::Citizen(l)), 100, 200, 5, None),
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::SelfDeal);
    let r = h.cmd_dry(Envelope::citizen(l, loan(None, 2000, 200, 5, None), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientFunds);
    h.cmd(Envelope::citizen(
        l,
        loan(Some(Party::Citizen(b)), 100, 200, 5, None),
        0,
    ))
    .unwrap();
    assert_eq!(
        h.citizen(l).household.balance,
        Money::credits(900),
        "principal escrowed"
    );
    assert_eq!(h.world.escrow.len(), 1);
    let ev = h
        .cmd(Envelope::citizen(
            b,
            Command::AcceptCredit { offer: OfferId(0) },
            0,
        ))
        .unwrap();
    assert!(
        matches!(ev[0], Event::CreditAccepted { installment, installments: 5, .. } if installment == Money::credits(22))
    );
    assert_eq!(h.citizen(b).household.balance, Money::credits(1100));
    assert!(h.world.escrow.is_empty() && h.world.offers.is_empty());
    h.check_every_step = false;
    for cycle in 0..5u32 {
        let events = h.run_cycle();
        let inst: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::CreditInstallment { .. }))
            .collect();
        assert_eq!(inst.len(), 1, "cycle {cycle}");
        assert!(
            matches!(inst[0], Event::CreditInstallment { amount, remaining, .. } if *amount == Money::credits(22) && *remaining == 4 - cycle)
        );
        let repaid = events
            .iter()
            .any(|e| matches!(e, Event::CreditRepaid { .. }));
        assert_eq!(repaid, cycle == 4, "cycle {cycle}");
    }
    assert_eq!(h.citizen(l).household.balance, Money::credits(1010));
    assert_eq!(h.citizen(b).household.balance, Money::credits(990));
    assert_eq!(
        h.world.contracts[&ContractId(0)].status,
        ContractStatus::Ended
    );
    let events = h.run_cycle();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::CreditInstallment { .. })),
        "nothing after repayment"
    );
    h.check();
}

#[test]
fn a_missed_installment_defaults_on_the_next_first_tick_and_seizes_the_dwelling() {
    let mut h = quiet(2);
    let (l, b, mgr) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    // b buys the dwelling, then borrows against it
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::OfferSale {
                asset: SaleAsset::Dwelling(DwellingId(0)),
                price: Price::Money(Money::credits(100)),
                to: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        l,
        loan(
            Some(Party::Citizen(b)),
            500,
            100,
            4,
            Some(Collateral::Dwelling(DwellingId(0))),
        ),
        0,
    ));
    assert!(r.is_ok());
    h.cmd(Envelope::citizen(
        l,
        loan(None, 500, 100, 4, Some(Collateral::Dwelling(DwellingId(0)))),
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        l,
        Command::AcceptCredit { offer: OfferId(1) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::SelfDeal);
    let stranger = nth(&h, 2);
    let r = h.cmd_dry(Envelope::citizen(
        stranger,
        Command::AcceptCredit { offer: OfferId(1) },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::NotOwner,
        "collateral must be the borrower's"
    );
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptCredit { offer: OfferId(1) },
        0,
    ))
    .unwrap();
    // a pledged dwelling cannot be sold
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::OfferSale {
            asset: SaleAsset::Dwelling(DwellingId(0)),
            price: Price::Money(Money::credits(1)),
            to: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::AlreadyExists);
    // b gives everything away, so the first installment is missed
    h.cmd(Envelope::citizen(
        b,
        Command::Transfer {
            to: Party::Citizen(l),
            asset: Asset::Money(h.citizen(b).household.balance),
            memo: String::new(),
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    let c0 = h.run_cycle();
    assert!(c0.iter().any(|e| matches!(e, Event::CreditMissed { .. })));
    assert!(
        !c0.iter()
            .any(|e| matches!(e, Event::CreditDefaulted { .. })),
        "the default resolves next tick, not at payday"
    );
    let first_tick = h.tick();
    assert!(first_tick.iter().any(|e| matches!(
        e,
        Event::CreditDefaulted {
            collateral_seized: Some(Collateral::Dwelling(DwellingId(0))),
            ..
        }
    )));
    assert_eq!(h.world.dwellings[&DwellingId(0)].owner, Owner::Citizen(l));
    assert!(h.citizen(b).flags.defaulted);
    assert_eq!(
        h.world.contracts[&ContractId(0)].status,
        ContractStatus::Ended
    );
    h.check();
}

#[test]
fn shares_as_collateral_are_escrowed_and_returned_on_repayment() {
    let mut h = quiet(2);
    let (l, b, mgr) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::OfferSale {
                asset: SaleAsset::Shares(OrgId(0), 100),
                price: Price::Money(Money::credits(100)),
                to: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        l,
        loan(None, 50, 0, 1, Some(Collateral::Shares(OrgId(0), 40))),
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptCredit { offer: OfferId(1) },
        0,
    ))
    .unwrap();
    assert_eq!(
        isms_core::shares::shares_held(&h.world, Party::Citizen(b), OrgId(0)),
        60,
        "40 in escrow"
    );
    assert_eq!(h.world.share_escrow.len(), 1);
    h.check_every_step = false;
    let c0 = h.run_cycle();
    assert!(c0.iter().any(|e| matches!(e, Event::CreditRepaid { .. })));
    assert_eq!(
        isms_core::shares::shares_held(&h.world, Party::Citizen(b), OrgId(0)),
        100
    );
    assert!(h.world.share_escrow.is_empty());
    h.check();
}

#[test]
fn associations_admit_members_who_can_fund_the_pantry() {
    let mut h = quiet(3);
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    h.cmd(Envelope::citizen(
        a,
        Command::FoundOrg {
            kind: OrgKind::Association,
            name: "Mutual Aid".into(),
            first_workplace: None,
        },
        0,
    ))
    .unwrap();
    let org = OrgId(1);
    assert!(h.world.orgs[&org].members.contains(&a));
    assert_eq!(h.world.orgs[&org].manager, Some(a));
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::AdmitMember { org, citizen: c },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotManager);
    h.cmd(Envelope::citizen(b, Command::RequestMembership { org }, 0))
        .unwrap();
    assert_eq!(h.world.offers.len(), 1);
    let r = h.cmd_dry(Envelope::citizen(b, Command::RequestMembership { org }, 0));
    assert_eq!(r.unwrap_err().code, RejectCode::AlreadyExists);
    h.cmd(Envelope::citizen(
        a,
        Command::AdmitMember { org, citizen: b },
        0,
    ))
    .unwrap();
    assert!(h.world.orgs[&org].members.contains(&b));
    assert!(h.world.offers.is_empty(), "the request was consumed");
    // a member funds the pantry; only the manager disburses
    h.cmd(Envelope::citizen(
        b,
        Command::Transfer {
            to: Party::Org(org),
            asset: Asset::Good(Good::Food, 10),
            memo: "dues".into(),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.orgs[&org].inventory[&Good::Food], 10);
    let r = h.cmd_dry(
        Envelope::citizen(
            b,
            Command::Transfer {
                to: Party::Citizen(c),
                asset: Asset::Good(Good::Food, 1),
                memo: String::new(),
            },
            0,
        )
        .on_behalf_of(org),
    );
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::NotManager,
        "a non-manager cannot draw"
    );
    h.cmd(
        Envelope::citizen(
            a,
            Command::Transfer {
                to: Party::Citizen(c),
                asset: Asset::Good(Good::Food, 4),
                memo: "alms".into(),
            },
            0,
        )
        .on_behalf_of(org),
    )
    .unwrap();
    assert_eq!(h.world.orgs[&org].inventory[&Good::Food], 6);
    // the manager leaving vacates the chair
    let ev = h
        .cmd(Envelope::citizen(a, Command::LeaveOrg { org }, 0))
        .unwrap();
    assert_eq!(
        ev.iter().map(Event::kind).collect::<Vec<_>>(),
        ["MemberLeft", "ManagerAppointed"]
    );
    assert_eq!(h.world.orgs[&org].manager, None);
    let _ = Holder::Org(org);
    h.check();
}

/// Q109 (D9): a withdrawn loan offer returns the escrowed principal to the lender.
#[test]
fn a_withdrawn_loan_offer_returns_the_principal() {
    let mut h = quiet(2);
    let (lender, borrower) = (nth(&h, 0), nth(&h, 1));
    let before = h.citizen(lender).household.balance;
    let ev = h
        .cmd(Envelope::citizen(lender, loan(None, 100, 100, 4, None), 0))
        .unwrap();
    let Event::CreditOffered { offer, .. } = &ev[0] else {
        panic!("{:?}", ev[0].kind())
    };
    let offer = *offer;
    assert_eq!(
        h.citizen(lender).household.balance,
        before - Money::credits(100)
    );
    assert_eq!(h.world.escrow.len(), 1);
    let r = h.cmd_dry(Envelope::citizen(
        borrower,
        Command::WithdrawOffer { offer },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotParty);
    let ev = h
        .cmd(Envelope::citizen(
            lender,
            Command::WithdrawOffer { offer },
            0,
        ))
        .unwrap();
    assert_eq!(
        ev.iter().map(Event::kind).collect::<Vec<_>>(),
        ["OfferWithdrawn"]
    );
    assert_eq!(
        h.citizen(lender).household.balance,
        before,
        "the principal came back"
    );
    assert!(h.world.escrow.is_empty());
    assert!(h.world.offers.is_empty());
    let r = h.cmd_dry(Envelope::citizen(
        borrower,
        Command::AcceptCredit { offer },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::UnknownOffer);
    h.check();
}
