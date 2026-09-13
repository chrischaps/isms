#![allow(
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
//! S0.16b done gate: wage grades pay by skill band from the till; the bonus
//! pays exactly on target; the ratchet raises next cycle's target; `SetPlan`
//! publishes targets and patches policy; a transfer request is decided by the
//! System; a short till pays pro rata and conserves.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::{Actor, Event};
use isms_core::explain::{Num, RuleId};
use isms_core::ids::{CitizenId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::labor::has_position;
use isms_core::ledger::{Asset, Holder};
use isms_core::money::Money;
use isms_core::test_support::{Harness, WorldBuilder};
use std::collections::BTreeMap;

/// Two state Farms with three humans each, the till stocked, meters quiet.
fn fixture(till: Money) -> Harness {
    let mut h = WorldBuilder::new("directorate")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(6)
        .org(OrgKind::StateEnterprise, "State Farm")
        .workplace(WorkplaceKind::Farm, 0, 0)
        .workplace(WorkplaceKind::Farm, 0, 0)
        .assign(0, 0, 8, Effort::Normal)
        .assign(1, 0, 8, Effort::Normal)
        .assign(2, 0, 4, Effort::Normal)
        .assign(3, 1, 8, Effort::Normal)
        .assign(4, 1, 8, Effort::Normal)
        .assign(5, 1, 8, Effort::Normal)
        .build();
    h.check_every_step = false;
    for i in 0..6 {
        let mut plan = h.citizen(CitizenId(i)).plan.clone();
        plan.keep_food_at_least = 0;
        plan.buy_wares_when = None;
        h.apply(Event::PlanChanged {
            citizen: CitizenId(i),
            plan: Box::new(plan),
        });
    }
    if till > Money::ZERO {
        h.apply(Event::Seeded {
            holder: Holder::StateStock,
            asset: Asset::Money(till),
        });
    }
    h
}

fn paid(events: &[Event], rule: RuleId) -> BTreeMap<CitizenId, Money> {
    let mut out = BTreeMap::new();
    for e in events {
        if let Event::Paid {
            citizen,
            amount,
            explain,
            ..
        } = e
            && explain.rule == rule
        {
            *out.entry(*citizen).or_insert(Money::ZERO) += *amount;
        }
    }
    out
}

fn set_plan(h: &mut Harness, targets: BTreeMap<WorkplaceId, f64>) -> Vec<Event> {
    let tick = h.world.meta.tick;
    h.cmd(Envelope::system(
        Command::SetPlan {
            targets,
            materials_split: None,
            price_list: None,
            wage_grades: None,
            ration_caps: None,
        },
        tick,
    ))
    .unwrap()
}

#[test]
fn wage_grades_pay_by_skill_band_from_the_till() {
    let mut h = fixture(Money::credits(10_000));
    let till_before = h.world.state_stock.as_ref().unwrap().till;
    let events = h.run_cycle();
    let wages = paid(&events, RuleId::PayScale);
    let grades = h.world.policy.wage_grades.clone().unwrap();
    // Fresh skill sits in band 0 for the whole first cycle (skill grows with
    // hours but stays under 20 after one cycle): grade 0 x hours.
    assert_eq!(wages[&CitizenId(0)], Money(grades[0].0 * 8));
    assert_eq!(wages[&CitizenId(2)], Money(grades[0].0 * 4));
    let total: Money = wages.values().copied().sum();
    assert_eq!(
        h.world.state_stock.as_ref().unwrap().till,
        till_before - total,
        "wages come from the till"
    );
    assert!(h.world.orgs.values().all(|o| o.treasury == Money::ZERO));
    assert!(
        paid(&events, RuleId::PlanBonus).is_empty(),
        "no target, no bonus"
    );
    // A skilled worker moves up a band: 30 skill points is grade 1.
    let mut c = h.citizen(CitizenId(0)).clone();
    c.labor
        .skill
        .get_mut(&WorkplaceKind::Farm.job_family())
        .unwrap()
        .level = 30.0;
    let (band, grade) =
        isms_core::planning::grade_of(&h.world, CitizenId(0), WorkplaceId(0)).unwrap();
    assert_eq!((band, grade), (0, grades[0]), "the live world is unchanged");
    let mut w = h.world.clone();
    w.citizens.insert(CitizenId(0), c);
    assert_eq!(
        isms_core::planning::grade_of(&w, CitizenId(0), WorkplaceId(0)),
        Some((1, grades[1]))
    );
    h.check();
}

#[test]
fn bonus_pays_exactly_on_target_and_the_ratchet_raises_it() {
    let mut h = fixture(Money::credits(10_000));
    // Learn this cycle's output first, then set targets around it.
    let mut probe = h.world.clone();
    let rules = h.rules();
    let mut out = [0.0f64; 2];
    for _ in 0..24 {
        let input = isms_core::tick::TickInput::next_for(&probe);
        let events = isms_core::tick::tick(&probe, &rules, input).unwrap();
        for e in &events {
            if let Event::Produced {
                workplace, units, ..
            } = e
            {
                out[workplace.0 as usize] += f64::from(*units);
            }
            isms_core::apply(&mut probe, e);
        }
    }
    assert!(out[0] > 0.0 && out[1] > 0.0);
    // Workplace 0's target is exactly its output; workplace 1's is one unit more.
    let events = set_plan(
        &mut h,
        BTreeMap::from([(WorkplaceId(0), out[0]), (WorkplaceId(1), out[1] + 1.0)]),
    );
    let out = out[0];
    assert!(matches!(events[0], Event::PlanPublished { .. }));
    assert_eq!(h.world.workplaces[&WorkplaceId(0)].target, Some(out));
    let events = h.run_cycle();
    let wages = paid(&events, RuleId::PayScale);
    let bonus = paid(&events, RuleId::PlanBonus);
    let frac = h.world.policy.plan_bonus_fraction.unwrap();
    for c in [CitizenId(0), CitizenId(1), CitizenId(2)] {
        let expected = Money((wages[&c].0 as f64 * frac).floor() as i64);
        assert_eq!(bonus.get(&c), Some(&expected), "{c}: bonus on target");
    }
    for c in [CitizenId(3), CitizenId(4), CitizenId(5)] {
        assert_eq!(bonus.get(&c), None, "{c}: one unit short, no bonus");
    }
    // The ratchet: on target exactly is not overfulfilment; nothing rises.
    assert_eq!(h.world.workplaces[&WorkplaceId(0)].target, Some(out));
    assert_eq!(
        h.world.workplaces[&WorkplaceId(0)].last_fulfillment,
        Some(1.0)
    );
    assert!(
        h.world.workplaces[&WorkplaceId(1)]
            .last_fulfillment
            .unwrap()
            < 1.0
    );
    // Set a low target and overfulfil: the target rises to output x ratchet_mult.
    set_plan(&mut h, BTreeMap::from([(WorkplaceId(0), 1.0)]));
    let events = h.run_cycle();
    let raised = events.iter().find_map(|e| match e {
        Event::TargetSet {
            workplace: WorkplaceId(0),
            target,
            by: Actor::System,
        } => Some(*target),
        _ => None,
    });
    let output = h.world.workplaces[&WorkplaceId(0)].last_cycle_output;
    assert_eq!(
        raised,
        Some(output * h.world.params.governance.ratchet_mult)
    );
    assert_eq!(h.world.workplaces[&WorkplaceId(0)].target, raised);
    h.check();
}

#[test]
fn set_plan_publishes_targets_and_patches_policy() {
    let mut h = fixture(Money::ZERO);
    let mut grades = h.world.policy.wage_grades.clone().unwrap();
    grades[0] = Money::credits(5);
    let events = h
        .cmd(Envelope::system(
            Command::SetPlan {
                targets: BTreeMap::from([(WorkplaceId(1), 40.0)]),
                materials_split: None,
                price_list: Some(BTreeMap::from([(Good::Food, Money::cents(150))])),
                wage_grades: Some(grades.clone()),
                ration_caps: Some(BTreeMap::from([(Good::Food, 12)])),
            },
            0,
        ))
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[1], Event::PolicyChanged { .. }));
    assert_eq!(h.world.workplaces[&WorkplaceId(1)].target, Some(40.0));
    assert_eq!(h.world.workplaces[&WorkplaceId(0)].target, None);
    assert_eq!(h.world.policy.wage_grades, Some(grades));
    assert_eq!(
        h.world.policy.price_list.as_ref().unwrap()[&Good::Food],
        Money::cents(150)
    );
    assert_eq!(
        h.world.policy.ration_caps.as_ref().unwrap()[&Good::Food],
        12
    );
    // Descending grades are rejected; so is a citizen; so is a market society.
    let err = h
        .cmd(Envelope::system(
            Command::SetPlan {
                targets: BTreeMap::new(),
                materials_split: None,
                price_list: None,
                wage_grades: Some(vec![Money::credits(9), Money::credits(6)]),
                ration_caps: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    let err = h
        .cmd(Envelope::citizen(
            CitizenId(0),
            Command::SetPlan {
                targets: BTreeMap::new(),
                materials_split: None,
                price_list: None,
                wage_grades: None,
                ration_caps: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .build();
    let err = f
        .cmd(Envelope::system(
            Command::SetPlan {
                targets: BTreeMap::new(),
                materials_split: None,
                price_list: None,
                wage_grades: None,
                ration_caps: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
}

#[test]
fn transfer_request_is_decided_by_the_system() {
    let mut h = fixture(Money::ZERO);
    let a = CitizenId(0);
    // Nothing pending yet.
    let err = h
        .cmd(Envelope::system(
            Command::DecideTransfer {
                citizen: a,
                approve: true,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NoRequestPending);
    // A request to the other Farm (which has room), then approval moves the position.
    h.cmd(Envelope::citizen(
        a,
        Command::RequestTransfer {
            to_workplace: WorkplaceId(1),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.transfer_requests.get(&a), Some(&WorkplaceId(1)));
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::DecideTransfer {
                citizen: a,
                approve: true,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    let events = h
        .cmd(Envelope::system(
            Command::DecideTransfer {
                citizen: a,
                approve: true,
            },
            0,
        ))
        .unwrap();
    assert!(matches!(
        events[0],
        Event::TransferDecided { approved: true, .. }
    ));
    assert!(!h.world.workplaces[&WorkplaceId(0)].workers.contains_key(&a));
    assert!(h.world.workplaces[&WorkplaceId(1)].workers.contains_key(&a));
    assert!(
        h.citizen(a).labor.allocations.is_empty(),
        "the allocation resets"
    );
    assert!(h.world.transfer_requests.is_empty());
    assert!(has_position(&h.world, a));
    // A denial leaves the citizen where they are.
    let b = CitizenId(3);
    h.cmd(Envelope::citizen(
        b,
        Command::RequestTransfer {
            to_workplace: WorkplaceId(0),
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::system(
        Command::DecideTransfer {
            citizen: b,
            approve: false,
        },
        0,
    ))
    .unwrap();
    assert!(h.world.workplaces[&WorkplaceId(1)].workers.contains_key(&b));
    // Not in a free-labor society.
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(1)
        .build();
    let err = f
        .cmd(Envelope::citizen(
            CitizenId(0),
            Command::RequestTransfer {
                to_workplace: WorkplaceId(0),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
}

#[test]
fn a_short_till_pays_pro_rata_and_conserves() {
    // Six workers owed 8 h (or 4 h) at grade 0 against a till of 100.00.
    let mut h = fixture(Money::credits(100));
    let grades = h.world.policy.wage_grades.clone().unwrap();
    let owed = Money(grades[0].0 * (8 * 5 + 4));
    assert!(owed > Money::credits(100));
    let events = h.run_cycle();
    let wages = paid(&events, RuleId::PayScale);
    let total: Money = wages.values().copied().sum();
    assert!(total <= Money::credits(100));
    assert!(
        total > Money::credits(99),
        "the till is emptied to the cent: {total}"
    );
    let full = Money(grades[0].0 * 8);
    assert_eq!(wages[&CitizenId(0)], Money(full.0 * 10_000 / owed.0));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PaymentMissed { .. })),
        "no contract, no breach"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Paid { explain, .. }
            if explain.inputs.iter().any(|(n, v)| n == "till" && *v == Num::Money(Money::credits(100)))))
    );
    h.check();
}
