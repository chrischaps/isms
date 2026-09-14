#![allow(clippy::many_single_char_names)]
//! S0.17d done gate (unions): only employees of the firm join; an agreement
//! raises member pay to the floor at 8a; offers below the agreed floor are
//! rejected; a strike zeroes hours and pays strike pay from dues; strike pay
//! never exceeds the union's treasury; Freeport rejects `FormUnion`.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::ids::{OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder};
use isms_core::money::Money;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::Pay;

/// A Republic firm (citizen 0 manages) employing citizens 1..=n at 7.00 an
/// hour, plus one outsider; meters quiet, pantries full.
fn firm(n: u32) -> Harness {
    let mut b = WorldBuilder::new("republic")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n + 2)
        .org(OrgKind::Firm, "Iron & Sons")
        .workplace(WorkplaceKind::Mine, 0, 0);
    for i in 0..(n + 2) as usize {
        b = b.pantry(i, Good::Food, 48);
    }
    let mut h = b.build();
    h.check_every_step = false;
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(nth(&h, 0)),
    });
    h.apply(Event::Seeded {
        holder: Holder::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(100_000)),
    });
    for i in 0..(n + 2) as usize {
        let c = nth(&h, i);
        let mut plan = h.citizen(c).plan.clone();
        plan.keep_food_at_least = 0;
        plan.buy_wares_when = None;
        h.apply(Event::PlanChanged {
            citizen: c,
            plan: Box::new(plan),
        });
    }
    for i in 1..=n as usize {
        let events = h
            .cmd(
                Envelope::citizen(
                    nth(&h, 0),
                    Command::OfferEmployment {
                        org: OrgId(0),
                        workplace: WorkplaceId(0),
                        pay: Pay::Hourly(Money::credits(7)),
                        max_hours: 8,
                        term_cycles: None,
                        notice_cycles: 1,
                        places: 1,
                    },
                    0,
                )
                .on_behalf_of(OrgId(0)),
            )
            .unwrap();
        let Event::EmploymentOffered { offer, .. } = events[0] else {
            panic!()
        };
        h.cmd(Envelope::citizen(
            nth(&h, i),
            Command::AcceptEmployment { offer },
            0,
        ))
        .unwrap();
        h.cmd(Envelope::citizen(
            nth(&h, i),
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
    }
    h
}

fn form(h: &mut Harness) -> OrgId {
    let events = h
        .cmd(Envelope::citizen(
            nth(h, 1),
            Command::FormUnion {
                firm: OrgId(0),
                name: "Miners United".into(),
            },
            0,
        ))
        .unwrap();
    let Event::OrgFounded { org, .. } = events[0] else {
        panic!()
    };
    assert!(matches!(events[1], Event::UnionFormed { .. }));
    org
}

fn paid_to(events: &[Event], who: isms_core::ids::CitizenId) -> Money {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Paid {
                citizen, amount, ..
            } if *citizen == who => Some(*amount),
            _ => None,
        })
        .sum()
}

#[test]
fn only_employees_of_the_firm_join() {
    let mut h = firm(2);
    let outsider = nth(&h, 3);
    // An outsider cannot found the union.
    let err = h
        .cmd(Envelope::citizen(
            outsider,
            Command::FormUnion {
                firm: OrgId(0),
                name: "x".into(),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotParty);
    let union = form(&mut h);
    assert_eq!(h.world.orgs[&union].kind, OrgKind::Union);
    assert_eq!(h.world.orgs[&union].manager, Some(nth(&h, 1)));
    assert!(h.world.orgs[&union].members.contains(&nth(&h, 1)));
    // Only one union per firm.
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 2),
            Command::FormUnion {
                firm: OrgId(0),
                name: "y".into(),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::AlreadyExists);
    // The steward admits a fellow employee, not an outsider.
    h.cmd(Envelope::citizen(
        nth(&h, 1),
        Command::AdmitMember {
            org: union,
            citizen: nth(&h, 2),
        },
        0,
    ))
    .unwrap();
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::AdmitMember {
                org: union,
                citizen: outsider,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotParty);
}

#[test]
fn an_agreement_raises_member_pay_to_the_floor_and_caps_hours() {
    let mut h = firm(2);
    let union = form(&mut h);
    // Citizen 2 stays out of the union: paid at the contract's 7.00.
    let events = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::OfferCollectiveAgreement {
                union,
                wage_floor: Money::credits(9),
                hours: 6,
                term_cycles: 2,
            },
            0,
        ))
        .unwrap();
    let Event::CollectiveAgreementOffered { offer, .. } = events[0] else {
        panic!()
    };
    // The firm's manager signs (nobody else can).
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 2),
            Command::AcceptCollectiveAgreement { offer },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotManager);
    h.cmd(
        Envelope::citizen(nth(&h, 0), Command::AcceptCollectiveAgreement { offer }, 0)
            .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert_eq!(
        isms_core::union::agreement_for(&h.world, OrgId(0)),
        Some((Money::credits(9), 6))
    );
    assert_eq!(
        isms_core::tax::wage_floor(&h.world, OrgId(0)),
        Money::credits(9)
    );
    // The member's 8 h allocation now exceeds the agreed 6 h cap.
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::SetLabor {
                allocations: vec![isms_core::world::Allocation {
                    workplace: WorkplaceId(0),
                    hours: 8,
                    effort: Effort::Normal,
                }],
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::OverContractHours);
    h.cmd(Envelope::citizen(
        nth(&h, 1),
        Command::SetLabor {
            allocations: vec![isms_core::world::Allocation {
                workplace: WorkplaceId(0),
                hours: 6,
                effort: Effort::Normal,
            }],
        },
        0,
    ))
    .unwrap();
    // A new offer below the agreed floor is rejected; at it, accepted.
    let offer_at = |pay: i64| Command::OfferEmployment {
        org: OrgId(0),
        workplace: WorkplaceId(0),
        pay: Pay::Hourly(Money::credits(pay)),
        max_hours: 8,
        term_cycles: None,
        notice_cycles: 1,
        places: 1,
    };
    let err = h
        .cmd(Envelope::citizen(nth(&h, 0), offer_at(8), 0).on_behalf_of(OrgId(0)))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::BelowMinimumWage);
    h.cmd(Envelope::citizen(nth(&h, 0), offer_at(9), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    let events = h.run_cycle();
    // Member: 6 h at 9.00 (the floor), non-member: 8 h at 7.00; dues of 0.50 paid.
    assert_eq!(paid_to(&events, nth(&h, 1)), Money::credits(54));
    assert_eq!(paid_to(&events, nth(&h, 2)), Money::credits(56));
    let dues = h.world.params.union.dues_per_cycle;
    assert_eq!(h.world.orgs[&union].treasury, dues);
    assert!(events.iter().any(|e| matches!(e, Event::DuesPaid { .. })));
    // The agreement ends after its two cycles.
    h.run_cycle();
    assert_eq!(isms_core::union::agreement_for(&h.world, OrgId(0)), None);
    h.check();
}

#[test]
fn a_strike_zeroes_hours_and_pays_strike_pay_from_dues() {
    let mut h = firm(2);
    let union = form(&mut h);
    h.cmd(Envelope::citizen(
        nth(&h, 1),
        Command::AdmitMember {
            org: union,
            citizen: nth(&h, 2),
        },
        0,
    ))
    .unwrap();
    // Build up dues: 200 credits of dues seeded into the union.
    h.apply(Event::Seeded {
        holder: Holder::Org(union),
        asset: Asset::Money(Money::credits(200)),
    });
    let tick = h.world.meta.tick;
    let events = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::CallStrike { union, cycles: 1 },
            tick,
        ))
        .unwrap();
    assert!(matches!(
        events[0],
        Event::StrikeCalled { until_cycle: 1, .. }
    ));
    let err = h
        .cmd(Envelope::citizen(
            nth(&h, 1),
            Command::CallStrike { union, cycles: 1 },
            tick,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::AlreadyExists);
    let events = h.run_cycle();
    // No work, no wages, no output; strike pay to each member.
    assert_eq!(paid_to(&events, nth(&h, 1)), Money::ZERO);
    assert_eq!(paid_to(&events, nth(&h, 2)), Money::ZERO);
    assert!(!events.iter().any(|e| matches!(e, Event::Produced { .. })));
    let strike_pay = h.world.params.union.strike_pay_per_cycle;
    let paid: Vec<(isms_core::ids::CitizenId, Money)> = events
        .iter()
        .filter_map(|e| match e {
            Event::StrikePaid {
                citizen,
                amount,
                explain,
                ..
            } => {
                assert_eq!(explain.rule, RuleId::StrikePay);
                Some((*citizen, *amount))
            }
            _ => None,
        })
        .collect();
    assert_eq!(paid.len(), 2);
    assert!(paid.iter().all(|(_, a)| *a == strike_pay));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StrikeEnded { .. }))
    );
    assert_eq!(h.world.orgs[&union].union.unwrap().strike_until, None);
    // Work resumes next cycle.
    let events = h.run_cycle();
    assert!(paid_to(&events, nth(&h, 1)) > Money::ZERO);
    h.check();
}

#[test]
fn strike_pay_never_exceeds_the_union_treasury() {
    let mut h = firm(2);
    let union = form(&mut h);
    h.cmd(Envelope::citizen(
        nth(&h, 1),
        Command::AdmitMember {
            org: union,
            citizen: nth(&h, 2),
        },
        0,
    ))
    .unwrap();
    // Only 10 credits in the till for two strikers: 5 each (dues come in first, adding 1).
    h.apply(Event::Seeded {
        holder: Holder::Org(union),
        asset: Asset::Money(Money::credits(10)),
    });
    h.cmd(Envelope::citizen(
        nth(&h, 1),
        Command::CallStrike { union, cycles: 1 },
        0,
    ))
    .unwrap();
    let events = h.run_cycle();
    let paid: Money = events
        .iter()
        .filter_map(|e| match e {
            Event::StrikePaid { amount, .. } => Some(*amount),
            _ => None,
        })
        .sum();
    let dues = h.world.params.union.dues_per_cycle;
    assert_eq!(paid, Money::credits(10) + Money(dues.0 * 2));
    assert_eq!(h.world.orgs[&union].treasury, Money::ZERO);
    h.check();
}

#[test]
fn freeport_rejects_form_union_not_in_this_society() {
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .org(OrgKind::Firm, "F")
        .build();
    let err = f
        .cmd(Envelope::citizen(
            nth(&f, 0),
            Command::FormUnion {
                firm: OrgId(0),
                name: "x".into(),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
}
