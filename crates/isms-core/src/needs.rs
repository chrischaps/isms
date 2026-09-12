//! Needs, consumption, hardship and destitution (GDD §4.2, Q3; TDD §5.5 phase 5
//! and step 8h). Meters are kept in integer tenths of a point (0..=1000) so the
//! fractional decay rates in Appendix A are exact and deterministic.

use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::CitizenId;
use crate::kinds::{Effort, Good};
use crate::params::Params;
use crate::tick::TickBuilder;
use crate::world::{Citizen, Needs};

/// Tenths per meter point.
pub const TENTHS: u16 = 10;
/// A full meter, in tenths.
pub const FULL: u16 = 100 * TENTHS;

impl Needs {
    /// A fresh citizen's meters.
    #[must_use]
    pub fn at_start(params: &Params) -> Self {
        let start = u16::from(params.needs.meter_start) * TENTHS;
        Needs {
            food: start,
            shelter: start,
            comfort: start,
            low_food_ticks_this_cycle: 0,
            consecutive_hardship_cycles: 0,
        }
    }

    /// Food meter in points (0..=100) for display.
    #[must_use]
    pub fn food_points(&self) -> f64 {
        f64::from(self.food) / f64::from(TENTHS)
    }
}

/// The effort that scales Food decay this tick: the highest effort among
/// allocations with hours, Normal when idle (QUESTIONS Q19).
#[must_use]
pub fn effort_for_decay(citizen: &Citizen) -> Effort {
    citizen
        .labor
        .allocations
        .iter()
        .filter(|a| a.hours > 0)
        .map(|a| a.effort)
        .max()
        .unwrap_or(Effort::Normal)
}

/// Scale a per-tick decay (in points) by a multiplier, in tenths, exactly.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn tenths(points: u8, mult: f64) -> u16 {
    // Multipliers in config have at most one decimal, so this is exact.
    (f64::from(points) * mult * f64::from(TENTHS)).round() as u16
}

/// The output multiplier a citizen carries into the next tick, with its Explain
/// (TDD §5.5 phase 5): the Food term is linear from 1.0 at `food_full_output_meter`
/// down to `output_floor` at 0 (`output_floor_destitute` when destitute), times
/// `output_mult_unhoused` when unhoused. Comfort has no effect unless
/// `comfort_affects_output` (T7; not implemented).
#[must_use]
pub fn output_multiplier(
    needs: &Needs,
    housed: bool,
    destitute: bool,
    params: &Params,
) -> (f64, Explain) {
    let p = &params.needs;
    let floor = if destitute {
        p.output_floor_destitute
    } else {
        p.output_floor
    };
    let full = f64::from(p.food_full_output_meter);
    let food = needs.food_points();
    let food_term = if food >= full {
        1.0
    } else {
        floor + (1.0 - floor) * (food / full)
    };
    let shelter_term = if housed { 1.0 } else { p.output_mult_unhoused };
    let result = food_term * shelter_term;
    let explain = Explain::new(RuleId::OutputMultiplier, "food_term * shelter_term", result)
        .input("food", food)
        .input("food_full_output_meter", full)
        .input("floor", floor)
        .input("food_term", food_term)
        .input("shelter_term", shelter_term);
    (result, explain)
}

/// Phase 5: eat, consume Wares, decay, clamp, and set next tick's output multiplier.
/// Dormant citizens are skipped entirely.
pub fn phase_5_needs(b: &mut TickBuilder) {
    let params = b.world.params.clone();
    let p = &params.needs;
    let ids: Vec<CitizenId> = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| c.id)
        .collect();
    for id in ids {
        let c = b.world.citizens.get_mut(&id).expect("citizen exists");
        let effort = effort_for_decay(c);
        let housed = c.household.dwelling.is_some();
        let destitute = c.flags.destitute;
        let mut food = i32::from(c.needs.food);
        let mut shelter = i32::from(c.needs.shelter);
        let mut comfort = i32::from(c.needs.comfort);

        // Eat one Food if any is on hand.
        let mut ate = 0u32;
        if let Some(have) = c.household.pantry.get_mut(&Good::Food)
            && *have > 0
        {
            *have -= 1;
            ate = 1;
            food += i32::from(u16::from(p.food_meter_per_unit) * TENTHS);
        }
        // Consume one Wares when Comfort can absorb it.
        let mut wares = 0u32;
        let absorb = i32::from(FULL) - i32::from(u16::from(p.comfort_per_wares) * TENTHS);
        if comfort <= absorb
            && let Some(have) = c.household.pantry.get_mut(&Good::Wares)
            && *have > 0
        {
            *have -= 1;
            wares = 1;
            comfort += i32::from(u16::from(p.comfort_per_wares) * TENTHS);
        }
        c.household.pantry.retain(|_, q| *q > 0);

        // Decay.
        let food_mult = params.labor.effort_food_decay_mult.get(effort);
        food -= i32::from(tenths(p.food_decay_per_tick, food_mult));
        if housed {
            shelter += i32::from(u16::from(p.shelter_recovery_per_tick) * TENTHS);
        } else {
            shelter -= i32::from(u16::from(p.shelter_decay_unhoused) * TENTHS);
        }
        let comfort_mult = if housed {
            1.0
        } else {
            p.comfort_decay_unhoused_mult
        };
        comfort -= i32::from(tenths(p.comfort_decay_per_tick, comfort_mult));
        if destitute {
            comfort = 0;
        }

        // Clamp and store.
        let clamp = |v: i32| u16::try_from(v.clamp(0, i32::from(FULL))).unwrap_or(0);
        c.needs.food = clamp(food);
        c.needs.shelter = clamp(shelter);
        c.needs.comfort = clamp(comfort);
        if c.needs.food < u16::from(p.hardship_food_threshold) * TENTHS {
            c.needs.low_food_ticks_this_cycle = c.needs.low_food_ticks_this_cycle.saturating_add(1);
        }
        let (mult, _) = output_multiplier(&c.needs, housed, destitute, &params);
        c.labor.output_mult = mult;

        // Ledger: consumption leaves the economy.
        crate::ledger::LedgerMeta::add(&mut b.world.ledger_meta.consumed, Good::Food, ate);
        crate::ledger::LedgerMeta::add(&mut b.world.ledger_meta.consumed, Good::Wares, wares);
        let entry = b.consumed.entry(id).or_insert((0, 0));
        entry.0 += ate;
        entry.1 += wares;
    }
}

/// Step 8h: hardship for the cycle just ended, destitution, and next cycle's
/// budget from fatigue debt (T20, QUESTIONS Q3).
pub fn cycle_end_8h_hardship_and_fatigue(b: &mut TickBuilder) {
    let params = b.world.params.clone();
    let ticks_per_cycle = u8::try_from(params.time.ticks_per_cycle).unwrap_or(u8::MAX);
    let cycle = b.cycle;
    let ids: Vec<CitizenId> = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| c.id)
        .collect();
    for id in ids {
        let c = b.world.citizens.get_mut(&id).expect("citizen exists");
        let in_hardship = c.needs.low_food_ticks_this_cycle >= ticks_per_cycle;
        c.needs.low_food_ticks_this_cycle = 0;
        if in_hardship {
            c.needs.consecutive_hardship_cycles += 1;
        } else {
            c.needs.consecutive_hardship_cycles = 0;
        }
        let destitute = c.needs.consecutive_hardship_cycles
            >= u32::from(params.needs.destitution_hardship_cycles);

        // Fatigue debt for next cycle.
        let high_effort_debt = if c.labor.consecutive_high_effort_cycles
            > params.labor.high_effort_debt_after_cycles
        {
            params.labor.high_effort_debt_hours
        } else {
            0
        };
        let hardship_debt = if in_hardship {
            params.labor.hardship_debt_hours
        } else {
            0
        };
        let debt = (high_effort_debt + hardship_debt).min(params.labor.fatigue_debt_cap_hours);
        c.labor.fatigue_debt = debt;
        c.labor.budget = params.labor.base_budget_hours.saturating_sub(debt);

        let was_hardship = c.flags.in_hardship;
        let was_destitute = c.flags.destitute;
        let mut events = Vec::new();
        if in_hardship && !was_hardship {
            events.push(Event::HardshipBegan { citizen: id, cycle });
        }
        if !in_hardship && was_hardship {
            events.push(Event::HardshipEnded { citizen: id, cycle });
        }
        if destitute && !was_destitute {
            events.push(Event::DestitutionBegan { citizen: id, cycle });
        }
        if !destitute && was_destitute {
            events.push(Event::DestitutionEnded { citizen: id, cycle });
        }
        for e in events {
            b.emit(e);
        }
    }
}
