//! `Params`: every tunable constant (GDD Appendix A, TDD Appendix A), loaded from
//! `presets/_base.toml` with the preset's `[params]` overlaid. No field has a
//! default: a missing value fails to load (TDD S0.2 done gate).

use crate::kinds::{EffortTable, Good, Product, WorkplaceKind};
use crate::money::{Money, credits, credits_map};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub time: TimeParams,
    pub labor: LaborParams,
    pub skill: SkillParams,
    pub needs: NeedsParams,
    /// Per-good pantry caps; a good absent here is uncapped (QUESTIONS Q12).
    pub pantry: BTreeMap<Good, u32>,
    pub capital: CapitalParams,
    /// Land slots per workplace kind; a kind absent here is unlimited (GDD §4.4).
    pub land: BTreeMap<WorkplaceKind, u32>,
    pub founding: FoundingParams,
    pub recipes: BTreeMap<WorkplaceKind, Recipe>,
    /// Reference basket weights for the price index and real output (TDD §5.7).
    pub basket: BTreeMap<Product, f64>,
    pub money: MoneyParams,
    pub market: MarketParams,
    pub population: PopulationParams,
    pub householder: HouseholderParams,
    pub contracts: ContractParams,
    pub metrics: MetricsParams,
    pub governance: GovernanceParams,
    /// Monitoring noise σ per A7 level.
    pub monitoring: MonitoringSigma,
    /// Workplaces seeded on the land at epoch start, per kind (legacy firms, collective
    /// workplaces, or state enterprises depending on the constitution).
    pub seeded_workplaces: BTreeMap<WorkplaceKind, u32>,
    /// Dwellings seeded at epoch start (owned by the seeded Builders, or the society).
    pub initial_dwellings: u32,
    pub seeding: SeedingParams,
    pub coop: CoopParams,
    pub bank: BankParams,
    pub union: UnionParams,
    /// The simulator's scripted assembly (S2.4); nothing in the engine reads it.
    #[serde(default)]
    pub sim: SimParams,
}

/// The headless simulator's scripted humans (S2.4; TDD §18.5). Sim only: the
/// server never seeds them and the engine never reads these.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SimParams {
    /// Scripted human citizens `isms-sim` joins before the first epoch; they
    /// keep the assembly and its offices real in `sim-check`. Zero for the
    /// presets without an assembly.
    pub assembly_size: u32,
    /// How far each cycle's split proposal moves toward the scarcest sink.
    pub split_nudge: f64,
    /// The assembly honors the top contributor every this many cycles.
    pub honor_every_cycles: u32,
}

/// Union dues and strike pay (GDD §6.4; S0.17d, Q101).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnionParams {
    #[serde(rename = "dues_credits_per_cycle", with = "crate::money::credits")]
    pub dues_per_cycle: Money,
    #[serde(
        rename = "strike_pay_credits_per_cycle",
        with = "crate::money::credits"
    )]
    pub strike_pay_per_cycle: Money,
}

/// The Public Investment Bank's lending formula (GDD §6.5; S0.17c, Q94).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankParams {
    pub rate_per_cycle_bp: u32,
    #[serde(rename = "max_loan_credits", with = "crate::money::credits")]
    pub max_loan: Money,
    pub max_term_cycles: u32,
}

/// Cooperative defaults (GDD §6.5; S0.17b).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoopParams {
    /// How a new coop splits its surplus until it decides otherwise.
    pub default_share_rule: crate::world::ShareRule,
    /// A coop's steward buys no Machine beyond this many per working member (Q98).
    pub max_machines_per_member: u32,
}

/// What the seeded (legacy) orgs start with besides their treasury (ADR-0004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedingParams {
    /// Goods seeded into each legacy org's inventory, per workplace kind, so that
    /// day one has stock to sell before the first production lands (Q45).
    pub legacy_inventory: BTreeMap<WorkplaceKind, BTreeMap<Good, u32>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeParams {
    pub ticks_per_cycle: u32,
    pub epoch_cycles: u32,
    pub tick_seconds: u64,
    /// Minutes the closing-statements window stays open after the epoch ends
    /// (S1.15, GDD 11.5: 48 hours). The server keeps the window; the engine
    /// never reads it. Overridable per society at seed, like `tick_seconds`.
    pub closing_window_minutes: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaborParams {
    pub base_budget_hours: u8,
    /// Relative staffing weights per workplace kind for the balancing rule that
    /// places a citizen where labor is scarcest (norm and assigned systems, S0.15c).
    pub balance_weights: BTreeMap<WorkplaceKind, u32>,
    /// Skill points per wage grade band (wage-scale systems, Q73).
    pub skill_band: u8,
    pub max_workplaces: u8,
    pub max_workers_per_workplace: u32,
    pub effort_output_mult: EffortTable<f64>,
    pub effort_food_decay_mult: EffortTable<f64>,
    pub high_effort_debt_after_cycles: u8,
    pub high_effort_debt_hours: u8,
    pub hardship_debt_hours: u8,
    pub fatigue_debt_cap_hours: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillParams {
    pub k: f64,
    pub h0: f64,
    pub max: u8,
    pub decay_per_idle_cycles: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedsParams {
    pub food_decay_per_tick: u8,
    pub food_meter_per_unit: u8,
    pub shelter_decay_unhoused: u8,
    pub shelter_recovery_per_tick: u8,
    pub comfort_decay_per_tick: u8,
    pub comfort_per_wares: u8,
    pub comfort_decay_unhoused_mult: f64,
    pub hardship_food_threshold: u8,
    pub destitution_hardship_cycles: u8,
    pub output_floor: f64,
    pub output_floor_destitute: f64,
    pub output_mult_unhoused: f64,
    pub food_full_output_meter: u8,
    pub comfort_affects_output: bool,
    pub meter_start: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapitalParams {
    pub capital_log_coeff: f64,
    pub machine_depreciation_per_cycle: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundingParams {
    pub materials: u32,
    /// Shares a founded firm issues to its founder (a unit choice; 100% either way).
    pub initial_shares: u32,
}

/// What a workplace kind consumes and produces (TDD §5.7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub produces: Product,
    pub consumes: BTreeMap<Good, u32>,
    /// Units per worker-hour at skill mult 1.0 with no machines.
    pub base_rate: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoneyParams {
    #[serde(rename = "endowment_credits", with = "credits")]
    pub endowment: Money,
    #[serde(rename = "founding_cost_money_credits", with = "credits")]
    pub founding_cost_money: Money,
    #[serde(rename = "legacy_wage_credits", with = "credits")]
    pub legacy_wage: Money,
    #[serde(rename = "legacy_rent_credits", with = "credits")]
    pub legacy_rent: Money,
    #[serde(rename = "legacy_treasury_credits", with = "credits")]
    pub legacy_treasury: Money,
    #[serde(rename = "start_prices_credits", with = "credits_map")]
    pub start_prices: BTreeMap<Good, Money>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketParams {
    pub order_expiry_cycles: u32,
    pub rate_limit: RateLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimit {
    pub per_second: u32,
    pub burst: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationParams {
    pub floor: u32,
    /// Humans the seed expects to join: an epoch's start fills the floor to
    /// `floor - max(active humans, expected_humans)` so their places stand
    /// empty rather than emigrate at the first cycle end (E-4, Q161). Zero
    /// in every preset; a lab seed sets it with `seed --expect-humans N`.
    pub expected_humans: u32,
    pub comfortable_min: u32,
    pub comfortable_max: u32,
    pub cap: u32,
    pub dormancy_absent_cycles: u32,
    pub office_vacancy_absent_cycles: u32,
    pub quorum_fraction: f64,
    pub collapse_cycles: u32,
    pub collapse_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholderParams {
    pub keep_food_at_least: u32,
    pub wares_comfort_below: u8,
    pub wares_balance_living_cost_mult: f64,
    pub save_fraction: f64,
    pub rent_income_fraction_max: f64,
    pub sell_surplus: bool,
    pub legacy_markup: f64,
    /// How far a legacy firm's ask markup moves per cycle when its shelf
    /// answers (E-1): down a step when the closing stock grew on the last
    /// close, up a step when it closed empty after producing; clamped to
    /// `legacy_markup_min..=legacy_markup_max`.
    pub legacy_markup_step: f64,
    pub legacy_markup_min: f64,
    pub legacy_markup_max: f64,
    pub legacy_machine_buy_payroll_mult: f64,
    pub legacy_hire_inventory_cycles_cap: u32,
    pub legacy_offer_max_hours: u8,
    pub legacy_offer_notice_cycles: u32,
    pub living_cost_food: u32,
    pub living_cost_wares: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractParams {
    pub lease_grace_cycles: u32,
    /// A destitute citizen may not sign contracts longer than this (GDD Q3, Q26).
    pub long_contract_cycles: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsParams {
    pub housed_tick_weight: f64,
    pub mobility_window_cycles: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernanceParams {
    /// Open proposals one citizen may hold at once (S2.1).
    pub open_proposals_per_citizen: u32,
    /// Cycles an office may stand short of holders before the engine says
    /// so once per cycle with `OfficeUnfilled` (S2.2; GDD 8.3).
    pub unfilled_office_headline_cycles: u32,
    pub coordinator_term_cycles: u32,
    pub committee_term_cycles: u32,
    pub legislature_term_cycles: u32,
    pub sim_planner_target_growth: f64,
    /// Next target = max(target, output x this) when a workplace overfulfils (Q70).
    pub ratchet_mult: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitoringSigma {
    pub high: f64,
    pub medium: f64,
    pub low: f64,
}

impl Params {
    /// Ticks in one epoch: `ticks_per_cycle × epoch_cycles`.
    #[must_use]
    pub const fn ticks_per_epoch(&self) -> u32 {
        self.time.ticks_per_cycle * self.time.epoch_cycles
    }
}
