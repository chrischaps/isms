//! `isms-server`: one binary, subsystems by subcommand (TDD 3): `migrate`,
//! `rebuild`, `seed`, `invite`, `openapi`, and `serve` (actors, scheduler, API).

use clap::{Parser, Subcommand};
use isms_server::runtime::{Runtime, RuntimeError, SeedSpec, StartOptions, seed_society};
use isms_server::state::AppState;
use isms_store::{EventStore, PgEventStore, load_world, verify_latest_snapshot};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "isms-server",
    about = "Isms API, scheduler, and society actors"
)]
struct Cli {
    /// Postgres URL (default: `DATABASE_URL` from the environment or `.env`).
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
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
    /// Mint invite codes (Phase 1 sign-up is invite only) and print them.
    Invite {
        #[arg(long, default_value_t = 1)]
        count: u32,
    },
    /// Print the `OpenAPI` document (no database needed).
    Openapi,
    /// Mint a browser session token for an email (creating the account) and print it.
    /// For dev and end-to-end tests: `isms login --session <token>`.
    Session {
        #[arg(long)]
        email: String,
    },
    /// Clear and regenerate a society's Chronicle from its event log.
    RebuildProjections {
        #[arg(long)]
        society: i64,
    },
    /// Load every active society, run actors, schedulers, and the API until Ctrl-C.
    Serve {
        /// Replay the whole log against the latest snapshot before starting (TDD 9.1).
        #[arg(long)]
        verify_replay: bool,
        /// Listen address.
        #[arg(long, env = "ISMS_BIND", default_value = "127.0.0.1:8080")]
        bind: String,
        /// Public origin used in magic links and redirects.
        #[arg(long, env = "ISMS_BASE_URL", default_value = "http://localhost:8080")]
        base_url: String,
    },
}

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error(transparent)]
    Store(#[from] isms_store::StoreError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Mail(#[from] isms_server::mail::MailError),
    #[error("{0}")]
    Arg(String),
    #[error("listen: {0}")]
    Io(#[from] std::io::Error),
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
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

async fn connect(url: Option<&str>) -> Result<PgEventStore, ServerError> {
    let url = url.ok_or_else(|| ServerError::Arg("DATABASE_URL is not set".into()))?;
    Ok(PgEventStore::connect(url).await?)
}

/// One arm per subcommand: a flat dispatcher reads better than a hierarchy here.
#[allow(clippy::too_many_lines)]
async fn run(cli: Cli) -> Result<(), ServerError> {
    let presets_dir = cli
        .presets
        .unwrap_or_else(|| PathBuf::from(isms_core::WORKSPACE_PRESETS_DIR));
    match cli.cmd {
        Cmd::Openapi => {
            let doc = isms_server::api::openapi();
            println!(
                "{}",
                doc.to_pretty_json()
                    .map_err(|e| ServerError::Arg(e.to_string()))?
            );
        }
        Cmd::Migrate => {
            // Create the database when it is missing (dev and e2e databases are made this way).
            let url = cli
                .database_url
                .as_deref()
                .ok_or_else(|| ServerError::Arg("DATABASE_URL is not set".into()))?;
            use sqlx::migrate::MigrateDatabase;
            if !sqlx::Postgres::database_exists(url).await? {
                sqlx::Postgres::create_database(url).await?;
                tracing::info!("database created");
            }
            let store = connect(Some(url)).await?;
            store.migrate().await?;
            tracing::info!("migrations applied");
        }
        Cmd::Rebuild { society } => {
            let store = connect(cli.database_url.as_deref()).await?;
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
            let store = connect(cli.database_url.as_deref()).await?;
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
        Cmd::Invite { count } => {
            let store = connect(cli.database_url.as_deref()).await?;
            for _ in 0..count {
                let code = format!(
                    "{}-{}",
                    isms_server::auth::short_id(4),
                    isms_server::auth::short_id(4)
                );
                store.create_invite_code(&code, None).await?;
                println!("{code}");
            }
        }
        Cmd::Session { email } => {
            let store = connect(cli.database_url.as_deref()).await?;
            let account = store.ensure_account(email.trim()).await?;
            let token = isms_server::auth::random_token();
            store
                .create_session(
                    &isms_server::auth::hash_token(&token),
                    account.id,
                    isms_server::auth::expiry(isms_server::auth::SESSION_TTL),
                )
                .await?;
            println!("{token}");
        }
        Cmd::RebuildProjections { society } => {
            rebuild_projections(cli.database_url.as_deref(), &presets_dir, society).await?;
        }
        Cmd::Serve {
            verify_replay,
            bind,
            base_url,
        } => {
            serve(
                cli.database_url.as_deref(),
                presets_dir,
                verify_replay,
                &bind,
                base_url,
            )
            .await?;
        }
    }
    Ok(())
}

async fn rebuild_projections(
    database_url: Option<&str>,
    presets_dir: &std::path::Path,
    society: i64,
) -> Result<(), ServerError> {
    let store = connect(database_url).await?;
    let row = store
        .list_societies()
        .await?
        .into_iter()
        .find(|r| r.id == society)
        .ok_or_else(|| ServerError::Arg(format!("no society {society}")))?;
    let templates = isms_server::chronicle::Templates::load(presets_dir, &row.preset)
        .map_err(ServerError::Arg)?;
    let n = isms_server::chronicle::rebuild(&store, society, templates).await?;
    tracing::info!(society, headlines = n, "chronicle rebuilt");
    Ok(())
}

async fn serve(
    database_url: Option<&str>,
    presets_dir: PathBuf,
    verify_replay: bool,
    bind: &str,
    base_url: String,
) -> Result<(), ServerError> {
    let store = connect(database_url).await?;
    let mail: Arc<dyn isms_server::mail::MailSender> = Arc::from(isms_server::mail::from_env()?);
    let runtime = Runtime::start(
        &store,
        StartOptions {
            verify_replay,
            presets_dir: Some(presets_dir.clone()),
        },
    )
    .await?;
    let state = AppState::new(
        store.clone(),
        AppState::entries_from(&runtime),
        presets_dir,
        mail,
        base_url.trim_end_matches('/').to_owned(),
    );
    let app = isms_server::api::router(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(societies = runtime.societies.len(), %bind, "serving; Ctrl-C to stop");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    tracing::info!("shutting down");
    runtime.shutdown().await?;
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
