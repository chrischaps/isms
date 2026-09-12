//! The tick pipeline (TDD §5.5, Appendix B). Ten phases in a fixed, documented
//! order; each is a function so it can be unit-tested alone. `tick` never
//! mutates the caller's `World`: it works on a scratch clone inside
//! `TickBuilder` so mid-tick commands see each other's effects, and returns the
//! events for the caller to persist and apply.

use crate::apply::apply;
use crate::event::{CitizenDelta, CycleAggregates, Event, WorkplaceDelta};
use crate::ids::{Cycle, Epoch, Tick};
use crate::kinds::CitizenKind;
use crate::rules::Rules;
use crate::world::{EpochEndReason, World};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeMap;

/// What a tick needs besides the world: which tick, and the seed for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickInput {
    pub tick: Tick,
    pub seed: [u8; 32],
}

impl TickInput {
    /// The input for the world's next tick: seed = `blake3(society_seed || epoch || tick)`.
    #[must_use]
    pub fn next_for(world: &World) -> Self {
        let tick = world.meta.tick;
        TickInput {
            tick,
            seed: derive_seed(world.meta.seed, world.meta.epoch, tick),
        }
    }
}

/// TDD §5.9: one seed per (society, epoch, tick).
#[must_use]
pub fn derive_seed(society_seed: u64, epoch: Epoch, tick: Tick) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(&society_seed.to_le_bytes());
    h.update(&epoch.to_le_bytes());
    h.update(&tick.to_le_bytes());
    *h.finalize().as_bytes()
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TickError {
    #[error("the epoch has ended; start a new epoch before ticking")]
    EpochEnded,
    #[error("expected tick {expected}, got {got}")]
    WrongTick { expected: Tick, got: Tick },
}

/// Scratch state for one tick.
pub struct TickBuilder<'r> {
    pub world: World,
    pub rules: &'r Rules,
    pub rng: ChaCha8Rng,
    pub tick: Tick,
    pub cycle: Cycle,
    pub events: Vec<Event>,
    pub citizen_deltas: BTreeMap<crate::ids::CitizenId, CitizenDelta>,
    pub workplace_deltas: BTreeMap<crate::ids::WorkplaceId, WorkplaceDelta>,
    /// (Food eaten, Wares consumed) per citizen this tick, for the deltas.
    pub consumed: BTreeMap<crate::ids::CitizenId, (u32, u32)>,
}

impl std::fmt::Debug for TickBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TickBuilder")
            .field("tick", &self.tick)
            .field("cycle", &self.cycle)
            .field("events", &self.events.len())
            .finish_non_exhaustive()
    }
}

impl<'r> TickBuilder<'r> {
    fn new(world: &World, rules: &'r Rules, input: TickInput) -> Self {
        TickBuilder {
            world: world.clone(),
            rules,
            rng: ChaCha8Rng::from_seed(input.seed),
            tick: input.tick,
            cycle: world.cycle_of(input.tick),
            events: Vec::new(),
            citizen_deltas: BTreeMap::new(),
            workplace_deltas: BTreeMap::new(),
            consumed: BTreeMap::new(),
        }
    }

    /// Record a discrete event and apply it to the scratch world so later
    /// phases see its effect.
    pub fn emit(&mut self, event: Event) {
        apply(&mut self.world, &event);
        self.events.push(event);
    }

    /// Non-dormant citizens in a per-tick shuffled order (phase 2). Public so
    /// tests can observe that the seed changes the order.
    #[must_use]
    pub fn shuffled_active_citizens(&mut self) -> Vec<crate::ids::CitizenId> {
        let mut ids: Vec<_> = self
            .world
            .citizens
            .values()
            .filter(|c| !c.dormant)
            .map(|c| c.id)
            .collect();
        ids.shuffle(&mut self.rng);
        ids
    }

    #[must_use]
    pub fn is_cycle_end(&self) -> bool {
        self.world.is_cycle_end(self.tick)
    }

    #[must_use]
    pub fn active_humans(&self) -> u32 {
        u32::try_from(
            self.world
                .citizens
                .values()
                .filter(|c| c.kind == CitizenKind::Human && !c.dormant)
                .count(),
        )
        .unwrap_or(u32::MAX)
    }
}

/// Resolve one tick. Returns the discrete events in generation order followed by
/// `TickResolved` as the commit marker.
pub fn tick(world: &World, rules: &Rules, input: TickInput) -> Result<Vec<Event>, TickError> {
    if world.meta.epoch_ended.is_some() {
        return Err(TickError::EpochEnded);
    }
    if input.tick != world.meta.tick {
        return Err(TickError::WrongTick {
            expected: world.meta.tick,
            got: input.tick,
        });
    }
    let mut b = TickBuilder::new(world, rules, input);
    phase_1_open(&mut b);
    phase_2_standing_plans(&mut b);
    phase_3_labor(&mut b);
    phase_4_production(&mut b);
    phase_5_needs(&mut b);
    phase_6_markets(&mut b);
    phase_7_contracts(&mut b);
    if b.is_cycle_end() {
        phase_8_cycle_end(&mut b);
    }
    phase_9_epoch_checks(&mut b);
    Ok(phase_10_emit(b))
}

/// Start epoch `epoch` for this society as events (ADR-0003). Epoch 0's seeding
/// and later epochs' material reset are filled in by S0.12 and S0.13.
#[must_use]
pub fn start_epoch(world: &World, _rules: &Rules, epoch: Epoch) -> Vec<Event> {
    debug_assert!(epoch == 0 || world.meta.epoch_ended.is_some());
    vec![Event::EpochStarted { epoch }]
}

/// The phase-2 order a tick would use, for tests of seed sensitivity.
#[must_use]
pub fn shuffle_order_for_test(
    world: &World,
    rules: &Rules,
    input: TickInput,
) -> Vec<crate::ids::CitizenId> {
    let mut b = TickBuilder::new(world, rules, input);
    b.shuffled_active_citizens()
}

// ---------------------------------------------------------------------------
// Phases. Bodies arrive with their cards; the order is fixed here.

/// 1. Open: derive cycle and epoch position, seed the RNG (done in `new`).
fn phase_1_open(_b: &mut TickBuilder) {}

/// 2. Standing plans, in shuffled order (S0.9 fills the executor).
fn phase_2_standing_plans(b: &mut TickBuilder) {
    let _order = b.shuffled_active_citizens();
}

/// 3. Labor hours and multipliers (S0.6).
fn phase_3_labor(_b: &mut TickBuilder) {}

/// 4. Production and attribution (S0.6).
fn phase_4_production(_b: &mut TickBuilder) {}

/// 5. Consumption and needs (S0.5).
fn phase_5_needs(b: &mut TickBuilder) {
    crate::needs::phase_5_needs(b);
}

/// 6. Markets, store, state stock (S0.8, S0.15, S0.16).
fn phase_6_markets(_b: &mut TickBuilder) {}

/// 7. Evictions and defaults from last cycle's misses (S0.11).
fn phase_7_contracts(_b: &mut TickBuilder) {}

/// 8. Cycle end, in the TDD's exact sub-order.
fn phase_8_cycle_end(b: &mut TickBuilder) {
    cycle_end_8a_payroll(b);
    cycle_end_8b_tax_and_provision(b);
    cycle_end_8c_credit_installments(b);
    cycle_end_8d_rent(b);
    cycle_end_8e_dividends(b);
    cycle_end_8f_depreciation(b);
    cycle_end_8g_skill_decay(b);
    cycle_end_8h_hardship_and_fatigue(b);
    cycle_end_8i_norms_ledger(b);
    cycle_end_8j_votes_and_vacancies(b);
    cycle_end_8k_dormancy(b);
    cycle_end_8l_householder_fill(b);
    cycle_end_8m_aggregates(b);
}

fn cycle_end_8a_payroll(_b: &mut TickBuilder) {}
fn cycle_end_8b_tax_and_provision(_b: &mut TickBuilder) {}
fn cycle_end_8c_credit_installments(_b: &mut TickBuilder) {}
fn cycle_end_8d_rent(_b: &mut TickBuilder) {}
fn cycle_end_8e_dividends(_b: &mut TickBuilder) {}
fn cycle_end_8f_depreciation(_b: &mut TickBuilder) {}
fn cycle_end_8g_skill_decay(_b: &mut TickBuilder) {}
fn cycle_end_8h_hardship_and_fatigue(b: &mut TickBuilder) {
    crate::needs::cycle_end_8h_hardship_and_fatigue(b);
}
fn cycle_end_8i_norms_ledger(_b: &mut TickBuilder) {}
fn cycle_end_8j_votes_and_vacancies(_b: &mut TickBuilder) {}
fn cycle_end_8k_dormancy(_b: &mut TickBuilder) {}
fn cycle_end_8l_householder_fill(_b: &mut TickBuilder) {}

/// 8m. Per-cycle aggregates and the collapse counter (S0.13 fills the metrics).
fn cycle_end_8m_aggregates(b: &mut TickBuilder) {
    let active_humans = b.active_humans();
    let householders = u32::try_from(
        b.world
            .citizens
            .values()
            .filter(|c| c.kind == CitizenKind::Householder)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let population = u32::try_from(b.world.citizens.len()).unwrap_or(u32::MAX);
    let low_population_cycles = if active_humans < b.world.params.population.floor {
        b.world.meta.low_population_cycles + 1
    } else {
        0
    };
    b.emit(Event::CycleClosed {
        cycle: b.cycle,
        aggregates: CycleAggregates {
            population,
            active_humans,
            householders,
        },
        low_population_cycles,
    });
}

/// 9. Epoch checks: collapse (only when enabled) and the scheduled end.
fn phase_9_epoch_checks(b: &mut TickBuilder) {
    if !b.is_cycle_end() {
        return;
    }
    let p = &b.world.params;
    let collapse = p.population.collapse_enabled
        && b.world.meta.low_population_cycles >= p.population.collapse_cycles;
    let scheduled = b.cycle + 1 >= p.time.epoch_cycles;
    let reason = if collapse {
        Some(EpochEndReason::Collapse)
    } else if scheduled {
        Some(EpochEndReason::Scheduled)
    } else {
        None
    };
    if let Some(reason) = reason {
        b.emit(Event::EpochEnded {
            reason,
            cycle: b.cycle,
        });
    }
}

/// 10. Emit: discrete events in order, then `TickResolved`.
fn phase_10_emit(b: TickBuilder) -> Vec<Event> {
    let mut events = b.events;
    // Continuous state of every non-dormant citizen, as it now stands in the scratch world.
    let citizen_deltas = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| {
            let (food_eaten, wares_consumed) = b.consumed.get(&c.id).copied().unwrap_or((0, 0));
            CitizenDelta {
                citizen: c.id,
                needs: c.needs.clone(),
                food_eaten,
                wares_consumed,
                output_mult: c.labor.output_mult,
                budget: c.labor.budget,
                fatigue_debt: c.labor.fatigue_debt,
                consecutive_high_effort_cycles: c.labor.consecutive_high_effort_cycles,
                skill: c.labor.skill.clone(),
            }
        })
        .collect();
    events.push(Event::TickResolved {
        tick: b.tick,
        cycle: b.cycle,
        price_index: None,
        citizen_deltas,
        workplace_deltas: b.workplace_deltas.into_values().collect(),
    });
    events
}
