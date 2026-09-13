//! `isms-server`: one binary, subsystems by subcommand (TDD 3). `migrate`,
//! `rebuild`, `seed`, and `serve` (actors + scheduler; the API arrives in S1.3).

use clap::{Parser, Subcommand};
use isms_server::runtime::{Runtime, RuntimeError, SeedSpec, StartOptions, seed_society};
use isms_store::{EventStore, PgEventStore, load_world, verify_latest_snapshot};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "isms-server",
    about = "Isms API, scheduler, and society actors"
)]
struct Cli {
    /// Postgres URL (default: `DATABASE_URL` from the environment or `.env`).
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    /// Presets directory (default: the workspace's; embedded in the image from S1.14).
    #[arg(long, env = "ISMS_PRESETS_DIR")]
    presets: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Apply pending migrations.
    Migrate,
    /// Verify a society's latest snapshot against a full replay and write a fresh one.
    Rebuild {
        #[arg(long)]
        society: i64,
    },
    /// Create a society with householders and its epoch-0 seeding.
    Seed {
        #[arg(long, default_value = "freeport")]
        preset: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Wall-clock seconds per tick (0 = as fast as possible).
        #[arg(long, env = "ISMS_TICK_SECONDS", default_value_t = 3600)]
        tick_seconds: u32,
        /// Hour (UTC) at which cycles end.
        #[arg(long, default_value_t = 4)]
        cycle_boundary_hour: u32,
        /// Param override `params.x.y=value` (TOML value syntax); repeatable.
        #[arg(long = "param")]
        params: Vec<String>,
    },
    /// Load every active society, run actors and schedulers until Ctrl-C.
    Serve {
        /// Replay the whole log against the latest snapshot before starting (TDD 9.1).
        #[arg(long)]
        verify_replay: bool,
    },
}

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error(transparent)]
    Store(#[from] isms_store::StoreError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("{0}")]
    Arg(String),
}

fn parse_override(s: &str) -> Result<(String, toml::Value), ServerError> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| ServerError::Arg(format!("expected key=value, got {s}")))?;
    let v: toml::Value = format!("x = {v}")
        .parse::<toml::Table>()
        .map_err(|e| ServerError::Arg(format!("bad value for {k}: {e}")))?
        .remove("x")
        .ok_or_else(|| ServerError::Arg("empty value".to_owned()))?;
    Ok((k.trim().to_owned(), v))
}

async fn run(cli: Cli) -> Result<(), ServerError> {
    let store = PgEventStore::connect(&cli.database_url).await?;
    let presets_dir = cli
        .presets
        .unwrap_or_else(|| PathBuf::from(isms_core::WORKSPACE_PRESETS_DIR));
    match cli.cmd {
        Cmd::Migrate => {
            store.migrate().await?;
            tracing::info!("migrations applied");
        }
        Cmd::Rebuild { society } => {
            if let Some(seq) = verify_latest_snapshot(&store, society).await? {
                tracing::info!(society, seq, "latest snapshot verified");
            } else {
                tracing::info!(society, "no snapshot yet");
            }
            let loaded = load_world(&store, society).await?;
            store
                .write_snapshot(society, &loaded.world, loaded.last_seq)
                .await?;
            tracing::info!(
                society,
                seq = loaded.last_seq,
                tick = loaded.world.meta.tick,
                "snapshot written"
            );
        }
        Cmd::Seed {
            preset,
            name,
            seed,
            tick_seconds,
            cycle_boundary_hour,
            params,
        } => {
            let overrides = params
                .iter()
                .map(|p| parse_override(p))
                .collect::<Result<Vec<_>, _>>()?;
            let spec = SeedSpec {
                name,
                preset,
                seed,
                tick_seconds,
                cycle_boundary_hour,
                overrides,
            };
            let row = seed_society(&store, &presets_dir, &spec).await?;
            println!("{}", row.id);
        }
        Cmd::Serve { verify_replay } => {
            let runtime = Runtime::start(&store, StartOptions { verify_replay }).await?;
            tracing::info!(
                societies = runtime.societies.len(),
                "serving; Ctrl-C to stop"
            );
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
            runtime.shutdown().await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    if std::env::var("ISMS_LOG_JSON").is_ok_and(|v| v == "1") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}
