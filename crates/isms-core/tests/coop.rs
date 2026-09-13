#![allow(clippy::many_single_char_names)]
//! S0.17b done gate (cooperatives): the share-out sums to the surplus under
//! both rules; seed money and loan principal are not surplus; installments
//! due are reserved; admission places the member and clears their other
//! requests; leaving forfeits; a coop cannot employ; the System can appoint a
//! coop's manager; forty householders run three Commonwealth cycles with no
//! rejection and everyone a member.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::householder::run_round;
use isms_core::ids::{CitizenId, OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::labor::has_position;
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Pay, ShareRule};

/// A coop with one Mine; citizen 0 manages and is the first member; `n`
/// more humans exist unplaced. Meters quiet, pantries stocked.
fn coop(n: u32) -> Harness {
    let mut b = WorldBuilder::new("commonwealth")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n + 1)
        .org(OrgKind::Cooperative, "Miners' Co-op")
        .workplace(WorkplaceKind::Mine, 0, 0);
    for i in 0..=n as usize {
        b = b.pantry(i, Good::Food, 48);
    }
    let mut h = b.build();
    h.check_every_step = false;
    let m = nth(&h, 0);
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(m),
    });
    h.apply(Event::MemberAdmitted {
        org: OrgId(0),
        citizen: m,
    });
    h.apply(Event::Assigned {
        workplace: WorkplaceId(0),
        citizen: m,
        contract: None,
    });
    for i in 0..=n as usize {
        let c = nth(&h, i);
        let mut plan = h.citizen(c).plan.clone();
        plan.keep_food_at_least = 0;
        plan.buy_wares_when = None;
        h.apply(Event::PlanChanged {
            citizen: c,
            plan: Box::new(plan),
        });
    }
    h
}

fn admit(h: &mut Harness, who: usize) {
    let (m, c) = (nth(h, 0), nth(h, who));
    h.cmd(Envelope::citizen(
        m,
        Command::AdmitMember {
            org: OrgId(0),
            citizen: c,
        },
        0,
    ))
    .unwrap();
}

fn work(h: &mut Harness, who: usize, hours: u8) {
    let c = nth(h, who);
    h.cmd(Envelope::citizen(
        c,
        Command::SetLabor {
            allocations: vec![isms_core::world::Allocation {
                workplace: WorkplaceId(0),
                hours,
                effort: Effort::Normal,
            }],
        },
        0,
    ))
    .unwrap();
}

fn shares(events: &[Event]) -> Vec<(CitizenId, Money)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ShareOutPaid {
                citizen,
                amount,
                explain,
                ..
            } => {
                assert_eq!(explain.rule, RuleId::PayShare);
                Some((*citizen, *amount))
            }
            _ => None,
        })
        .collect()
}

fn declared(events: &[Event]) -> Option<Money> {
    events.iter().find_map(|e| match e {
        Event::SurplusDeclared { surplus, .. } => Some(*surplus),
        _ => None,
    })
}

/// Three members working 8, 4 and 0 hours; revenue arrives as a transfer
/// mid-cycle so the surplus is exactly that revenue.
fn three_members(rule: ShareRule) -> (Harness, Vec<Event>) {
    let mut h = coop(2);
    admit(&mut h, 1);
    admit(&mut h, 2);
    work(&mut h, 0, 8);
    work(&mut h, 1, 4);
    h.cmd(
        Envelope::citizen(
            nth(&h, 0),
            Command::SetShareRule {
                org: OrgId(0),
                rule,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    // Revenue: 100.01 credits arrives from a customer (citizen 2's own money).
    h.apply(Event::Transferred {
        from: Party::Citizen(nth(&h, 2)),
        to: Party::Org(OrgId(0)),
        asset: Asset::Money(Money::cents(10_001)),
        memo: "sale".into(),
    });
    let events = h.run_cycle();
    (h, events)
}

#[test]
fn share_out_sums_to_surplus_equal_and_hours_weighted() {
    let (h, events) = three_members(ShareRule::HoursWeighted);
    assert_eq!(declared(&events), Some(Money::cents(10_001)));
    let s = shares(&events);
    let total: Money = s.iter().map(|x| x.1).sum();
    assert_eq!(
        total,
        Money::cents(10_001),
        "the sum is exactly the surplus"
    );
    // 8 h and 4 h: two thirds and one third; the idle member gets nothing.
    assert_eq!(s.len(), 2);
    assert_eq!(s[0], (nth(&h, 0), Money::cents(6668)));
    assert_eq!(s[1], (nth(&h, 1), Money::cents(3333)));
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::ZERO);
    h.check();

    let (h, events) = three_members(ShareRule::Equal);
    let s = shares(&events);
    let total: Money = s.iter().map(|x| x.1).sum();
    assert_eq!(total, Money::cents(10_001));
    assert_eq!(s.len(), 3, "equal shares pay the idle member too");
    assert_eq!(s[0].1, Money::cents(3334));
    assert_eq!(s[1].1, Money::cents(3334));
    assert_eq!(s[2].1, Money::cents(3333));
    assert!(h.citizen(nth(&h, 0)).wages_total > Money::ZERO);
    assert_eq!(h.citizen(nth(&h, 0)).taxable_income, Money::cents(3334));
    h.check();
}

#[test]
fn seed_money_and_loan_principal_are_not_surplus() {
    let mut h = coop(1);
    admit(&mut h, 1);
    work(&mut h, 0, 8);
    h.apply(Event::Seeded {
        holder: Holder::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(500)),
    });
    let events = h.run_cycle();
    assert_eq!(
        declared(&events),
        Some(Money::ZERO),
        "seed capital is not a surplus"
    );
    assert!(shares(&events).is_empty());
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::credits(500));
    // A customer's money next cycle is: surplus = the sale, not the 500 held.
    h.apply(Event::Transferred {
        from: Party::Citizen(nth(&h, 1)),
        to: Party::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(40)),
        memo: "sale".into(),
    });
    let events = h.run_cycle();
    assert_eq!(declared(&events), Some(Money::credits(40)));
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::credits(500));
    h.check();
}

#[test]
fn installments_due_are_reserved_before_share_out() {
    let mut h = coop(1);
    admit(&mut h, 1);
    work(&mut h, 0, 8);
    // A 100-credit peer loan to the coop (public_credit is gated like credit
    // until S0.17c): 5 installments of 22 at 2 % a cycle.
    let lender = nth(&h, 1);
    let events = h
        .cmd(Envelope::citizen(
            lender,
            Command::OfferCredit {
                to: Some(Party::Org(OrgId(0))),
                principal: Money::credits(100),
                rate_per_cycle_bp: 200,
                term_cycles: 5,
                collateral: None,
            },
            0,
        ))
        .unwrap();
    let Event::CreditOffered { offer, .. } = events[0] else {
        panic!()
    };
    h.cmd(Envelope::citizen(nth(&h, 0), Command::AcceptCredit { offer }, 0).on_behalf_of(OrgId(0)))
        .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::credits(100));
    // Revenue of 30 this cycle: the 22 due is held back, 8 is shared.
    h.apply(Event::Transferred {
        from: Party::Citizen(lender),
        to: Party::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(30)),
        memo: "sale".into(),
    });
    let events = h.run_cycle();
    assert_eq!(declared(&events), Some(Money::credits(8)));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::CreditInstallment { .. })),
        "the installment was then paid at 8c"
    );
    h.check();
}

#[test]
fn admission_places_the_member_and_clears_other_requests() {
    let mut h = WorldBuilder::new("commonwealth")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(3)
        .org(OrgKind::Cooperative, "A")
        .org(OrgKind::Cooperative, "B")
        .workplace(WorkplaceKind::Farm, 0, 0)
        .workplace(WorkplaceKind::Mine, 0, 0)
        .workplace(WorkplaceKind::Mine, 1, 0)
        .build();
    h.check_every_step = false;
    let (m, c) = (nth(&h, 0), nth(&h, 2));
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(m),
    });
    h.apply(Event::ManagerAppointed {
        org: OrgId(1),
        citizen: Some(nth(&h, 1)),
    });
    // Two requests; the Farm is fuller (one worker) than the Mine (none).
    h.apply(Event::Assigned {
        workplace: WorkplaceId(0),
        citizen: m,
        contract: None,
    });
    for org in [OrgId(0), OrgId(1)] {
        h.cmd(Envelope::citizen(c, Command::RequestMembership { org }, 0))
            .unwrap();
    }
    assert_eq!(h.world.offers.len(), 2);
    let events = h
        .cmd(Envelope::citizen(
            m,
            Command::AdmitMember {
                org: OrgId(0),
                citizen: c,
            },
            0,
        ))
        .unwrap();
    assert!(matches!(
        events[1],
        Event::Assigned {
            workplace: WorkplaceId(1),
            ..
        }
    ));
    assert!(h.world.orgs[&OrgId(0)].members.contains(&c));
    assert!(has_position(&h.world, c));
    assert_eq!(h.world.offers.len(), 0, "the request to B lapsed");
    assert!(h.world.orgs[&OrgId(0)].member_since.contains_key(&c));
    // A placed citizen cannot be admitted elsewhere.
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::AdmitMember {
                org: OrgId(1),
                citizen: c,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::AlreadyExists);
    h.check();
}

#[test]
fn leaving_forfeits_this_cycles_share() {
    let mut h = coop(2);
    admit(&mut h, 1);
    admit(&mut h, 2);
    work(&mut h, 0, 8);
    work(&mut h, 1, 8);
    h.apply(Event::Transferred {
        from: Party::Citizen(nth(&h, 2)),
        to: Party::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(90)),
        memo: "sale".into(),
    });
    // Member 1 leaves mid-cycle: position dropped, no claim.
    let events = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::LeaveOrg { org: OrgId(0) },
            0,
        ))
        .unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::Unassigned { .. })));
    assert!(!has_position(&h.world, nth(&h, 1)));
    let events = h.run_cycle();
    let s = shares(&events);
    assert!(s.iter().all(|(c, _)| *c != nth(&h, 1)));
    let total: Money = s.iter().map(|x| x.1).sum();
    assert_eq!(total, Money::credits(90));
    h.check();
}

#[test]
fn a_coop_cannot_employ_and_the_system_can_appoint_its_manager() {
    let mut h = coop(1);
    let err = h
        .cmd(
            Envelope::citizen(
                nth(&h, 0),
                Command::OfferEmployment {
                    org: OrgId(0),
                    workplace: WorkplaceId(0),
                    pay: Pay::Hourly(Money::credits(8)),
                    max_hours: 8,
                    term_cycles: None,
                    notice_cycles: 1,
                    places: 1,
                },
                0,
            )
            .on_behalf_of(OrgId(0)),
        )
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    admit(&mut h, 1);
    h.cmd(Envelope::system(
        Command::AppointManager {
            org: OrgId(0),
            citizen: Some(nth(&h, 1)),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].manager, Some(nth(&h, 1)));
    // A non-member citizen cannot set the share rule; the manager can.
    let err = h
        .cmd(
            Envelope::citizen(
                nth(&h, 0),
                Command::SetShareRule {
                    org: OrgId(0),
                    rule: ShareRule::Equal,
                },
                0,
            )
            .on_behalf_of(OrgId(0)),
        )
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotManager);
}

#[test]
fn householder_commonwealth_three_cycles_no_rejection_everyone_is_a_member() {
    let mut h = WorldBuilder::new("commonwealth")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build();
    h.check_every_step = false;
    assert!(
        h.world
            .orgs
            .values()
            .all(|o| o.kind == OrgKind::Cooperative && o.members.len() == 1)
    );
    let mut shared = Money::ZERO;
    for _ in 0..72 {
        let rules = h.rules();
        let tick = h.world.meta.tick;
        let mut scratch = h.world.clone();
        let (events, rejected) = run_round(&mut scratch, &rules, tick);
        assert!(rejected.is_empty(), "{rejected:#?}");
        h.apply_all(events);
        for e in h.tick() {
            if let Event::ShareOutPaid { amount, .. } = e {
                shared += amount;
            }
        }
    }
    h.check();
    assert!(shared > Money::ZERO, "surplus was shared");
    for c in h.world.citizens.values() {
        assert!(has_position(&h.world, c.id), "{} has no position", c.handle);
        assert!(
            h.world.orgs.values().any(|o| o.members.contains(&c.id)),
            "{} is no coop's member",
            c.handle
        );
        assert!(c.household.dwelling.is_some(), "{} is unhoused", c.handle);
    }
    assert!(
        !h.log
            .iter()
            .any(|e| matches!(e, Event::EmploymentOffered { .. })),
        "no coop ever posted a job offer"
    );
}
