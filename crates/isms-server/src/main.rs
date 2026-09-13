//! `isms-server`: one binary, subsystems by subcommand (TDD 3). S1.1 ships
//! `migrate` and `rebuild`; `serve` arrives with S1.2.

use clap::{Parser, Subcommand};
use isms_store::{EventStore, PgEventStore, load_world, verify_latest_snapshot};
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
    /// Load every active society and serve the API (S1.2).
    Serve,
}

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error(transparent)]
    Store(#[from] isms_store::StoreError),
    #[error("not implemented until {0}")]
    NotYet(&'static str),
}

async fn run(cli: Cli) -> Result<(), ServerError> {
    let store = PgEventStore::connect(&cli.database_url).await?;
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
        Cmd::Serve => return Err(ServerError::NotYet("S1.2")),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}
