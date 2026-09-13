#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
//! S0.17a done gate (tax and transfer): a flat tax moves exactly the assessed
//! amount to the treasury and conserves; brackets are marginal and
//! hand-computed; an empty bracket list is flat; dividends are taxed at the
//! next assessment; the need floor serves the largest shortfall first and
//! stops at the treasury; every tax and floor event carries an Explain; the
//! minimum wage rejects offers below it and zero means no enforcement; public
//! dwellings are society-owned and assigned to the unhoused; Freeport emits no
//! tax events.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::ids::{CitizenId, OrgId, WorkplaceId};
use isms_core::kinds::{CitizenKind, Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::policy::TaxBracket;
use isms_core::tax::assess;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Owner, Pay};

/// A Republic firm with a Mine, a manager (citizen 0) and `n` fed workers.
fn firm(n: u32, treasury: i64) -> Harness {
    let mut b = WorldBuilder::new("republic")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n + 1)
        .org(OrgKind::Firm, "Iron & Sons")
        .workplace(WorkplaceKind::Mine, 0, 0);
    for i in 0..=n as usize {
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
        asset: Asset::Money(Money::credits(treasury)),
    });
    for i in 0..=n as usize {
        let mut plan = h.citizen(nth(&h, i)).plan.clone();
        plan.keep_food_at_least = 0;
        plan.buy_wares_when = None;
        h.apply(Event::PlanChanged {
            citizen: nth(&h, i),
            plan: Box::new(plan),
        });
    }
    h
}

fn hire(h: &mut Harness, who: usize, wage: Money) {
    let me = nth(h, who);
    let events = h
        .cmd(
            Envelope::citizen(
                nth(h, 0),
                Command::OfferEmployment {
                    org: OrgId(0),
                    workplace: WorkplaceId(0),
                    pay: Pay::Hourly(wage),
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
        me,
        Command::AcceptEmployment { offer },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        me,
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

fn taxes(events: &[Event]) -> Vec<(CitizenId, Money, Money)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::TaxAssessed {
                citizen,
                income,
                tax,
                explain,
            } => {
                assert_eq!(explain.rule, RuleId::TaxIncome);
                Some((*citizen, *income, *tax))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn flat_tax_moves_exactly_the_assessed_amount_to_the_treasury() {
    let mut h = firm(3, 100_000);
    for i in 1..=3 {
        hire(&mut h, i, Money::credits(8));
    }
    let events = h.run_cycle();
    let t = taxes(&events);
    assert_eq!(t.len(), 3);
    let rate = h.world.policy.tax_rate.unwrap();
    let mut total = Money::ZERO;
    for (citizen, income, tax) in &t {
        let wages = events
            .iter()
            .filter_map(|e| match e {
                Event::Paid {
                    citizen: c, amount, ..
                } if c == citizen => Some(*amount),
                _ => None,
            })
            .sum::<Money>();
        assert_eq!(*income, wages, "the base is this cycle's pay");
        assert_eq!(*tax, Money((income.0 as f64 * rate).floor() as i64));
        total += *tax;
    }
    assert_eq!(h.world.treasury, total);
    assert!(
        h.world
            .citizens
            .values()
            .all(|c| c.taxable_income == Money::ZERO),
        "the base resets at the assessment"
    );
    h.check();
}

#[test]
fn brackets_are_marginal_hand_computed_and_an_empty_list_is_flat() {
    let h = firm(0, 0);
    let mut policy = h.world.policy.clone();
    policy.tax_rate = Some(0.2);
    policy.tax_brackets = Some(vec![TaxBracket {
        above: Money::credits(50),
        rate: 0.3,
    }]);
    // 100.00: 50.00 at 20 % = 10.00, plus 50.00 at 30 % = 15.00.
    let (tax, explain) = assess(&policy, Money::credits(100));
    assert_eq!(tax, Money::credits(25));
    assert!(explain.inputs.iter().any(|(n, _)| n == "bracket_1_rate"));
    // Below the bracket only the base rate applies.
    assert_eq!(assess(&policy, Money::credits(40)).0, Money::credits(8));
    policy.tax_brackets = Some(vec![]);
    assert_eq!(assess(&policy, Money::credits(100)).0, Money::credits(20));
    policy.tax_brackets = None;
    assert_eq!(assess(&policy, Money::credits(100)).0, Money::credits(20));
    // Two brackets, unsorted, still marginal: 0-50 at 20 %, 50-80 at 30 %, 80+ at 40 %.
    policy.tax_brackets = Some(vec![
        TaxBracket {
            above: Money::credits(80),
            rate: 0.4,
        },
        TaxBracket {
            above: Money::credits(50),
            rate: 0.3,
        },
    ]);
    assert_eq!(assess(&policy, Money::credits(100)).0, Money::credits(27));
}

#[test]
fn dividends_paid_at_8e_are_taxed_at_the_next_8b() {
    let mut h = firm(1, 10_000);
    // Citizen 0 (the manager) holds every share; declare a dividend.
    let me = nth(&h, 0);
    h.apply(Event::SharesTransferred {
        org: OrgId(0),
        from: isms_core::world::ShareHolder::OrgSelf,
        to: isms_core::world::ShareHolder::Citizen(me),
        qty: 100,
    });
    h.cmd(Envelope::citizen(
        me,
        Command::DeclareDividend {
            org: OrgId(0),
            per_share: Money::credits(1),
        },
        0,
    ))
    .unwrap();
    let events = h.run_cycle();
    let paid: Money = events
        .iter()
        .filter_map(|e| match e {
            Event::DividendPaid { amount, .. } => Some(*amount),
            _ => None,
        })
        .sum();
    assert_eq!(paid, Money::credits(100));
    // 8b ran before 8e this cycle: the dividend is in the base for next cycle.
    assert!(!taxes(&events).iter().any(|(c, ..)| *c == me));
    assert_eq!(h.citizen(me).taxable_income, paid);
    let events = h.run_cycle();
    let t = taxes(&events);
    let mine = t.iter().find(|(c, ..)| *c == me).expect("assessed");
    assert_eq!(mine.1, paid);
    h.check();
}

#[test]
fn need_floor_serves_the_largest_shortfall_first_and_stops_at_the_treasury() {
    let mut h = firm(3, 0);
    // Nobody works; three citizens are broke to different degrees; the treasury holds 20.
    let floor_food = h.world.policy.need_floor_food.unwrap();
    let price = h.world.params.money.start_prices[&Good::Food];
    let floor = Money(price.0 * i64::from(floor_food));
    for i in 1..=3 {
        let c = nth(&h, i);
        let have = h.citizen(c).household.balance;
        h.apply(Event::Transferred {
            from: Party::Citizen(c),
            to: Party::Citizen(nth(&h, 0)),
            asset: Asset::Money(have),
            memo: String::new(),
        });
        // Empty pantries too, so means are zero; then give back a little.
        let food = h
            .citizen(c)
            .household
            .pantry
            .get(&Good::Food)
            .copied()
            .unwrap_or(0);
        h.apply(Event::Transferred {
            from: Party::Citizen(c),
            to: Party::Citizen(nth(&h, 0)),
            asset: Asset::Good(Good::Food, food),
            memo: String::new(),
        });
    }
    // Means: c1 = 0, c2 = floor / 2, c3 = floor - 1 cent.
    h.apply(Event::Seeded {
        holder: Holder::Citizen(nth(&h, 2)),
        asset: Asset::Money(Money(floor.0 / 2)),
    });
    h.apply(Event::Seeded {
        holder: Holder::Citizen(nth(&h, 3)),
        asset: Asset::Money(Money(floor.0 - 1)),
    });
    h.apply(Event::Seeded {
        holder: Holder::Treasury,
        asset: Asset::Money(Money(floor.0 + floor.0 / 4)),
    });
    let events = h.run_cycle();
    let paid: Vec<(CitizenId, Money)> = events
        .iter()
        .filter_map(|e| match e {
            Event::NeedFloorPaid {
                citizen,
                amount,
                from,
                explain,
            } => {
                assert_eq!(*from, Holder::Treasury);
                assert_eq!(explain.rule, RuleId::NeedFloorTransfer);
                Some((*citizen, *amount))
            }
            _ => None,
        })
        .collect();
    // Largest shortfall first (c1 gets the whole floor), then c2 gets what is
    // left (a quarter of the floor, short of its half), c3 nothing.
    assert_eq!(paid.len(), 2, "{paid:?}");
    assert_eq!(paid[0], (nth(&h, 1), floor));
    assert_eq!(paid[1], (nth(&h, 2), Money(floor.0 / 4)));
    assert_eq!(h.world.treasury, Money::ZERO);
    h.check();
}

#[test]
fn minimum_wage_rejects_offers_below_it_and_zero_is_not_enforced() {
    let mut h = firm(1, 1000);
    let floor = h.world.policy.minimum_wage.unwrap();
    assert!(floor > Money::ZERO);
    let offer = |pay: Pay| Command::OfferEmployment {
        org: OrgId(0),
        workplace: WorkplaceId(0),
        pay,
        max_hours: 8,
        term_cycles: None,
        notice_cycles: 1,
        places: 1,
    };
    let manager = nth(&h, 0);
    let err = h
        .cmd(
            Envelope::citizen(manager, offer(Pay::Hourly(floor - Money(1))), 0)
                .on_behalf_of(OrgId(0)),
        )
        .unwrap_err();
    assert_eq!(err.code, RejectCode::BelowMinimumWage);
    // A piece rate is judged at the workplace's base rate: Mine 10 units/h.
    let base = h.world.params.recipes[&WorkplaceKind::Mine].base_rate;
    let low = Money(((floor.0 as f64) / base).floor() as i64 - 1);
    let err = h
        .cmd(Envelope::citizen(manager, offer(Pay::PieceRate(low)), 0).on_behalf_of(OrgId(0)))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::BelowMinimumWage);
    h.cmd(Envelope::citizen(manager, offer(Pay::Hourly(floor)), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    // Zero means no enforcement (Freeport has no minimum wage at all).
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .org(OrgKind::Firm, "Cheap & Co")
        .workplace(WorkplaceKind::Mine, 0, 0)
        .build();
    f.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(nth(&f, 0)),
    });
    f.cmd(Envelope::citizen(nth(&f, 0), offer(Pay::Hourly(Money(1))), 0).on_behalf_of(OrgId(0)))
        .unwrap();
}

#[test]
fn public_dwellings_are_society_owned_and_assigned_to_the_unhoused() {
    let mut h = WorldBuilder::new("republic")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.policy.public_dwellings = Some(3);
        })
        .seed_epoch()
        .build();
    h.check_every_step = false;
    let public: Vec<_> = h
        .world
        .dwellings
        .values()
        .filter(|d| d.owner == Owner::Society)
        .collect();
    assert_eq!(public.len(), 3);
    assert_eq!(h.world.dwellings.len(), 43);
    // Householders take the free public dwellings at join, before any lease.
    assert_eq!(public.iter().filter(|d| d.occupant.is_some()).count(), 3);
    // A human joining with none free waits for a cycle end and a free one.
    let tick = h.world.meta.tick;
    let events = h
        .cmd(Envelope::system(
            Command::Join {
                handle: "eve".into(),
                kind: CitizenKind::Human,
            },
            tick,
        ))
        .unwrap();
    let Event::CitizenJoined { dwelling, .. } = &events[0] else {
        panic!()
    };
    assert_eq!(*dwelling, None);
    h.check();
}

#[test]
fn freeport_emits_no_tax_events() {
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build();
    f.check_every_step = false;
    let events = f.run_cycle();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::TaxAssessed { .. } | Event::NeedFloorPaid { .. }))
    );
    assert_eq!(f.world.treasury, Money::ZERO);
}
