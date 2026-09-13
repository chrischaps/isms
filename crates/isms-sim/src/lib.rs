//! `isms-sim`: the headless simulator (TDD §18.2 S0.13, GDD §17). Links
//! `isms-core` directly, never touches the network or a database, and runs
//! householders only. Its metrics are the engine's own `CycleAggregates`.

use isms_core::apply::apply;
use isms_core::config::{ConfigError, Preset, load_preset_with_overrides};
use isms_core::event::Event;
use isms_core::householder::run_round;
use isms_core::kinds::Good;
use isms_core::market::depth;
use isms_core::rules::Rules;
use isms_core::tick::{TickInput, start_epoch, tick};
use isms_core::world::{Instrument, Side, World};
use serde::Serialize;
use std::path::Path;

/// One row of the metrics table: a `CycleClosed` plus a few market readings.
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

/// The outcome of one run.
#[derive(Debug)]
pub struct RunResult {
    pub rows: Vec<Row>,
    pub events: u64,
    pub rejected: u32,
    pub world: World,
}

fn ask_depth(world: &World, good: Good) -> u32 {
    depth(world, Instrument::Good(good), Side::Ask)
        .iter()
        .map(|(_, q)| *q)
        .sum()
}

/// Run a householder-only society for `epochs` epochs. Collapse is off (the
/// simulator never has humans). Between ticks the householder scripts run once.
pub fn run(presets_dir: &Path, spec: &RunSpec) -> Result<RunResult, ConfigError> {
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
    let mut rows = Vec::new();
    let mut events: u64 = 1;
    let mut rejected_total = 0u32;
    for epoch in 0..spec.epochs {
        for e in start_epoch(&world, &rules, epoch) {
            apply(&mut world, &e);
            events += 1;
        }
        let mut rejected_cycle = 0u32;
        loop {
            let now = world.meta.tick;
            let (round, rejected) = run_round(&mut world, &rules, now);
            events += round.len() as u64;
            let n = u32::try_from(rejected.len()).unwrap_or(u32::MAX);
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
            for e in &tick_events {
                apply(&mut world, e);
            }
            events += tick_events.len() as u64;
            if let Some((cycle, a)) = closed {
                rows.push(Row {
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
                    food_ask_depth: ask_depth(&world, Good::Food),
                    wares_ask_depth: ask_depth(&world, Good::Wares),
                    food_last_price: isms_core::market::last_price(
                        &world,
                        Instrument::Good(Good::Food),
                    )
                    .map(isms_core::money::Money::as_credits_f64),
                    rejected_commands: rejected_cycle,
                });
                rejected_cycle = 0;
            }
            if world.meta.epoch_ended.is_some() {
                break;
            }
        }
    }
    Ok(RunResult {
        rows,
        events,
        rejected: rejected_total,
        world,
    })
}

/// Per-epoch summary used by the console table and the stability checks.
#[derive(Clone, Debug, Serialize)]
pub struct EpochSummary {
    pub epoch: u32,
    pub cycles: usize,
    pub mean_need_fulfillment: f64,
    pub min_price_index: f64,
    pub max_price_index: f64,
    pub mean_gini: f64,
    pub mean_investment_share: f64,
    pub max_hardship: u32,
    pub max_unemployed: u32,
    /// Longest run of consecutive cycles with no Food asks resting at cycle end.
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
            if r.food_ask_depth == 0 {
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
            min_price_index: idx.iter().copied().fold(f64::INFINITY, f64::min),
            max_price_index: idx.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            mean_gini: rs.iter().map(|r| r.consumption_gini).sum::<f64>() / n,
            mean_investment_share: rs.iter().map(|r| r.investment_share).sum::<f64>() / n,
            max_hardship: rs.iter().map(|r| r.hardship_count).max().unwrap_or(0),
            max_unemployed: rs.iter().map(|r| r.unemployed).max().unwrap_or(0),
            longest_food_stockout: longest,
            rejected_commands: rs.iter().map(|r| r.rejected_commands).sum(),
        });
    }
    out
}

/// GDD §17 tuning targets, checked per epoch. Returns the failures as text.
#[must_use]
pub fn stability_failures(summary: &[EpochSummary]) -> Vec<String> {
    let mut f = Vec::new();
    for s in summary {
        if s.mean_need_fulfillment < 0.95 {
            f.push(format!(
                "epoch {}: need fulfillment {:.3} < 0.95",
                s.epoch, s.mean_need_fulfillment
            ));
        }
        if s.min_price_index < 0.7 || s.max_price_index > 1.3 {
            f.push(format!(
                "epoch {}: price index {:.2}..{:.2} outside 0.7..1.3",
                s.epoch, s.min_price_index, s.max_price_index
            ));
        }
        if s.longest_food_stockout > 3 {
            f.push(format!(
                "epoch {}: Food stock-out for {} consecutive cycles",
                s.epoch, s.longest_food_stockout
            ));
        }
        if s.mean_investment_share < 0.05 || s.mean_investment_share > 0.6 {
            f.push(format!(
                "epoch {}: investment share {:.3} outside the 0.05..0.6 band",
                s.epoch, s.mean_investment_share
            ));
        }
        if s.rejected_commands > 0 {
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

/// Render the summary as a fixed-width table.
#[must_use]
#[allow(clippy::format_push_string)]
pub fn table(summary: &[EpochSummary]) -> String {
    let mut s = String::from(
        "epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected\n",
    );
    for e in summary {
        s.push_str(&format!(
            "{:>5}  {:>6}  {:>5.1}  {:>9.2}  {:>9.2}  {:>5.3}  {:>6.3}  {:>8}  {:>5}  {:>8}  {:>8}\n",
            e.epoch,
            e.cycles,
            e.mean_need_fulfillment * 100.0,
            e.min_price_index,
            e.max_price_index,
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
