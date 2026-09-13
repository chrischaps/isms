//! `isms-sim` CLI: `isms-sim run --preset freeport --epochs 5 --seed 1`, or
//! `isms-sim all --epochs 5 --seeds 1..5` for the five presets side by side.

use clap::{Parser, Subcommand};
use isms_sim::{PRESETS, sweep};
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
    /// Run all five presets over the same seeds and print their tables in turn.
    All {
        #[arg(long, default_value_t = 5)]
        epochs: u32,
        /// Inclusive seed range `a..b`.
        #[arg(long, default_value = "1..5")]
        seeds: String,
        /// Directory for `<preset>-<seed>.csv` files.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Presets directory (default: the workspace's).
        #[arg(long)]
        presets: Option<PathBuf>,
        /// Fail (exit 1) if any preset misses its targets in any epoch.
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

fn parse_seeds(range: &str, default: u64) -> Vec<u64> {
    let (a, b) = range.split_once("..").unwrap_or((range, range));
    let (a, b): (u64, u64) = (a.parse().unwrap_or(default), b.parse().unwrap_or(default));
    (a..=b).collect()
}

fn presets_dir(presets: Option<PathBuf>) -> PathBuf {
    presets.unwrap_or_else(|| PathBuf::from(isms_core::WORKSPACE_PRESETS_DIR))
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
            let overrides: Result<Vec<_>, _> = params.iter().map(|p| parse_override(p)).collect();
            let overrides = match overrides {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            let seed_list = seeds.map_or_else(|| vec![seed], |r| parse_seeds(&r, seed));
            match sweep(
                &presets_dir(presets),
                &preset,
                epochs,
                seed_list,
                &overrides,
                out.as_deref(),
            ) {
                Ok(failures) if check && !failures.is_empty() => ExitCode::FAILURE,
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Cmd::All {
            epochs,
            seeds,
            out,
            presets,
            check,
        } => {
            let dir = presets_dir(presets);
            let seed_list = parse_seeds(&seeds, 1);
            let mut failed = Vec::new();
            for preset in PRESETS {
                println!("== {preset}");
                match sweep(
                    &dir,
                    preset,
                    epochs,
                    seed_list.iter().copied(),
                    &[],
                    out.as_deref(),
                ) {
                    Ok(f) if f.is_empty() => println!("{preset}: stable"),
                    Ok(f) => {
                        println!("{preset}: {} target misses", f.len());
                        failed.push(preset);
                    }
                    Err(e) => {
                        eprintln!("{preset}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            if failed.is_empty() {
                println!("all five presets stable");
                ExitCode::SUCCESS
            } else {
                println!("missed targets: {}", failed.join(", "));
                if check {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
    }
}
