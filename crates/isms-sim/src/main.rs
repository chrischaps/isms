//! `isms-sim` CLI: `isms-sim run --preset freeport --epochs 5 --seed 1`.

use clap::{Parser, Subcommand};
use isms_sim::{RunSpec, run, stability_failures, summarize, table, write_csv};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "isms-sim", about = "Headless simulator for Isms societies")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run a householder-only society and print per-epoch metrics.
    Run {
        #[arg(long, default_value = "freeport")]
        preset: String,
        #[arg(long, default_value_t = 5)]
        epochs: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Inclusive seed range `a..b`; overrides --seed.
        #[arg(long)]
        seeds: Option<String>,
        /// Param override `params.x.y=value` (TOML value syntax); repeatable.
        #[arg(long = "param")]
        params: Vec<String>,
        /// Directory for `<preset>-<seed>.csv` files.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Presets directory (default: the workspace's).
        #[arg(long)]
        presets: Option<PathBuf>,
        /// Fail (exit 1) if any epoch misses the GDD §17 targets.
        #[arg(long)]
        check: bool,
    },
}

fn parse_override(s: &str) -> Result<(String, toml::Value), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected key=value, got {s}"))?;
    let v: toml::Value = format!("x = {v}")
        .parse::<toml::Table>()
        .map_err(|e| format!("bad value for {k}: {e}"))?
        .remove("x")
        .ok_or_else(|| "empty value".to_owned())?;
    Ok((k.trim().to_owned(), v))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Run {
            preset,
            epochs,
            seed,
            seeds,
            params,
            out,
            presets,
            check,
        } => {
            let presets_dir =
                presets.unwrap_or_else(|| PathBuf::from(isms_core::WORKSPACE_PRESETS_DIR));
            let overrides: Result<Vec<_>, _> = params.iter().map(|p| parse_override(p)).collect();
            let overrides = match overrides {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            let seed_list: Vec<u64> = match seeds {
                Some(r) => {
                    let (a, b) = r.split_once("..").unwrap_or((&r, &r));
                    let (a, b): (u64, u64) = (a.parse().unwrap_or(seed), b.parse().unwrap_or(seed));
                    (a..=b).collect()
                }
                None => vec![seed],
            };
            let mut failed = false;
            for s in seed_list {
                let spec = RunSpec {
                    preset: preset.clone(),
                    epochs,
                    seed: s,
                    overrides: overrides.clone(),
                };
                let started = std::time::Instant::now();
                let result = match run(&presets_dir, &spec) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                };
                let summary = summarize(&result.rows);
                println!(
                    "{preset} seed {s}: {} epochs, {} events, {} rejected commands, {:.1}s",
                    epochs,
                    result.events,
                    result.rejected,
                    started.elapsed().as_secs_f64()
                );
                print!("{}", table(&summary));
                if let Some(dir) = &out {
                    if let Err(e) = std::fs::create_dir_all(dir) {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                    let path = dir.join(format!("{preset}-{s}.csv"));
                    if let Err(e) = write_csv(&path, &result.rows) {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                    println!("wrote {}", path.display());
                }
                let failures = stability_failures(&summary);
                for f in &failures {
                    println!("  ! {f}");
                }
                failed |= !failures.is_empty();
            }
            if check && failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}
