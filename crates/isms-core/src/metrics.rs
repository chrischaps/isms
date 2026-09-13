//! Per-cycle metrics (GDD §14.1, §14.2; TDD §13) computed in the engine at step
//! 8m so live views, the simulator and the Observatory share one definition.
//! Accumulators live in `World` and `Citizen` as continuous state.

use crate::event::CycleAggregates;
use crate::ids::CitizenId;
use crate::kinds::{CitizenKind, Good, Product};
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::world::{ContractBody, ContractStatus, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A citizen's running totals for the current cycle (reset at 8m).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CitizenCycle {
    pub ticks: u32,
    pub food_eaten: u32,
    pub wares_consumed: u32,
    pub housed_ticks: u32,
    /// Sum over ticks of (Food + Shelter + Comfort) / 3, in tenths of a point.
    pub wellbeing_tenths: u64,
    /// Whether Food or Shelter fell below the hardship threshold at any tick.
    pub needs_breached: bool,
}

/// The society's running totals for the current cycle (reset at `CycleClosed`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCycle {
    pub produced: BTreeMap<Good, u64>,
    pub dwellings_built: u64,
    /// Materials consumed by Machine Shops this cycle (the investment decision).
    pub materials_to_machines: u64,
    /// Units requested from the state store this cycle and not served (S0.16a).
    pub store_unfilled: u64,
    /// Units issued at zero price by provision this cycle (S0.16a).
    pub rations_issued: u64,
    /// Tax collected and need floor paid this cycle (S0.17a).
    pub tax_collected: Money,
    pub floor_paid: Money,
}

/// Gini coefficient of a non-negative sample; 0 for an empty or all-equal sample.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn gini(values: &[f64]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    let mut v: Vec<f64> = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = v.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let weighted: f64 = v
        .iter()
        .enumerate()
        .map(|(i, x)| (i as f64 + 1.0) * x)
        .sum();
    (2.0 * weighted) / (n as f64 * total) - (n as f64 + 1.0) / n as f64
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        f64::midpoint(v[n / 2 - 1], v[n / 2])
    }
}

/// Consumption score for a citizen-cycle (TDD §13).
#[must_use]
pub fn consumption_score(world: &World, c: &CitizenCycle) -> f64 {
    let wares_w = world
        .params
        .basket
        .get(&Product::Wares)
        .copied()
        .unwrap_or(1.0);
    f64::from(c.food_eaten)
        + f64::from(c.wares_consumed) * wares_w
        + f64::from(c.housed_ticks) * world.params.metrics.housed_tick_weight
}

/// Compute this cycle's aggregates from the scratch world at 8m.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
pub fn aggregates(world: &World, low_population_cycles: u32) -> CycleAggregates {
    let p = &world.params;
    let active: Vec<&crate::world::Citizen> =
        world.citizens.values().filter(|c| !c.dormant).collect();
    let population = u32::try_from(active.len()).unwrap_or(u32::MAX);
    let active_humans = u32::try_from(
        active
            .iter()
            .filter(|c| c.kind == CitizenKind::Human)
            .count(),
    )
    .unwrap_or(0);
    let householders = population - active_humans;

    let mut real_output = 0.0;
    for (g, units) in &world.cycle.produced {
        real_output += *units as f64 * p.basket.get(&Product::from(*g)).copied().unwrap_or(0.0);
    }
    real_output += world.cycle.dwellings_built as f64
        * p.basket.get(&Product::Dwelling).copied().unwrap_or(0.0);

    let wellbeing: Vec<f64> = active
        .iter()
        .filter(|c| c.cycle.ticks > 0)
        .map(|c| c.cycle.wellbeing_tenths as f64 / (f64::from(c.cycle.ticks) * 10.0))
        .collect();
    let need_met = active
        .iter()
        .filter(|c| c.cycle.ticks > 0 && !c.cycle.needs_breached)
        .count();
    let with_ticks = active.iter().filter(|c| c.cycle.ticks > 0).count();
    let need_fulfillment_rate = if with_ticks == 0 {
        1.0
    } else {
        need_met as f64 / with_ticks as f64
    };
    let scores: Vec<f64> = active
        .iter()
        .map(|c| consumption_score(world, &c.cycle))
        .collect();

    let materials_produced = world
        .cycle
        .produced
        .get(&Good::Materials)
        .copied()
        .unwrap_or(0);
    let investment_share = if materials_produced == 0 {
        0.0
    } else {
        world.cycle.materials_to_machines as f64 / materials_produced as f64
    };

    let unemployed = active
        .iter()
        .filter(|c| !crate::labor::has_position(world, c.id))
        .count();
    let credit_outstanding: Money = world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active)
        .filter_map(|k| match k.body {
            ContractBody::Credit {
                installment,
                installments_left,
                ..
            } => Some(Money(installment.0 * i64::from(installments_left))),
            _ => None,
        })
        .sum();
    let wages: Vec<f64> = active
        .iter()
        .filter(|c| c.cycle_wages > Money::ZERO)
        .map(|c| c.cycle_wages.as_credits_f64())
        .collect();
    let mean_wage = if wages.is_empty() {
        0.0
    } else {
        wages.iter().sum::<f64>() / wages.len() as f64
    };
    let hardship_count =
        u32::try_from(active.iter().filter(|c| c.flags.in_hardship).count()).unwrap_or(0);
    let store_stock: BTreeMap<Good, u32> = world
        .store
        .as_ref()
        .map(|s| s.stock.clone())
        .unwrap_or_default();
    let fulfillments: Vec<f64> = world
        .workplaces
        .values()
        .filter_map(|w| w.target.filter(|t| *t > 0.0).map(|t| w.cycle_output / t))
        .collect();
    let plan_fulfillment = if fulfillments.is_empty() {
        None
    } else {
        Some(fulfillments.iter().sum::<f64>() / fulfillments.len() as f64)
    };
    let contribution_gini = if world.constitution.labor == crate::constitution::LaborMode::Norm {
        let hours: Vec<f64> = active
            .iter()
            .map(|c| f64::from(c.contribution.last_cycle_tick_hours))
            .collect();
        gini(&hours)
    } else {
        0.0
    };

    CycleAggregates {
        population,
        active_humans,
        householders,
        real_output,
        median_wellbeing: median(wellbeing),
        need_fulfillment_rate,
        consumption_gini: gini(&scores),
        investment_share,
        price_index: world.price_index,
        mean_cycle_wage: mean_wage,
        unemployed: u32::try_from(unemployed).unwrap_or(u32::MAX),
        firm_count: u32::try_from(
            world
                .orgs
                .values()
                .filter(|o| !o.workplaces.is_empty())
                .count(),
        )
        .unwrap_or(0),
        credit_outstanding,
        hardship_count,
        store_stock,
        low_population_cycles,
        plan_fulfillment,
        store_unfilled: world.cycle.store_unfilled,
        rations_issued: world.cycle.rations_issued,
        state_stock: world
            .state_stock
            .as_ref()
            .map(|s| s.stock.clone())
            .unwrap_or_default(),
        till: world.state_stock.as_ref().map_or(Money::ZERO, |s| s.till),
        contribution_gini,
        treasury: world.treasury,
        tax_collected: world.cycle.tax_collected,
        floor_paid: world.cycle.floor_paid,
    }
}

/// The wellbeing index of one citizen over the cycle so far: the mean of the
/// three meters, 0..=100 (the Republic's scoreboard alongside net worth).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn wellbeing_index(citizen: &crate::world::Citizen) -> f64 {
    if citizen.cycle.ticks == 0 {
        return 0.0;
    }
    citizen.cycle.wellbeing_tenths as f64 / (f64::from(citizen.cycle.ticks) * 10.0)
}

/// Phase 5 hook: fold this tick's needs into the citizen's cycle accumulators.
pub fn record_tick(
    params: &crate::params::Params,
    c: &mut crate::world::Citizen,
    food_eaten: u32,
    wares_consumed: u32,
) {
    let threshold = u16::from(params.needs.hardship_food_threshold) * crate::needs::TENTHS;
    c.cycle.ticks += 1;
    c.cycle.food_eaten += food_eaten;
    c.cycle.wares_consumed += wares_consumed;
    if c.household.dwelling.is_some() {
        c.cycle.housed_ticks += 1;
    }
    c.cycle.wellbeing_tenths +=
        (u64::from(c.needs.food) + u64::from(c.needs.shelter) + u64::from(c.needs.comfort)) / 3;
    if c.needs.food < threshold || c.needs.shelter < threshold {
        c.cycle.needs_breached = true;
    }
}

/// Step 8m: aggregates, then reset the per-citizen accumulators on the scratch
/// world (the deltas carry the reset; `CycleClosed` resets the world's).
pub fn cycle_end_8m_aggregates(b: &mut TickBuilder) {
    let low = if b.active_humans() < b.world.params.population.floor {
        b.world.meta.low_population_cycles + 1
    } else {
        0
    };
    // Close the workplaces' cycle: remember this cycle's output and fulfilment
    // for the scoreboard and the planner, then take the aggregates and reset.
    for w in b.world.workplaces.values_mut() {
        w.last_cycle_output = w.cycle_output;
        w.last_fulfillment = w.target.filter(|t| *t > 0.0).map(|t| w.cycle_output / t);
    }
    let aggregates = aggregates(&b.world, low);
    crate::labor::reset_cycle_accumulators(&mut b.world);
    let ids: Vec<CitizenId> = b.world.citizens.keys().copied().collect();
    for id in ids {
        if let Some(c) = b.world.citizens.get_mut(&id) {
            c.cycle = CitizenCycle::default();
        }
    }
    b.emit(crate::event::Event::CycleClosed {
        cycle: b.cycle,
        aggregates,
        low_population_cycles: low,
    });
}
