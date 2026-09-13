//! S0.13b done gate: a 5-epoch Freeport run completes quickly with the roster
//! kept and material state reset at each rollover; CSV columns match TDD §13;
//! the stability check runs (it may fail: that is S0.14's job).
//!
//! S0.14d: the targets derive from each preset's capabilities, every preset
//! has its own ignored `stability_<preset>` test (`make sim-check PRESET=x`),
//! and all five presets complete an epoch and conserve on the same seed.

use isms_sim::{PRESETS, RunSpec, StabilityTargets, run, stability_failures, summarize};
use std::path::Path;

fn presets() -> &'static Path {
    Path::new(isms_core::WORKSPACE_PRESETS_DIR)
}

#[test]
fn five_freeport_epochs_run_and_roll_over() {
    let started = std::time::Instant::now();
    let r = run(presets(), &RunSpec::new("freeport", 5, 1)).unwrap();
    let elapsed = started.elapsed();
    assert_eq!(r.rows.len(), 5 * 42, "one row per cycle");
    assert!(
        r.rows
            .iter()
            .all(|row| row.householders == 40 && row.active_humans == 0)
    );
    assert_eq!(r.world.meta.epoch, 4);
    assert!(r.world.meta.epoch_ended.is_some());
    assert!(elapsed.as_secs() < 120, "took {elapsed:?} (debug build)");
    isms_core::ledger::conservation_check(&r.world).unwrap();
}

#[test]
fn csv_columns_match_the_tdd() {
    let r = run(presets(), &RunSpec::new("freeport", 1, 2)).unwrap();
    let tmp = std::env::temp_dir().join(format!("isms-sim-{}.csv", std::process::id()));
    isms_sim::write_csv(&tmp, &r.rows).unwrap();
    let text = std::fs::read_to_string(&tmp).unwrap();
    let header = text.lines().next().unwrap();
    for col in [
        "preset",
        "seed",
        "epoch",
        "cycle",
        "population",
        "active_humans",
        "householders",
        "real_output",
        "median_wellbeing",
        "need_fulfillment_rate",
        "consumption_gini",
        "investment_share",
        "price_index",
        "mean_cycle_wage",
        "unemployed",
        "firm_count",
        "credit_outstanding_credits",
        "hardship_count",
        "food_ask_depth",
        "wares_ask_depth",
        "food_last_price",
        "rejected_commands",
        "food_available",
        "food_unfilled",
        "stocked_out",
        "store_food",
        "state_food",
        "treasury_credits",
        "till_credits",
    ] {
        assert!(header.split(',').any(|h| h == col), "missing column {col}");
    }
    assert_eq!(text.lines().count(), 1 + 42);
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn param_overrides_reach_the_engine() {
    let mut spec = RunSpec::new("freeport", 1, 3);
    spec.overrides
        .push(("params.population.floor".into(), toml::Value::Integer(12)));
    let r = run(presets(), &spec).unwrap();
    assert!(r.rows.iter().all(|row| row.householders == 12));
}

#[test]
fn runs_are_deterministic() {
    let a = run(presets(), &RunSpec::new("freeport", 1, 9)).unwrap();
    let b = run(presets(), &RunSpec::new("freeport", 1, 9)).unwrap();
    assert_eq!(a.world.hash(), b.world.hash());
    assert_eq!(a.events, b.events);
}

/// The targets follow the capabilities: a price band only where order books
/// exist; the stock-out reading follows the society's Food holder.
#[test]
fn targets_derive_from_capabilities() {
    let market = run(presets(), &RunSpec::new("freeport", 1, 1)).unwrap();
    let t = StabilityTargets::for_capabilities(&market.capabilities);
    assert_eq!(t.price_index_band, Some((0.7, 1.3)));
    assert!(
        market
            .rows
            .iter()
            .all(|r| r.food_available == r.food_ask_depth)
    );
    assert!(
        market
            .rows
            .iter()
            .all(|r| r.stocked_out == (r.food_ask_depth == 0))
    );

    let commune = run(presets(), &RunSpec::new("commune", 1, 1)).unwrap();
    let t = StabilityTargets::for_capabilities(&commune.capabilities);
    assert_eq!(t.price_index_band, None);
    assert!(
        commune
            .rows
            .iter()
            .all(|r| r.food_available == r.store_food)
    );
    assert!(
        commune
            .rows
            .iter()
            .all(|r| r.stocked_out == (r.food_unfilled > 0))
    );
    let s = summarize(&commune.rows);
    assert!(s.iter().all(|e| e.min_price_index.is_none()));
    assert!(
        !stability_failures(&t, &s)
            .iter()
            .any(|f| f.contains("price index")),
        "a moneyless society is never judged on prices"
    );
}

/// Every preset completes an epoch on the same seed, and money and goods are
/// conserved at the end of it. Stability is each preset's own card; this only
/// guards against one preset's session breaking another.
#[test]
fn all_five_presets_complete_one_epoch_and_conserve() {
    for preset in PRESETS {
        let r =
            run(presets(), &RunSpec::new(preset, 1, 1)).unwrap_or_else(|e| panic!("{preset}: {e}"));
        assert_eq!(r.rows.len(), 42, "{preset}: one row per cycle");
        assert!(r.world.meta.epoch_ended.is_some(), "{preset}");
        isms_core::ledger::conservation_check(&r.world)
            .unwrap_or_else(|e| panic!("{preset}: {e:?}"));
    }
}

/// GDD §17 stability targets over seeds 1..=5. Ignored by default;
/// `make sim-check PRESET=<preset>` runs one, `make sim-all` prints all five.
fn stability(preset: &str) {
    let mut all = Vec::new();
    for seed in 1..=5u64 {
        let r = run(presets(), &RunSpec::new(preset, 5, seed)).unwrap();
        let s = summarize(&r.rows);
        let t = StabilityTargets::for_capabilities(&r.capabilities);
        let f = stability_failures(&t, &s);
        println!("{preset} seed {seed}\n{}", isms_sim::table(&s));
        all.extend(f.into_iter().map(|x| format!("seed {seed}: {x}")));
    }
    assert!(
        all.is_empty(),
        "{preset}: stability targets missed:\n{}",
        all.join("\n")
    );
}

#[test]
#[ignore = "long; run with make sim-check PRESET=freeport"]
fn stability_freeport() {
    stability("freeport");
}

#[test]
#[ignore = "long; run with make sim-check PRESET=commune (green from S0.15c)"]
fn stability_commune() {
    stability("commune");
}

#[test]
#[ignore = "long; run with make sim-check PRESET=directorate (green from S0.16c)"]
fn stability_directorate() {
    stability("directorate");
}

#[test]
#[ignore = "long; run with make sim-check PRESET=republic (green from S0.17a)"]
fn stability_republic() {
    stability("republic");
}

#[test]
#[ignore = "long; run with make sim-check PRESET=commonwealth (green from S0.17c)"]
fn stability_commonwealth() {
    stability("commonwealth");
}
