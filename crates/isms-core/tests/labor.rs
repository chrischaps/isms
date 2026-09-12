#![allow(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::bool_to_int_with_if
)] // tests state exact expectations

//! S0.6a done gate (TDD §18.2 S0.6): the output formula, input capping, the
//! Farm -> Mill chain, attribution noise, skill decay, the high-effort streak,
//! and `SetLabor` validation.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::constitution::Monitoring;
use isms_core::event::Event;
use isms_core::ids::WorkplaceId;
use isms_core::kinds::{Effort, Good, JobFamily, OrgKind, WorkplaceKind};
use isms_core::labor::{capital_mult, skill_level, skill_mult, skill_of};
use isms_core::ledger::Party;
use isms_core::needs::FULL;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::Allocation;

fn one_mine(machines: u32, workers: u32) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000; // no presence in these fixtures
        })
        .humans(workers)
        .org(OrgKind::Firm, "Iron & Sons")
        .workplace(WorkplaceKind::Mine, 0, machines);
    for i in 0..workers as usize {
        b = b.pantry(i, Good::Food, 48).assign(i, 0, 8, Effort::Normal);
    }
    b.build()
}

fn produced(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|e| matches!(e, Event::Produced { .. }))
        .collect()
}

#[test]
fn skill_and_capital_formulas_match_hand_computed_values() {
    let h = one_mine(0, 1);
    let p = &h.world.params;
    let level = skill_level(100 * 24, 24, p);
    assert!((level - 47.958).abs() < 0.01, "{level}");
    assert!((skill_mult(48.0) - 1.48).abs() < 1e-9);
    assert!((capital_mult(2, 1, p) - 1.549_306).abs() < 1e-5);
    assert!((capital_mult(0, 3, p) - 1.0).abs() < 1e-12);
    assert!(
        (capital_mult(5, 0, p) - 1.0).abs() < 1e-12,
        "no workers, no multiplier"
    );
    assert!((skill_level(0, 24, p)).abs() < 1e-12);
    assert!(skill_level(u64::MAX / 2, 24, p) <= 100.0);
}

#[test]
fn a_mine_produces_base_rate_times_hours() {
    // 10 Ore/h * 8/24 h = 3.333 per tick; integer units with a carried remainder.
    let mut h = one_mine(0, 1);
    let events = h.tick();
    let ps = produced(&events);
    assert_eq!(ps.len(), 1);
    let Event::Produced {
        output,
        units,
        per_worker,
        ..
    } = ps[0]
    else {
        unreachable!()
    };
    assert_eq!(*output, Good::Ore);
    assert_eq!(*units, 3);
    assert!((per_worker[0].true_output - 10.0 / 3.0).abs() < 1e-9);
    assert_eq!(
        per_worker[0].attributed_output, per_worker[0].true_output,
        "sigma 0"
    );
    assert_eq!(
        per_worker[0].explain.rule,
        isms_core::explain::RuleId::LaborOutput
    );
    assert_eq!(
        h.world.orgs[&isms_core::ids::OrgId(0)].inventory[&Good::Ore],
        3
    );
    let wp = &h.world.workplaces[&WorkplaceId(0)];
    assert!((wp.output_remainder - 1.0 / 3.0).abs() < 1e-9);
    // Over more ticks (skill grows, the citizen is unhoused), the org holds exactly
    // the floor of the summed true output, and the remainder is the fraction.
    let mut total = per_worker[0].true_output;
    for _ in 0..5 {
        for e in h.tick() {
            if let Event::Produced { per_worker, .. } = e {
                total += per_worker[0].true_output;
            }
        }
    }
    let held = h.world.orgs[&isms_core::ids::OrgId(0)].inventory[&Good::Ore];
    assert_eq!(f64::from(held), total.floor());
    let rem = h.world.workplaces[&WorkplaceId(0)].output_remainder;
    assert!((rem - (total - total.floor())).abs() < 1e-9);
}

#[test]
fn output_scales_with_effort_machines_and_needs() {
    // High effort x1.3 and 2 machines per worker x1.549.
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000; // no presence in these fixtures
        })
        .humans(1)
        .pantry(0, Good::Food, 48)
        .org(OrgKind::Firm, "x")
        .workplace(WorkplaceKind::Mine, 0, 2)
        .assign(0, 0, 8, Effort::High)
        .build();
    let events = h.tick();
    let Event::Produced { per_worker, .. } = produced(&events)[0] else {
        unreachable!()
    };
    let expected = 10.0 * 1.3 * capital_mult(2, 1, &h.world.params) * 8.0 / 24.0;
    assert!((per_worker[0].true_output - expected).abs() < 1e-9);
    // next tick the citizen is unhoused (x0.7 from needs) and has a little skill
    let skill = skill_mult(skill_of(&h.world, nth(&h, 0), JobFamily::Mining).level);
    assert!(skill > 1.0 && skill < 1.01);
    let events = h.tick();
    let Event::Produced { per_worker, .. } = produced(&events)[0] else {
        unreachable!()
    };
    assert!((per_worker[0].true_output - expected * 0.7 * skill).abs() < 1e-9);
}

#[test]
fn a_mill_with_five_grain_makes_five_food_and_no_more() {
    // Two millers: 15 * 16/24 = 10 Food of labor, but only 5 Grain.
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000; // no presence in these fixtures
        })
        .humans(2)
        .org(OrgKind::Firm, "Millers' Row")
        .workplace(WorkplaceKind::Mill, 0, 0)
        .org_inventory(0, Good::Grain, 5)
        .assign(0, 0, 8, Effort::Normal)
        .assign(1, 0, 8, Effort::Normal)
        .build();
    let events = h.tick();
    let Event::Produced {
        units,
        inputs_consumed,
        ..
    } = produced(&events)[0]
    else {
        unreachable!()
    };
    assert_eq!(*units, 5);
    assert_eq!(inputs_consumed[&Good::Grain], 5);
    let org = &h.world.orgs[&isms_core::ids::OrgId(0)];
    assert_eq!(org.inventory.get(&Good::Grain), None);
    assert_eq!(org.inventory[&Good::Food], 5);
    assert_eq!(
        h.world.workplaces[&WorkplaceId(0)].output_remainder,
        0.0,
        "starved: no carry"
    );
    // and with no Grain at all, nothing is produced or consumed
    let events = h.tick();
    assert!(produced(&events).is_empty());
}

#[test]
fn farm_to_mill_chain_conserves_over_three_cycles() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000; // no presence in these fixtures
        })
        .humans(2)
        .pantry(0, Good::Food, 48)
        .pantry(1, Good::Food, 48)
        .org(OrgKind::Firm, "Farm & Mill")
        .workplace(WorkplaceKind::Farm, 0, 0)
        .workplace(WorkplaceKind::Mill, 0, 0)
        .assign(0, 0, 8, Effort::Normal)
        .assign(1, 1, 8, Effort::Normal)
        .build();
    h.check_every_step = false;
    h.run_cycles(3);
    h.check();
    let org = &h.world.orgs[&isms_core::ids::OrgId(0)];
    assert!(org.inventory[&Good::Food] > 100, "{:?}", org.inventory);
    let produced_food = h.world.ledger_meta.produced[&Good::Food];
    let consumed_grain = h.world.ledger_meta.consumed[&Good::Grain];
    assert_eq!(produced_food, consumed_grain);
    // skill grew in both families
    assert!(skill_of(&h.world, nth(&h, 0), JobFamily::Farming).level > 10.0);
    assert!(skill_of(&h.world, nth(&h, 1), JobFamily::Milling).level > 10.0);
}

#[test]
fn attribution_noise_has_the_stated_mean_and_sigma() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000;
            p.constitution.monitoring = Monitoring::Low; // sigma 0.6
        })
        .humans(12)
        .org(OrgKind::Firm, "x")
        .workplace(WorkplaceKind::Mine, 0, 0)
        .workplace(WorkplaceKind::Mine, 0, 0)
        .build();
    for i in 0..12usize {
        let workplace = WorkplaceId(u32::from(i >= 6));
        h.apply(Event::Assigned {
            workplace,
            citizen: nth(&h, i),
            contract: None,
        });
        h.apply(Event::LaborSet {
            citizen: nth(&h, i),
            allocations: vec![Allocation {
                workplace,
                hours: 8,
                effort: Effort::Normal,
            }],
        });
    }
    assert_eq!(h.capabilities().monitoring_sigma, 0.6);
    h.check_every_step = false;
    let mut ratios = Vec::new();
    while ratios.len() < 10_000 {
        // keep them fed so true output stays positive
        for i in 0..12 {
            let me = nth(&h, i);
            let mut needs = h.citizen(me).needs.clone();
            needs.food = FULL;
            h.set_needs(me, needs);
        }
        for e in h.tick() {
            if let Event::Produced { per_worker, .. } = e {
                for w in per_worker {
                    ratios.push(w.attributed_output / w.true_output);
                }
            }
        }
    }
    let n = ratios.len() as f64;
    let mean = ratios.iter().sum::<f64>() / n;
    let var = ratios.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    // max(0, 1 + N(0, 0.6)) truncates the lower tail slightly.
    assert!((mean - 1.0).abs() < 0.03, "mean {mean}");
    assert!((var.sqrt() - 0.6).abs() < 0.05, "sigma {}", var.sqrt());
    assert!(ratios.iter().all(|r| *r >= 0.0));
    h.check();
}

#[test]
fn skill_decays_one_point_per_ten_idle_cycles() {
    let mut h = one_mine(0, 1);
    h.check_every_step = false;
    let me = nth(&h, 0);
    h.run_cycle();
    let worked = skill_of(&h.world, me, JobFamily::Mining);
    assert!(worked.level > 5.0);
    assert_eq!(worked.idle_cycles, 0);
    h.apply(Event::LaborSet {
        citizen: me,
        allocations: vec![],
    });
    h.run_cycles(9);
    assert_eq!(
        skill_of(&h.world, me, JobFamily::Mining).level,
        worked.level
    );
    h.run_cycle();
    let idle = skill_of(&h.world, me, JobFamily::Mining);
    assert_eq!(idle.idle_cycles, 10);
    assert!((idle.level - (worked.level - 1.0)).abs() < 1e-9);
    h.check();
}

#[test]
fn high_effort_for_two_cycles_carries_no_debt_and_three_carries_one_hour() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000; // no presence in these fixtures
        })
        .humans(1)
        .pantry(0, Good::Food, 48 * 4)
        .org(OrgKind::Firm, "x")
        .workplace(WorkplaceKind::Mine, 0, 0)
        .assign(0, 0, 8, Effort::High)
        .build();
    h.check_every_step = false;
    let me = nth(&h, 0);
    for cycle in 1..=3u8 {
        let mut needs = h.citizen(me).needs.clone();
        needs.food = FULL;
        h.set_needs(me, needs);
        h.run_cycle();
        let c = h.citizen(me);
        assert_eq!(c.labor.consecutive_high_effort_cycles, cycle);
        let expected_debt = if cycle >= 3 { 1 } else { 0 };
        assert_eq!(c.labor.fatigue_debt, expected_debt, "after cycle {cycle}");
        assert_eq!(c.labor.budget, 8 - expected_debt, "after cycle {cycle}");
    }
    h.check();
}

#[test]
fn set_labor_validates_assignment_budget_and_count() {
    let mut h = one_mine(0, 1);
    let me = nth(&h, 0);
    let wp = WorkplaceId(0);
    let over = h.cmd_dry(Envelope::citizen(
        me,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: wp,
                hours: 9,
                effort: Effort::Normal,
            }],
        },
        0,
    ));
    assert_eq!(over.unwrap_err().code, RejectCode::OverBudget);
    let missing = h.cmd_dry(Envelope::citizen(
        me,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: WorkplaceId(9),
                hours: 1,
                effort: Effort::Normal,
            }],
        },
        0,
    ));
    assert_eq!(missing.unwrap_err().code, RejectCode::UnknownWorkplace);
    // a second workplace exists but they hold no position there
    h.apply(Event::WorkplaceAdded {
        workplace: WorkplaceId(1),
        org: isms_core::ids::OrgId(0),
        kind: WorkplaceKind::Foundry,
        slot: None,
        materials_consumed: 0,
    });
    let unassigned = h.cmd_dry(Envelope::citizen(
        me,
        Command::SetLabor {
            allocations: vec![Allocation {
                workplace: WorkplaceId(1),
                hours: 1,
                effort: Effort::Normal,
            }],
        },
        0,
    ));
    assert_eq!(unassigned.unwrap_err().code, RejectCode::NotAssigned);
    let ok = h
        .cmd(Envelope::citizen(
            me,
            Command::SetLabor {
                allocations: vec![Allocation {
                    workplace: wp,
                    hours: 6,
                    effort: Effort::Low,
                }],
            },
            0,
        ))
        .unwrap();
    assert!(matches!(ok[0], Event::LaborSet { .. }));
    assert_eq!(h.citizen(me).labor.allocations[0].hours, 6);
    assert_eq!(h.world.workplaces[&wp].workers[&me].hours, 6);
    // Unassigned drops the allocation too
    h.apply(Event::Unassigned {
        workplace: wp,
        citizen: me,
    });
    assert!(h.citizen(me).labor.allocations.is_empty());
    let _ = Party::Citizen(me);
}
