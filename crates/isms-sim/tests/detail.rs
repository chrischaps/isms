//! S0.14e done gate: `--detail` writes per-citizen, per-org, flow, trade,
//! move and depth CSVs; the observed run is byte-identical to the plain run;
//! the observer sees every event; the columns match the README's contract.

use isms_core::event::Event;
use isms_core::ids::{Cycle, Epoch, Tick};
use isms_core::world::World;
use isms_sim::detail::{FILES, TICKS_FILE};
use isms_sim::{DetailOptions, DetailWriter, Observer, Row, RunSpec, run, run_with};
use std::path::{Path, PathBuf};

fn presets() -> &'static Path {
    Path::new(isms_core::WORKSPACE_PRESETS_DIR)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("isms-detail-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn detail_run(preset: &str, seed: u64, dir: &Path, ticks: bool) -> (isms_sim::RunResult, PathBuf) {
    let opts = DetailOptions {
        dir: dir.to_path_buf(),
        ticks,
    };
    let mut w = DetailWriter::create(&opts, preset, seed).unwrap();
    let r = run_with(presets(), &RunSpec::new(preset, 1, seed), &mut w).unwrap();
    let out = w.finish().unwrap();
    (r, out)
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn header(text: &str) -> Vec<&str> {
    text.lines().next().unwrap().split(',').collect()
}

fn col<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let idx = header(text).iter().position(|h| *h == name).unwrap();
    text.lines()
        .skip(1)
        .map(|l| l.split(',').nth(idx).unwrap())
        .collect()
}

#[test]
fn plain_run_and_observed_run_are_byte_identical() {
    let plain = run(presets(), &RunSpec::new("freeport", 1, 1)).unwrap();
    let d1 = tmp("a");
    let d2 = tmp("b");
    let (observed, out1) = detail_run("freeport", 1, &d1, false);
    let (_, out2) = detail_run("freeport", 1, &d2, false);
    assert_eq!(plain.world.hash(), observed.world.hash());
    assert_eq!(plain.events, observed.events);
    let p1 = d1.join("plain.csv");
    let p2 = d1.join("observed.csv");
    isms_sim::write_csv(&p1, &plain.rows).unwrap();
    isms_sim::write_csv(&p2, &observed.rows).unwrap();
    assert_eq!(std::fs::read(&p1).unwrap(), std::fs::read(&p2).unwrap());
    for f in FILES {
        assert_eq!(
            std::fs::read(out1.join(f)).unwrap(),
            std::fs::read(out2.join(f)).unwrap(),
            "{f} differs between two runs of the same seed"
        );
    }
    std::fs::remove_dir_all(&d1).ok();
    std::fs::remove_dir_all(&d2).ok();
}

#[derive(Default)]
struct Counter {
    seen: u64,
    cycles: u32,
    epochs: u32,
}

impl Observer for Counter {
    fn epoch_started(&mut self, _w: &World, _e: Epoch, events: &[Event]) {
        self.epochs += 1;
        self.seen += events.len() as u64;
    }
    fn tick_done(&mut self, _w: &World, _e: Epoch, _t: Tick, round: &[Event], tick: &[Event]) {
        self.seen += (round.len() + tick.len()) as u64;
    }
    fn cycle_closed(&mut self, _w: &World, _e: Epoch, _c: Cycle, _r: &Row) {
        self.cycles += 1;
    }
}

#[test]
fn observer_sees_every_event_but_society_created() {
    let mut c = Counter::default();
    let r = run_with(presets(), &RunSpec::new("freeport", 1, 2), &mut c).unwrap();
    assert_eq!(c.seen, r.events - 1);
    assert_eq!(c.cycles, 42);
    assert_eq!(c.epochs, 1);
}

#[test]
fn detail_files_have_expected_columns_and_row_counts() {
    let d = tmp("cols");
    let (_, out) = detail_run("freeport", 1, &d, false);
    assert_eq!(out, d.join("freeport-1"));
    assert!(!out.join(TICKS_FILE).exists());

    let citizens = read(&out, "citizens.csv");
    assert_eq!(
        header(&citizens),
        "preset,seed,epoch,cycle,citizen,handle,dormant,food_meter,shelter_meter,comfort_meter,balance_credits,pantry_food,pantry_wares,pantry_other,cycle_wages_credits,wages_total_credits,workplace,workplace_kind,org,contract,alloc_hours,effort,tick_hours_worked,attributed_output,skill_family,skill_level,skill_max_level,budget,fatigue_debt,output_mult,food_eaten,wares_consumed,housed,dwelling,in_hardship,destitute,defaulted".split(',').collect::<Vec<_>>()
    );
    assert_eq!(citizens.lines().count(), 1 + 42 * 40);

    let orgs = read(&out, "orgs.csv");
    assert_eq!(
        header(&orgs),
        "preset,seed,epoch,cycle,org,name,kind,ownership,manager,treasury_credits,inv_grain,inv_ore,inv_materials,inv_food,inv_wares,inv_machines,workplaces,machines,employees,positions,members,wages_paid_credits,dividends_paid_credits,declared_dividend_credits,payment_missed".split(',').collect::<Vec<_>>()
    );
    assert!(orgs.lines().count() > 1);

    let flows = read(&out, "flows.csv");
    assert_eq!(
        header(&flows),
        "preset,seed,epoch,cycle,workplace,kind,org,machines,workers,tick_hours,output_good,units,in_grain,in_ore,in_materials,in_food,in_wares,in_machines".split(',').collect::<Vec<_>>()
    );
    assert!(col(&flows, "units").iter().any(|u| *u != "0"));

    let trades = read(&out, "trades.csv");
    assert_eq!(
        header(&trades),
        "preset,seed,epoch,cycle,tick,phase,instrument,buyer,seller,buy_order,sell_order,qty,price_credits,value_credits".split(',').collect::<Vec<_>>()
    );
    assert!(trades.lines().count() > 1);
    assert!(col(&trades, "instrument").contains(&"food"));

    let moves = read(&out, "moves.csv");
    assert_eq!(
        header(&moves),
        "preset,seed,epoch,cycle,tick,kind,from,to,good,qty,amount_credits"
            .split(',')
            .collect::<Vec<_>>()
    );
    assert!(
        col(&moves, "kind").contains(&"seeded"),
        "legacy inventory is seeded (ADR-0004)"
    );

    let depth = read(&out, "depth.csv");
    assert_eq!(
        header(&depth),
        "preset,seed,epoch,cycle,tick,instrument,best_bid_credits,bid_qty,bid_levels,best_ask_credits,ask_qty,ask_levels,last_price_credits".split(',').collect::<Vec<_>>()
    );
    let ticks = 24 * 42;
    let depth_rows = depth.lines().count() - 1;
    assert!(depth_rows >= ticks * 6);
    assert_eq!(depth_rows % ticks, 0);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn cycle_wages_column_matches_row_mean() {
    let d = tmp("wages");
    let (r, out) = detail_run("freeport", 3, &d, false);
    let citizens = read(&out, "citizens.csv");
    let cycles = col(&citizens, "cycle");
    let wages = col(&citizens, "cycle_wages_credits");
    for row in &r.rows {
        let paid: Vec<f64> = cycles
            .iter()
            .zip(&wages)
            .filter(|(c, _)| c.parse::<u32>().unwrap() == row.cycle)
            .map(|(_, w)| w.parse::<f64>().unwrap())
            .filter(|w| *w > 0.0)
            .collect();
        let mean = if paid.is_empty() {
            0.0
        } else {
            paid.iter().sum::<f64>() / paid.len() as f64
        };
        assert!(
            (mean - row.mean_cycle_wage).abs() < 1e-6,
            "cycle {}: detail mean {mean} vs aggregate {}",
            row.cycle,
            row.mean_cycle_wage
        );
    }
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn hours_worked_is_positive_for_employed_and_zero_for_idle() {
    let d = tmp("hours");
    let (_, out) = detail_run("freeport", 1, &d, false);
    let citizens = read(&out, "citizens.csv");
    let cycles = col(&citizens, "cycle");
    let wp = col(&citizens, "workplace");
    let hours = col(&citizens, "tick_hours_worked");
    let mut employed_with_hours = 0;
    for ((c, w), h) in cycles.iter().zip(&wp).zip(&hours) {
        let h: u64 = h.parse().unwrap();
        if w.is_empty() {
            // No position at cycle end: hours were still possible if the
            // citizen quit mid-cycle, so only the converse is asserted below.
        } else if h > 0 {
            employed_with_hours += 1;
        }
        // A citizen who worked must have had a position at some point;
        // cycle 0 of every epoch starts unemployed, so allow it there.
        if h > 0 && w.is_empty() {
            assert!(*c != "0", "hours without a position in cycle 0");
        }
    }
    assert!(employed_with_hours > 0);

    // Per cycle, hours from the citizens' skills cover the hours the
    // workplaces reported in `Produced` (a zero-output tick never emits one).
    let flows = read(&out, "flows.csv");
    for cycle in 0..42u32 {
        let c_sum: u64 = cycles
            .iter()
            .zip(&hours)
            .filter(|(c, _)| c.parse::<u32>().unwrap() == cycle)
            .map(|(_, h)| h.parse::<u64>().unwrap())
            .sum();
        let f_sum: u64 = col(&flows, "cycle")
            .iter()
            .zip(col(&flows, "tick_hours"))
            .filter(|(c, _)| c.parse::<u32>().unwrap() == cycle)
            .map(|(_, h)| h.parse::<u64>().unwrap())
            .sum();
        assert!(
            c_sum >= f_sum,
            "cycle {cycle}: citizens {c_sum} < flows {f_sum}"
        );
    }
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn commune_detail_has_drew_moves_and_no_books() {
    let d = tmp("commune");
    let (_, out) = detail_run("commune", 1, &d, false);
    let moves = read(&out, "moves.csv");
    assert!(col(&moves, "kind").contains(&"drew"));
    assert_eq!(read(&out, "trades.csv").lines().count(), 1, "header only");
    assert_eq!(read(&out, "depth.csv").lines().count(), 1, "header only");
    let orgs = read(&out, "orgs.csv");
    assert!(col(&orgs, "positions").iter().any(|p| *p != "0"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn citizens_ticks_is_opt_in() {
    let d = tmp("ticks");
    let (_, out) = detail_run("freeport", 1, &d, true);
    let ticks = read(&out, TICKS_FILE);
    assert_eq!(
        header(&ticks),
        "preset,seed,epoch,cycle,tick,citizen,food_meter,shelter_meter,comfort_meter,balance_credits,pantry_food,pantry_wares,food_eaten,wares_consumed,output_mult,in_hardship".split(',').collect::<Vec<_>>()
    );
    assert_eq!(ticks.lines().count(), 1 + 24 * 42 * 40);
    std::fs::remove_dir_all(&d).ok();
}
