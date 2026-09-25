//! `isms-sim`: the headless simulator (TDD §18.2 S0.13, GDD §17). Links
//! `isms-core` directly, never touches the network or a database, and runs
//! householders plus, where the preset asks for them, `sim.assembly_size`
//! scripted humans (S2.4). Its metrics are the engine's own `CycleAggregates`.
//!
//! The GDD §17 targets live here as `StabilityTargets` rather than in
//! `presets/*.toml`: they are the tuning gate the simulator applies to a
//! preset, not a rule the society runs by. Which targets apply is derived
//! from the preset's `Capabilities` (S0.14d): a price band only where order
//! books exist, and "stock-out" measured on whatever holds the society's Food.
//! In an administered society the System's scripted planner runs before the
//! householders each round (S0.16c); the scripted assembly runs between the
//! two (S2.4), so `sim-check commune` exercises quorum, offices and the
//! split vote with real humans on the rolls.

use isms_core::apply::apply;
use isms_core::capabilities::Capabilities;
use isms_core::config::{ConfigError, Preset, load_preset_with_overrides};
use isms_core::event::Event;
use isms_core::householder::run_round;
use isms_core::kinds::Good;
use isms_core::market::depth;
use isms_core::planner::{run_assembly_round, run_system_round};
use isms_core::rules::Rules;
use isms_core::tick::{TickInput, start_epoch, tick};
use isms_core::world::{Instrument, Side, World};
use serde::Serialize;
use std::path::Path;

pub mod detail;
pub mod observer;

pub use detail::{DetailOptions, DetailWriter};
pub use observer::{NoObserver, Observer};

/// The five presets, in the order `make sim-all` prints them.
pub const PRESETS: [&str; 5] = [
    "freeport",
    "commune",
    "directorate",
    "republic",
    "commonwealth",
];

/// One row of the metrics table: a `CycleClosed` plus a few readings of
/// wherever the society keeps its Food and money.
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub preset: String,
    pub seed: u64,
    pub epoch: u32,
    pub cycle: u32,
    pub population: u32,
    pub active_humans: u32,
    pub householders: u32,
    pub real_output: f64,
    pub median_wellbeing: f64,
    pub need_fulfillment_rate: f64,
    pub consumption_gini: f64,
    pub investment_share: f64,
    pub price_index: Option<f64>,
    pub mean_cycle_wage: f64,
    pub unemployed: u32,
    pub firm_count: u32,
    pub credit_outstanding_credits: f64,
    pub hardship_count: u32,
    pub food_ask_depth: u32,
    pub wares_ask_depth: u32,
    pub food_last_price: Option<f64>,
    pub rejected_commands: u32,
    /// Food a citizen could obtain at cycle end: resting asks in market
    /// societies, the Common Store's stock in moneyless ones, the state
    /// stock's in administered ones. Zero for a whole run of cycles is a
    /// stock-out whatever the system.
    pub food_available: u32,
    /// Food units requested from the society's store this cycle and not served.
    pub food_unfilled: u32,
    /// Whether this cycle counts as a Food stock-out for its system: no resting
    /// asks (market), or unserved requests (store and state stock).
    pub stocked_out: bool,
    pub store_food: u32,
    pub state_food: u32,
    pub treasury_credits: f64,
    pub till_credits: f64,
    pub plan_fulfillment: Option<f64>,
    pub rations_issued: u64,
    pub contribution_gini: f64,
    pub tax_collected_credits: f64,
    pub floor_paid_credits: f64,
    pub coop_surplus_per_member: f64,
    pub mean_tenure_cycles: f64,
    pub bank_pool_credits: f64,
    pub bank_loans_credits: f64,
    pub materials_produced: u64,
    pub materials_to_machines: u64,
}

/// What to run.
#[derive(Clone, Debug)]
pub struct RunSpec {
    pub preset: String,
    pub epochs: u32,
    pub seed: u64,
    /// `params.x.y = value` overrides applied to the merged TOML before loading.
    pub overrides: Vec<(String, toml::Value)>,
}

impl RunSpec {
    #[must_use]
    pub fn new(preset: &str, epochs: u32, seed: u64) -> Self {
        RunSpec {
            preset: preset.to_owned(),
            epochs,
            seed,
            overrides: Vec::new(),
        }
    }
}

/// The outcome of one run.
#[derive(Debug)]
pub struct RunResult {
    pub rows: Vec<Row>,
    pub events: u64,
    pub rejected: u32,
    pub world: World,
    /// The society's derived capabilities, so callers can pick targets
    /// without reloading the preset.
    pub capabilities: Capabilities,
}

fn ask_depth(world: &World, good: Good) -> u32 {
    depth(world, Instrument::Good(good), Side::Ask)
        .iter()
        .map(|(_, q)| *q)
        .sum()
}

fn store_stock(world: &World, good: Good) -> u32 {
    world
        .store
        .as_ref()
        .and_then(|s| s.stock.get(&good).copied())
        .unwrap_or(0)
}

fn state_stock(world: &World, good: Good) -> u32 {
    world
        .state_stock
        .as_ref()
        .and_then(|s| s.stock.get(&good).copied())
        .unwrap_or(0)
}

/// Food units requested from the store in these events, less the units served
/// by need (surplus shares are not requests).
fn unfilled_food(events: &[Event]) -> u32 {
    let mut requested = 0u32;
    let mut served = 0u32;
    for e in events {
        match e {
            Event::StoreDrawRequested {
                good: Good::Food,
                qty,
                ..
            }
            | Event::StateStoreRequested {
                good: Good::Food,
                qty,
                ..
            } => requested += qty,
            Event::Drew { goods, explain, .. }
                if explain.rule != isms_core::explain::RuleId::StoreSurplusShare =>
            {
                served += goods.get(&Good::Food).copied().unwrap_or(0);
            }
            Event::StateStoreSold {
                good: Good::Food,
                qty,
                ..
            } => served += qty,
            _ => {}
        }
    }
    requested.saturating_sub(served)
}

/// Where a citizen would get Food in this society (see `Row::food_available`).
#[must_use]
pub fn food_available(world: &World, caps: &Capabilities) -> u32 {
    if caps.common_store {
        store_stock(world, Good::Food)
    } else if caps.administered_prices {
        state_stock(world, Good::Food)
    } else {
        ask_depth(world, Good::Food)
    }
}

#[allow(clippy::too_many_arguments)]
fn make_row(
    spec: &RunSpec,
    epoch: u32,
    cycle: u32,
    a: &isms_core::event::CycleAggregates,
    world: &World,
    caps: &Capabilities,
    rejected: u32,
    unfilled: u32,
) -> Row {
    let available = food_available(world, caps);
    let stocked_out = if caps.order_books {
        available == 0
    } else {
        unfilled > 0
    };
    Row {
        preset: spec.preset.clone(),
        seed: spec.seed,
        epoch,
        cycle,
        population: a.population,
        active_humans: a.active_humans,
        householders: a.householders,
        real_output: a.real_output,
        median_wellbeing: a.median_wellbeing,
        need_fulfillment_rate: a.need_fulfillment_rate,
        consumption_gini: a.consumption_gini,
        investment_share: a.investment_share,
        price_index: a.price_index,
        mean_cycle_wage: a.mean_cycle_wage,
        unemployed: a.unemployed,
        firm_count: a.firm_count,
        credit_outstanding_credits: a.credit_outstanding.as_credits_f64(),
        hardship_count: a.hardship_count,
        food_ask_depth: ask_depth(world, Good::Food),
        wares_ask_depth: ask_depth(world, Good::Wares),
        food_last_price: isms_core::market::last_price(world, Instrument::Good(Good::Food))
            .map(isms_core::money::Money::as_credits_f64),
        rejected_commands: rejected,
        food_available: available,
        food_unfilled: unfilled,
        stocked_out,
        store_food: store_stock(world, Good::Food),
        state_food: state_stock(world, Good::Food),
        treasury_credits: a.treasury.as_credits_f64(),
        till_credits: a.till.as_credits_f64(),
        plan_fulfillment: a.plan_fulfillment,
        rations_issued: a.rations_issued,
        contribution_gini: a.contribution_gini,
        tax_collected_credits: a.tax_collected.as_credits_f64(),
        floor_paid_credits: a.floor_paid.as_credits_f64(),
        coop_surplus_per_member: a.coop_surplus_per_member,
        mean_tenure_cycles: a.mean_tenure_cycles,
        bank_pool_credits: a.levy_pool.as_credits_f64(),
        bank_loans_credits: a.bank_loans_outstanding.as_credits_f64(),
        materials_produced: a.materials_produced,
        materials_to_machines: a.materials_to_machines,
    }
}

/// Run a scripted society for `epochs` epochs. Collapse is off (the
/// simulator's humans never leave). Between ticks the scripts run once each.
pub fn run(presets_dir: &Path, spec: &RunSpec) -> Result<RunResult, ConfigError> {
    run_with(presets_dir, spec, &mut NoObserver)
}

/// The scripted assembly joins before the first epoch is seeded, so the
/// householder fill counts them toward the floor and the society's dwellings
/// reach them (S2.4). Returns the events applied; a refusal is a preset bug.
fn seed_assembly(world: &mut World, rules: &Rules, preset: &str) -> Result<u64, ConfigError> {
    let mut events = 0;
    for i in 0..world.params.sim.assembly_size {
        let env = isms_core::command::Envelope::system(
            isms_core::command::Command::Join {
                handle: format!("A-{i}"),
                kind: isms_core::kinds::CitizenKind::Human,
            },
            0,
        );
        let joined = isms_core::command::handle(world, rules, &env).map_err(|e| {
            ConfigError::Constraint {
                name: preset.to_owned(),
                reason: format!("seeding the assembly: {e}"),
            }
        })?;
        for e in &joined {
            apply(world, e);
            events += 1;
        }
    }
    Ok(events)
}

/// `run` with an [`Observer`] called after each applied step (S0.14e). The
/// observer only reads; with `NoObserver` this is byte-for-byte `run`.
#[allow(clippy::too_many_lines)]
pub fn run_with(
    presets_dir: &Path,
    spec: &RunSpec,
    obs: &mut dyn Observer,
) -> Result<RunResult, ConfigError> {
    let mut overrides = spec.overrides.clone();
    overrides.push((
        "params.population.collapse_enabled".into(),
        toml::Value::Boolean(false),
    ));
    let preset: Preset = load_preset_with_overrides(presets_dir, &spec.preset, &overrides)?;
    let mut world = World::new(spec.seed, spec.seed, &preset);
    apply(
        &mut world,
        &Event::SocietyCreated {
            society_id: spec.seed,
            seed: spec.seed,
            preset: Box::new(preset),
        },
    );
    let rules = Rules::from_world(&world);
    let caps = rules.capabilities.clone();
    let mut rows = Vec::new();
    let mut events: u64 = 1 + seed_assembly(&mut world, &rules, &spec.preset)?;
    let mut rejected_total = 0u32;
    for epoch in 0..spec.epochs {
        let started = start_epoch(&world, &rules, epoch);
        for e in &started {
            apply(&mut world, e);
            events += 1;
        }
        obs.epoch_started(&world, epoch, &started);
        let mut rejected_cycle = 0u32;
        let mut unfilled_cycle = 0u32;
        loop {
            let now = world.meta.tick;
            // The System's script first (the sim's Committee), then the
            // assembly's humans, then the householders.
            let (system_events, system_rejected) = run_system_round(&mut world, &rules, now);
            events += system_events.len() as u64;
            let (assembly_events, assembly_rejected) = run_assembly_round(&mut world, &rules, now);
            events += assembly_events.len() as u64;
            let (householder_events, rejected) = run_round(&mut world, &rules, now);
            events += householder_events.len() as u64;
            let n = u32::try_from(rejected.len() + system_rejected.len() + assembly_rejected.len())
                .unwrap_or(u32::MAX);
            for r in &assembly_rejected {
                eprintln!(
                    "{} seed {} tick {now}: assembly {r}",
                    spec.preset, spec.seed
                );
            }
            for r in &rejected {
                eprintln!(
                    "{} seed {} tick {now}: householder {r:?}",
                    spec.preset, spec.seed
                );
            }
            // The observer sees the round as one slice, assembly first.
            let mut round = assembly_events;
            round.extend(householder_events);
            rejected_cycle += n;
            rejected_total += n;
            let input = TickInput::next_for(&world);
            let Ok(tick_events) = tick(&world, &rules, input) else {
                break;
            };
            let mut closed = None;
            for e in &tick_events {
                if let Event::CycleClosed {
                    cycle, aggregates, ..
                } = e
                {
                    closed = Some((*cycle, aggregates.clone()));
                }
            }
            unfilled_cycle += unfilled_food(&tick_events);
            for e in &tick_events {
                apply(&mut world, e);
            }
            events += tick_events.len() as u64;
            obs.tick_done(&world, epoch, now, &round, &tick_events);
            if let Some((cycle, a)) = closed {
                rows.push(make_row(
                    spec,
                    epoch,
                    cycle,
                    &a,
                    &world,
                    &caps,
                    rejected_cycle,
                    unfilled_cycle,
                ));
                obs.cycle_closed(&world, epoch, cycle, rows.last().expect("just pushed"));
                rejected_cycle = 0;
                unfilled_cycle = 0;
            }
            if world.meta.epoch_ended.is_some() {
                break;
            }
        }
    }
    obs.finished(&world);
    Ok(RunResult {
        rows,
        events,
        rejected: rejected_total,
        world,
        capabilities: caps,
    })
}

/// Per-epoch summary used by the console table and the stability checks.
#[derive(Clone, Debug, Serialize)]
pub struct EpochSummary {
    pub epoch: u32,
    pub cycles: usize,
    pub mean_need_fulfillment: f64,
    /// `None` when no cycle of the epoch had a price index (no order books).
    pub min_price_index: Option<f64>,
    pub max_price_index: Option<f64>,
    pub mean_gini: f64,
    /// The epoch's Materials-to-Machines share as a ratio of sums: a mean of
    /// per-cycle ratios lets one cycle with two units of Materials made and
    /// forty consumed (a ratio of 19) carry the epoch (Q100).
    pub mean_investment_share: f64,
    pub max_hardship: u32,
    pub max_unemployed: u32,
    /// Longest run of consecutive cycles flagged `stocked_out`.
    pub longest_food_stockout: u32,
    pub rejected_commands: u32,
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn summarize(rows: &[Row]) -> Vec<EpochSummary> {
    let mut out = Vec::new();
    let epochs: std::collections::BTreeSet<u32> = rows.iter().map(|r| r.epoch).collect();
    for epoch in epochs {
        let rs: Vec<&Row> = rows.iter().filter(|r| r.epoch == epoch).collect();
        let n = rs.len().max(1) as f64;
        let idx: Vec<f64> = rs.iter().filter_map(|r| r.price_index).collect();
        let mut longest = 0u32;
        let mut run = 0u32;
        for r in &rs {
            if r.stocked_out {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        out.push(EpochSummary {
            epoch,
            cycles: rs.len(),
            mean_need_fulfillment: rs.iter().map(|r| r.need_fulfillment_rate).sum::<f64>() / n,
            min_price_index: idx.iter().copied().reduce(f64::min),
            max_price_index: idx.iter().copied().reduce(f64::max),
            mean_gini: rs.iter().map(|r| r.consumption_gini).sum::<f64>() / n,
            mean_investment_share: {
                let made: u64 = rs.iter().map(|r| r.materials_produced).sum();
                let to_machines: u64 = rs.iter().map(|r| r.materials_to_machines).sum();
                if made == 0 {
                    0.0
                } else {
                    to_machines as f64 / made as f64
                }
            },
            max_hardship: rs.iter().map(|r| r.hardship_count).max().unwrap_or(0),
            max_unemployed: rs.iter().map(|r| r.unemployed).max().unwrap_or(0),
            longest_food_stockout: longest,
            rejected_commands: rs.iter().map(|r| r.rejected_commands).sum(),
        });
    }
    out
}

/// GDD §17 tuning targets for one preset. Every preset shares the
/// need-fulfillment, stock-out, Materials-sink and zero-rejection targets;
/// the price band applies only where prices exist ("prices, where they
/// exist, within ±30 % of a reference basket", GDD §17 item 2).
#[derive(Clone, Debug, PartialEq)]
pub struct StabilityTargets {
    pub need_min: f64,
    pub price_index_band: Option<(f64, f64)>,
    pub max_food_stockout_cycles: u32,
    /// Share of Materials consumed by Machine Shops over Materials produced.
    /// The band is the simulator's own reading of "roughly balanced"
    /// (GDD §17 asks only for a documented band; `docs/tuning/README.md`).
    pub investment_band: (f64, f64),
    pub max_rejected: u32,
}

impl StabilityTargets {
    #[must_use]
    pub fn for_capabilities(caps: &Capabilities) -> Self {
        StabilityTargets {
            need_min: 0.95,
            price_index_band: caps.order_books.then_some((0.7, 1.3)),
            max_food_stockout_cycles: 3,
            investment_band: (0.05, 0.6),
            max_rejected: 0,
        }
    }
}

/// The targets, checked per epoch. Returns the failures as text.
#[must_use]
pub fn stability_failures(targets: &StabilityTargets, summary: &[EpochSummary]) -> Vec<String> {
    let mut f = Vec::new();
    for s in summary {
        if s.mean_need_fulfillment < targets.need_min {
            f.push(format!(
                "epoch {}: need fulfillment {:.3} < {:.2}",
                s.epoch, s.mean_need_fulfillment, targets.need_min
            ));
        }
        if let Some((lo, hi)) = targets.price_index_band {
            match (s.min_price_index, s.max_price_index) {
                (Some(min), Some(max)) if min < lo || max > hi => f.push(format!(
                    "epoch {}: price index {min:.2}..{max:.2} outside {lo}..{hi}",
                    s.epoch
                )),
                (None, _) | (_, None) => f.push(format!(
                    "epoch {}: no price index although the society has order books",
                    s.epoch
                )),
                _ => {}
            }
        }
        if s.longest_food_stockout > targets.max_food_stockout_cycles {
            f.push(format!(
                "epoch {}: Food stock-out for {} consecutive cycles (limit {})",
                s.epoch, s.longest_food_stockout, targets.max_food_stockout_cycles
            ));
        }
        let (lo, hi) = targets.investment_band;
        if s.mean_investment_share < lo || s.mean_investment_share > hi {
            f.push(format!(
                "epoch {}: investment share {:.3} outside the {lo}..{hi} band",
                s.epoch, s.mean_investment_share
            ));
        }
        if s.rejected_commands > targets.max_rejected {
            f.push(format!(
                "epoch {}: {} rejected householder commands",
                s.epoch, s.rejected_commands
            ));
        }
    }
    f
}

/// Write rows as CSV.
pub fn write_csv(path: &Path, rows: &[Row]) -> std::io::Result<()> {
    let mut w = csv::Writer::from_path(path)?;
    for r in rows {
        w.serialize(r)?;
    }
    w.flush()
}

fn index_cell(v: Option<f64>) -> String {
    v.map_or_else(|| "-".to_owned(), |x| format!("{x:.2}"))
}

/// Render the summary as a fixed-width table.
#[must_use]
#[allow(clippy::format_push_string)]
pub fn table(summary: &[EpochSummary]) -> String {
    let mut s = String::from(
        "epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected\n",
    );
    for e in summary {
        s.push_str(&format!(
            "{:>5}  {:>6}  {:>5.1}  {:>9}  {:>9}  {:>5.3}  {:>6.3}  {:>8}  {:>5}  {:>8}  {:>8}\n",
            e.epoch,
            e.cycles,
            e.mean_need_fulfillment * 100.0,
            index_cell(e.min_price_index),
            index_cell(e.max_price_index),
            e.mean_gini,
            e.mean_investment_share,
            e.max_hardship,
            e.max_unemployed,
            e.longest_food_stockout,
            e.rejected_commands
        ));
    }
    s
}

/// Run one preset over a seed range: print the per-seed table, write the CSVs
/// when `out` is given, and return every target miss as `seed n: ...` lines.
/// Shared by the `run` and `all` subcommands.
pub fn sweep(
    presets_dir: &Path,
    preset: &str,
    epochs: u32,
    seeds: impl IntoIterator<Item = u64>,
    overrides: &[(String, toml::Value)],
    out: Option<&Path>,
    detail: Option<&DetailOptions>,
) -> Result<Vec<String>, SweepError> {
    let mut failures = Vec::new();
    for seed in seeds {
        let spec = RunSpec {
            preset: preset.to_owned(),
            epochs,
            seed,
            overrides: overrides.to_vec(),
        };
        let started = std::time::Instant::now();
        let mut writer = detail
            .map(|d| DetailWriter::create(d, preset, seed))
            .transpose()
            .map_err(SweepError::Io)?;
        let result = match writer.as_mut() {
            Some(w) => run_with(presets_dir, &spec, w),
            None => run(presets_dir, &spec),
        }
        .map_err(SweepError::Config)?;
        if let Some(w) = writer.take() {
            let dir = w.finish().map_err(SweepError::Io)?;
            println!("wrote {}{}", dir.display(), std::path::MAIN_SEPARATOR);
        }
        let summary = summarize(&result.rows);
        println!(
            "{preset} seed {seed}: {epochs} epochs, {} events, {} rejected commands, {:.1}s",
            result.events,
            result.rejected,
            started.elapsed().as_secs_f64()
        );
        print!("{}", table(&summary));
        if let Some(dir) = out {
            std::fs::create_dir_all(dir).map_err(SweepError::Io)?;
            let path = dir.join(format!("{preset}-{seed}.csv"));
            write_csv(&path, &result.rows).map_err(SweepError::Io)?;
            println!("wrote {}", path.display());
        }
        let targets = StabilityTargets::for_capabilities(&result.capabilities);
        for f in stability_failures(&targets, &summary) {
            println!("  ! {f}");
            failures.push(format!("seed {seed}: {f}"));
        }
    }
    Ok(failures)
}

/// Why a sweep could not run (as opposed to running and missing targets).
#[derive(Debug)]
pub enum SweepError {
    Config(ConfigError),
    Io(std::io::Error),
}

impl std::fmt::Display for SweepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SweepError::Config(e) => write!(f, "{e}"),
            SweepError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SweepError {}
