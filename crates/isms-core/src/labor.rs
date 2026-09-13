//! Labor and production (GDD §4.3, §4.4; TDD §5.5 phases 3–4, step 8g, §5.7).
//!
//! Hours are integer tick-hours (Q5): an allocation of `h` hours per cycle
//! contributes `h` tick-hours per tick, i.e. `h / ticks_per_cycle` real hours.
//! Transcendentals go through `libm` so every platform agrees bit for bit.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::{Event, WorkerOutput};
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, WorkplaceId};
use crate::kinds::{Effort, Good, JobFamily};
use crate::params::Params;
use crate::tick::TickBuilder;
use crate::world::{Allocation, Skill, World};
use rand_distr::{Distribution, Normal};
use std::collections::BTreeMap;

/// `skill = min(max, k * ln(1 + hours / h0))` from real hours in the family.
#[must_use]
#[allow(clippy::cast_precision_loss)] // tick-hours never approach 2^52
pub fn skill_level(tick_hours: u64, ticks_per_cycle: u32, params: &Params) -> f64 {
    let hours = tick_hours as f64 / f64::from(ticks_per_cycle);
    let raw = params.skill.k * libm::log(1.0 + hours / params.skill.h0);
    raw.min(f64::from(params.skill.max))
}

/// `skill_mult = 1 + skill / 100` (1.0 → 2.0).
#[must_use]
pub fn skill_mult(level: f64) -> f64 {
    1.0 + level / 100.0
}

/// `capital_mult = 1 + coeff * ln(1 + machines / workers)`.
#[must_use]
pub fn capital_mult(machines: u32, workers: u32, params: &Params) -> f64 {
    if workers == 0 {
        return 1.0;
    }
    1.0 + params.capital.capital_log_coeff
        * libm::log(1.0 + f64::from(machines) / f64::from(workers))
}

/// One worker's contribution this tick, before input capping.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkerTick {
    pub citizen: CitizenId,
    pub tick_hours: u32,
    pub effort: Effort,
    pub family: JobFamily,
    pub skill_mult: f64,
    pub effort_mult: f64,
    pub output_mult: f64,
}

/// Phase 3: each non-dormant citizen's effective tick-hours per workplace this
/// tick. Allocations are a rate; if they exceed the budget they are scaled pro
/// rata in integer tick-hours (Q4). Only assignments count (Q16).
#[must_use]
pub fn phase_3_labor(world: &World) -> BTreeMap<WorkplaceId, Vec<WorkerTick>> {
    let params = &world.params;
    let mut by_workplace: BTreeMap<WorkplaceId, Vec<WorkerTick>> = BTreeMap::new();
    for c in world.citizens.values().filter(|c| !c.dormant) {
        let total: u32 = c.labor.allocations.iter().map(|a| u32::from(a.hours)).sum();
        if total == 0 {
            continue;
        }
        let budget = u32::from(c.labor.budget);
        for a in &c.labor.allocations {
            let Some(wp) = world.workplaces.get(&a.workplace) else {
                continue;
            };
            if !wp.workers.contains_key(&c.id) || a.hours == 0 {
                continue;
            }
            let hours = u32::from(a.hours);
            let tick_hours = if total > budget {
                hours * budget / total
            } else {
                hours
            };
            if tick_hours == 0 {
                continue;
            }
            let family = wp.kind.job_family();
            let level = c.labor.skill.get(&family).map_or(0.0, |s| s.level);
            by_workplace.entry(wp.id).or_default().push(WorkerTick {
                citizen: c.id,
                tick_hours,
                effort: a.effort,
                family,
                skill_mult: skill_mult(level),
                effort_mult: params.labor.effort_output_mult.get(a.effort),
                output_mult: c.labor.output_mult,
            });
        }
    }
    by_workplace
}

/// Phase 4: production per workplace, in workplace id order.
#[allow(clippy::too_many_lines, clippy::single_match_else)]
pub fn phase_4_production(b: &mut TickBuilder, labor: &BTreeMap<WorkplaceId, Vec<WorkerTick>>) {
    let params = b.world.params.clone();
    let sigma = b.rules.capabilities.monitoring_sigma;
    let noise = (sigma > 0.0).then(|| Normal::new(0.0, sigma).expect("sigma is finite"));
    let ticks_per_cycle = params.time.ticks_per_cycle;
    for (wp_id, workers) in labor {
        let Some(wp) = b.world.workplaces.get(wp_id) else {
            continue;
        };
        let recipe = &params.recipes[&wp.kind];
        let output_good = recipe.produces.as_good();
        let org_id = wp.org;
        let headcount = u32::try_from(workers.len()).unwrap_or(u32::MAX);
        let cap_mult = capital_mult(wp.machines, headcount, &params);
        let base_rate = recipe.base_rate;

        // Per-worker true output and attribution.
        let mut per_worker = Vec::with_capacity(workers.len());
        let mut total = 0.0f64;
        for w in workers {
            let hours = f64::from(w.tick_hours) / f64::from(ticks_per_cycle);
            let true_output =
                base_rate * w.skill_mult * w.effort_mult * w.output_mult * cap_mult * hours;
            let factor = noise.map_or(1.0, |n| (1.0 + n.sample(&mut b.rng)).max(0.0));
            let attributed = true_output * factor;
            total += true_output;
            let explain = Explain::new(
                RuleId::LaborOutput,
                "base * skill * effort * needs * capital * hours",
                true_output,
            )
            .input("base_rate", base_rate)
            .input("skill_mult", w.skill_mult)
            .input("effort_mult", w.effort_mult)
            .input("needs_mult", w.output_mult)
            .input("capital_mult", cap_mult)
            .input("hours", hours);
            per_worker.push(WorkerOutput {
                citizen: w.citizen,
                tick_hours: w.tick_hours,
                true_output,
                attributed_output: attributed,
                explain,
            });
        }

        // Integer units with a carried remainder, capped by inputs (Q21).
        let raw = wp.output_remainder + total;
        let uncapped = raw.floor();
        let mut remainder = raw - uncapped;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut units = uncapped as u32;
        let stock = crate::ledger::stock_holder(&b.world, org_id);
        let possible = recipe
            .consumes
            .iter()
            .map(|(g, per)| crate::ledger::goods_at(&b.world, stock, *g) / *per)
            .min()
            .unwrap_or(u32::MAX);
        if units > possible {
            units = possible;
            remainder = 0.0;
        }
        let inputs_consumed: BTreeMap<Good, u32> = recipe
            .consumes
            .iter()
            .map(|(g, per)| (*g, per * units))
            .collect();

        // Continuous bookkeeping on the scratch world.
        {
            let wp = b.world.workplaces.get_mut(wp_id).expect("workplace exists");
            wp.output_remainder = remainder;
            wp.cycle_output += f64::from(units);
            for w in &per_worker {
                if let Some(a) = wp.workers.get_mut(&w.citizen) {
                    a.cycle_tick_hours += w.tick_hours;
                    a.cycle_attributed += w.attributed_output;
                }
            }
        }
        for w in workers {
            let c = b
                .world
                .citizens
                .get_mut(&w.citizen)
                .expect("citizen exists");
            let s = c.labor.skill.entry(w.family).or_default();
            s.tick_hours += u64::from(w.tick_hours);
            s.tick_hours_this_cycle += w.tick_hours;
            s.level = skill_level(s.tick_hours, ticks_per_cycle, &params);
        }
        let consumed_any = inputs_consumed.values().any(|q| *q > 0);
        match output_good {
            Some(output) => {
                if units > 0 || consumed_any {
                    b.emit(Event::Produced {
                        workplace: *wp_id,
                        tick: b.tick,
                        output,
                        units,
                        inputs_consumed,
                        per_worker,
                    });
                }
            }
            None => {
                // A Builder's output is a dwelling asset per unit (GDD 4.1).
                let per_unit =
                    inputs_consumed.get(&Good::Materials).copied().unwrap_or(0) / units.max(1);
                for _ in 0..units {
                    let dwelling = b.world.next.dwelling;
                    b.emit(Event::DwellingBuilt {
                        dwelling,
                        org: org_id,
                        workplace: Some(*wp_id),
                        materials_consumed: per_unit,
                    });
                }
            }
        }
    }
}

/// Step 8g: skill decay for idle families and the high-effort streak (T20).
pub fn cycle_end_8g_skill_and_effort(b: &mut TickBuilder) {
    let params = b.world.params.clone();
    let ids: Vec<CitizenId> = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| c.id)
        .collect();
    for id in ids {
        let c = b.world.citizens.get_mut(&id).expect("citizen exists");
        for s in c.labor.skill.values_mut() {
            if s.tick_hours_this_cycle == 0 {
                s.idle_cycles += 1;
                if s.idle_cycles
                    .is_multiple_of(params.skill.decay_per_idle_cycles)
                {
                    s.level = (s.level - 1.0).max(0.0);
                }
            } else {
                s.idle_cycles = 0;
            }
            s.tick_hours_this_cycle = 0;
        }
        let high = c
            .labor
            .allocations
            .iter()
            .any(|a| a.hours > 0 && a.effort == Effort::High);
        c.labor.consecutive_high_effort_cycles = if high {
            c.labor.consecutive_high_effort_cycles.saturating_add(1)
        } else {
            0
        };
    }
    for wp in b.world.workplaces.values_mut() {
        wp.cycle_output = 0.0;
        for a in wp.workers.values_mut() {
            a.cycle_tick_hours = 0;
            a.cycle_attributed = 0.0;
        }
    }
}

/// `SetLabor`: at most `max_workplaces` allocations, each to a workplace where the
/// citizen holds an assignment, hours summing to at most this cycle's budget.
pub fn set_labor(
    world: &World,
    envelope: &Envelope<Command>,
    allocations: &[Allocation],
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let p = &world.params.labor;
    if allocations.len() > usize::from(p.max_workplaces) {
        return Err(Reject::new(
            RejectCode::TooManyWorkplaces,
            format!("at most {} workplaces", p.max_workplaces),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for a in allocations {
        if !seen.insert(a.workplace) {
            return Err(Reject::new(
                RejectCode::InvalidQuantity,
                "duplicate workplace",
            ));
        }
        let wp = world.workplaces.get(&a.workplace).ok_or_else(|| {
            Reject::new(
                RejectCode::UnknownWorkplace,
                format!("no workplace {}", a.workplace),
            )
        })?;
        let Some(assignment) = wp.workers.get(&citizen.id) else {
            return Err(Reject::new(
                RejectCode::NotAssigned,
                format!("{} has no position at {}", citizen.id, a.workplace),
            ));
        };
        if let Some(k) = assignment.contract.and_then(|k| world.contracts.get(&k))
            && let crate::world::ContractBody::Employment { max_hours, .. } = k.body
            && a.hours > max_hours
        {
            return Err(Reject::new(
                RejectCode::OverContractHours,
                format!(
                    "the contract allows at most {max_hours} h at {}",
                    a.workplace
                ),
            ));
        }
    }
    let total: u32 = allocations.iter().map(|a| u32::from(a.hours)).sum();
    if total > u32::from(citizen.labor.budget) {
        return Err(Reject::new(
            RejectCode::OverBudget,
            format!(
                "{total} h exceeds this cycle's budget of {} h",
                citizen.labor.budget
            ),
        ));
    }
    Ok(vec![Event::LaborSet {
        citizen: citizen.id,
        allocations: allocations.to_vec(),
    }])
}

/// Skill state helper for tests and views.
#[must_use]
pub fn skill_of(world: &World, citizen: CitizenId, family: JobFamily) -> Skill {
    world
        .citizens
        .get(&citizen)
        .and_then(|c| c.labor.skill.get(&family).copied())
        .unwrap_or_default()
}
