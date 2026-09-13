//! S0.13b done gate: a 5-epoch Freeport run completes quickly with the roster
//! kept and material state reset at each rollover; CSV columns match TDD §13;
//! the stability check runs (it may fail: that is S0.14's job).

use isms_sim::{RunSpec, run, stability_failures, summarize};
use std::path::Path;

fn presets() -> &'static Path {
    Path::new(isms_core::WORKSPACE_PRESETS_DIR)
}

#[test]
fn five_freeport_epochs_run_and_roll_over() {
    let started = std::time::Instant::now();
    let r = run(
        presets(),
        &RunSpec {
            preset: "freeport".into(),
            epochs: 5,
            seed: 1,
            overrides: vec![],
        },
    )
    .unwrap();
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
    let r = run(
        presets(),
        &RunSpec {
            preset: "freeport".into(),
            epochs: 1,
            seed: 2,
            overrides: vec![],
        },
    )
    .unwrap();
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
    ] {
        assert!(header.split(',').any(|h| h == col), "missing column {col}");
    }
    assert_eq!(text.lines().count(), 1 + 42);
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn param_overrides_reach_the_engine() {
    let r = run(
        presets(),
        &RunSpec {
            preset: "freeport".into(),
            epochs: 1,
            seed: 3,
            overrides: vec![("params.population.floor".into(), toml::Value::Integer(12))],
        },
    )
    .unwrap();
    assert!(r.rows.iter().all(|row| row.householders == 12));
}

#[test]
fn runs_are_deterministic() {
    let a = run(
        presets(),
        &RunSpec {
            preset: "freeport".into(),
            epochs: 1,
            seed: 9,
            overrides: vec![],
        },
    )
    .unwrap();
    let b = run(
        presets(),
        &RunSpec {
            preset: "freeport".into(),
            epochs: 1,
            seed: 9,
            overrides: vec![],
        },
    )
    .unwrap();
    assert_eq!(a.world.hash(), b.world.hash());
    assert_eq!(a.events, b.events);
}

/// GDD §17 stability targets over seeds 1..=5. Ignored by default; `make sim-check`.
#[test]
#[ignore = "long; run with make sim-check PRESET=freeport"]
fn stability_freeport() {
    let mut all = Vec::new();
    for seed in 1..=5u64 {
        let r = run(
            presets(),
            &RunSpec {
                preset: "freeport".into(),
                epochs: 5,
                seed,
                overrides: vec![],
            },
        )
        .unwrap();
        let s = summarize(&r.rows);
        let f = stability_failures(&s);
        println!("seed {seed}\n{}", isms_sim::table(&s));
        all.extend(f.into_iter().map(|x| format!("seed {seed}: {x}")));
    }
    assert!(
        all.is_empty(),
        "stability targets missed:\n{}",
        all.join("\n")
    );
}
