//! The tick pipeline (TDD §5.5, Appendix B). Ten phases in a fixed, documented
//! order; each is a function so it can be unit-tested alone. `tick` never
//! mutates the caller's `World`: it works on a scratch clone inside
//! `TickBuilder` so mid-tick commands see each other's effects, and returns the
//! events for the caller to persist and apply.

use crate::apply::apply;
use crate::event::{CitizenDelta, Event, WorkplaceDelta};
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
    /// Phase 3's result, consumed by phase 4.
    pub labor: BTreeMap<crate::ids::WorkplaceId, Vec<crate::labor::WorkerTick>>,
    /// Phase 6's market statistics for `TickResolved`.
    pub vwap: Vec<(crate::world::Instrument, crate::money::Money)>,
    pub price_index: Option<f64>,
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
            labor: BTreeMap::new(),
            vwap: Vec::new(),
            price_index: None,
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
pub fn start_epoch(world: &World, rules: &Rules, epoch: Epoch) -> Vec<Event> {
    debug_assert!(epoch == 0 || world.meta.epoch_ended.is_some());
    crate::seeding::start_epoch(world, rules, epoch)
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
    let order = b.shuffled_active_citizens();
    crate::plan::phase_2_standing_plans(b, &order);
}

/// 3. Labor hours and multipliers (S0.6).
fn phase_3_labor(b: &mut TickBuilder) {
    b.labor = crate::labor::phase_3_labor(&b.world);
}

/// 4. Production and attribution (S0.6).
fn phase_4_production(b: &mut TickBuilder) {
    let labor = std::mem::take(&mut b.labor);
    crate::labor::phase_4_production(b, &labor);
}

/// 5. Consumption and needs (S0.5).
fn phase_5_needs(b: &mut TickBuilder) {
    crate::needs::phase_5_needs(b);
}

/// 6. Markets, store, state stock (S0.8, S0.15, S0.16).
fn phase_6_markets(b: &mut TickBuilder) {
    if b.rules.capabilities.order_books {
        let (vwap, index) = crate::market::phase_6_markets(b);
        b.vwap = vwap;
        b.price_index = index;
    } else if b.rules.capabilities.common_store {
        crate::store::phase_6_store(b);
    } else if b.rules.capabilities.administered_prices {
        crate::state_store::phase_6_state_store(b);
    }
}

/// 7. Evictions and defaults from last cycle's misses (S0.11).
fn phase_7_contracts(b: &mut TickBuilder) {
    crate::housing::phase_7_evictions(b);
    crate::credit::phase_7_defaults(b);
}

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

fn cycle_end_8a_payroll(b: &mut TickBuilder) {
    crate::employment::cycle_end_8a_payroll(b);
    crate::planning::cycle_end_8a_scale_payroll(b);
    crate::coop::cycle_end_8a_share_out(b);
    crate::union::cycle_end_8a_unions(b);
}
/// 8b. Tax on the cycle's income and the resulting transfers and provision
/// floors (TDD 5.5): the income tax and the money floor (tax-transfer), then
/// society-owned dwellings for the unhoused, the Common Store's surplus shares,
/// and the state's Food ration.
fn cycle_end_8b_tax_and_provision(b: &mut TickBuilder) {
    crate::tax::cycle_end_8b_tax(b);
    crate::housing::cycle_end_8b_assign_dwellings(b);
    if b.rules.capabilities.common_store {
        crate::store::cycle_end_8b_surplus_shares(b);
    }
    crate::state_store::cycle_end_8b_provision_ration(b);
    crate::bank::cycle_end_8b_levy_floor_and_lending(b);
}
fn cycle_end_8c_credit_installments(b: &mut TickBuilder) {
    crate::credit::cycle_end_8c_credit_installments(b);
}
fn cycle_end_8d_rent(b: &mut TickBuilder) {
    crate::housing::cycle_end_8d_rent(b);
}
fn cycle_end_8e_dividends(b: &mut TickBuilder) {
    crate::shares::cycle_end_8e_dividends(b);
}
fn cycle_end_8f_depreciation(b: &mut TickBuilder) {
    crate::orgs::cycle_end_8f_depreciation(b);
}
fn cycle_end_8g_skill_decay(b: &mut TickBuilder) {
    crate::labor::cycle_end_8g_skill_and_effort(b);
}
fn cycle_end_8h_hardship_and_fatigue(b: &mut TickBuilder) {
    crate::needs::cycle_end_8h_hardship_and_fatigue(b);
}
fn cycle_end_8i_norms_ledger(b: &mut TickBuilder) {
    crate::norms::cycle_end_8i_norms_ledger(b);
}
/// 8j. Votes close (S2.1): every proposal whose `closes_cycle` has come is
/// tallied and, where it passed, takes effect. Elections and vacancies are S2.2.
/// 8j. Proposals close first (a recall that carries empties a seat), then the
/// offices: term expiry, absence, the elections' close, re-runs, the open for
/// any vacancy, and the unfilled headline (S2.2).
fn cycle_end_8j_votes_and_vacancies(b: &mut TickBuilder) {
    crate::governance::cycle_end_8j_close_proposals(b);
    crate::offices::cycle_end_8j_offices(b);
}
fn cycle_end_8k_dormancy(b: &mut TickBuilder) {
    crate::plan::cycle_end_8k_dormancy(b);
}
fn cycle_end_8l_householder_fill(b: &mut TickBuilder) {
    crate::seeding::cycle_end_8l_householder_fill(b);
}

/// 8m. Per-cycle aggregates and the collapse counter.
fn cycle_end_8m_aggregates(b: &mut TickBuilder) {
    crate::metrics::cycle_end_8m_aggregates(b);
}

/// 9. Epoch checks: collapse (only when enabled) and the scheduled end.
fn phase_9_epoch_checks(b: &mut TickBuilder) {
    if !b.is_cycle_end() {
        return;
    }
    let p = &b.world.params;
    let epoch_cycles = p.time.epoch_cycles;
    let collapse = p.population.collapse_enabled
        && b.world.meta.low_population_cycles >= p.population.collapse_cycles;
    let scheduled = b.cycle + 1 >= epoch_cycles;
    let reason = if collapse {
        Some(EpochEndReason::Collapse)
    } else if scheduled {
        Some(EpochEndReason::Scheduled)
    } else {
        None
    };
    if let Some(reason) = reason {
        // The summary is frozen here, from the aggregates 8m just took and the
        // scratch world as it stands, so the archive replays byte for byte.
        let aggregates = b
            .events
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::CycleClosed { aggregates, .. } => Some(aggregates.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let summary = crate::metrics::epoch_summary(&b.world, aggregates);
        b.emit(Event::EpochEnded {
            reason,
            cycle: b.cycle,
            summary,
        });
    } else if b.cycle + 3 == epoch_cycles {
        // Two cycles remain (GDD 11.5). An epoch of two cycles or fewer never
        // announces (Q115): it has no cycle two before its last.
        b.emit(Event::EpochEnding {
            final_cycle: epoch_cycles - 1,
        });
    }
}

/// 10. Emit: discrete events in order, then `TickResolved`.
fn phase_10_emit(b: TickBuilder) -> Vec<Event> {
    let mut events = b.events;
    // Continuous state of every non-dormant citizen, as it now stands in the
    // scratch world. A citizen who ate this tick and then left (emigration at
    // 8l) still gets a delta, or the live pantry would keep the eaten unit.
    let citizen_deltas = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant || b.consumed.contains_key(&c.id))
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
                cycle: c.cycle.clone(),
            }
        })
        .collect();
    events.push(Event::TickResolved {
        tick: b.tick,
        cycle: b.cycle,
        price_index: b.price_index,
        vwap: b.vwap,
        citizen_deltas,
        workplace_deltas: b
            .world
            .workplaces
            .values()
            .map(|w| WorkplaceDelta {
                workplace: w.id,
                machine_wear: w.machine_wear,
                output_remainder: w.output_remainder,
                cycle_output: w.cycle_output,
                last_cycle_output: w.last_cycle_output,
                last_fulfillment: w.last_fulfillment,
                workers: w
                    .workers
                    .iter()
                    .map(|(c, a)| {
                        (
                            *c,
                            crate::event::WorkerCycle {
                                tick_hours: a.cycle_tick_hours,
                                attributed: a.cycle_attributed,
                            },
                        )
                    })
                    .collect(),
            })
            .collect(),
    });
    events
}
