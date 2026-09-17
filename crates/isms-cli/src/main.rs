//! `isms`: a scriptable client for the public API (TDD 10.4). It exists so
//! end-to-end tests are shell scripts, so a coding agent can play the game,
//! and so player-agent authors have a reference client. Amounts are entered
//! and shown in credits and sent as cents. `--json` prints responses raw.

use clap::{Parser, Subcommand};
use isms_api_types::chronicle::{ChronicleView, MessageView, MessagesView};
use isms_api_types::society::{
    BookView, BooksView, Committed, ContractsView, HomeView, NoticeBoardView, OrgsView,
    PayslipsView, PlanView, StreamFrame,
};
use isms_api_types::{ApiKeyCreated, Joined, Me, Problem, SocietyList};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "isms", about = "Isms command-line client")]
struct Cli {
    /// Server origin (default: `ISMS_URL` or the saved config).
    #[arg(long, env = "ISMS_URL")]
    url: Option<String>,
    /// Society id (default: `ISMS_SOCIETY` or the one you last joined).
    #[arg(long, env = "ISMS_SOCIETY")]
    society: Option<i64>,
    /// Print the raw JSON response.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Save credentials: a browser session token (from `isms-server session`) or an API key.
    Login {
        /// A session token is URL-safe base64, so one in 64 starts with a hyphen.
        #[arg(long, allow_hyphen_values = true)]
        session: Option<String>,
        #[arg(long)]
        key: Option<String>,
    },
    /// Who am I.
    Me,
    /// Every society on the server.
    Societies,
    /// Join a society with a handle (and make it the default).
    Join {
        society: i64,
        #[arg(long)]
        handle: String,
    },
    /// Make a society the default.
    Use { society: i64 },
    /// The Situation view.
    Home,
    /// The notice board.
    Board,
    /// Accept an offer (job, sale, credit, lease).
    Accept { offer: u32 },
    /// Labor: `isms labor set --workplace 3 --hours 8 --effort normal`.
    Labor {
        #[command(subcommand)]
        cmd: LaborCmd,
    },
    /// Standing plan: `isms plan set --food 24 --balance 100`.
    Plan {
        #[command(subcommand)]
        cmd: PlanCmd,
    },
    /// Your payslips.
    Payslips,
    /// Order books, or one book.
    Books { instrument: Option<String> },
    /// Place an order: `isms order bid food 10 @ 1.30` (credits per unit).
    Order {
        side: String,
        instrument: String,
        qty: u32,
        /// Literal `@`.
        at: String,
        price: String,
    },
    /// Organizations: `isms org found firm "Iron & Sons" --workplace mine`, `isms org offer ...`.
    Org {
        #[command(subcommand)]
        cmd: OrgCmd,
    },
    /// Your contracts.
    Contracts,
    /// One cycle's Chronicle.
    Chronicle {
        #[arg(long)]
        cycle: Option<u32>,
    },
    /// Post in a channel: `isms say square "hello"`, `isms say org:3 "..."`, `isms say dm:7 "..."`.
    Say { channel: String, body: String },
    /// Read a channel.
    Read { channel: String },
    /// API keys: `isms key create --label agent`.
    Key {
        #[command(subcommand)]
        cmd: KeyCmd,
    },
    /// Tail the live event stream.
    Watch,
}

#[derive(Subcommand, Debug)]
enum LaborCmd {
    Set {
        #[arg(long)]
        workplace: u32,
        #[arg(long)]
        hours: u8,
        #[arg(long, default_value = "normal")]
        effort: String,
    },
}

#[derive(Subcommand, Debug)]
enum PlanCmd {
    Set {
        /// Keep at least this much Food in the pantry.
        #[arg(long)]
        food: Option<u32>,
        /// Keep at least this balance, in credits.
        #[arg(long)]
        balance: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum OrgCmd {
    List,
    Found {
        kind: String,
        name: String,
        #[arg(long)]
        workplace: Option<String>,
    },
    /// Manager: post a job offer.
    Offer {
        org: u32,
        #[arg(long)]
        workplace: u32,
        /// Hourly wage in credits.
        #[arg(long)]
        hourly: String,
        #[arg(long, default_value_t = 8)]
        max_hours: u8,
        #[arg(long, default_value_t = 1)]
        places: u32,
    },
}

#[derive(Subcommand, Debug)]
enum KeyCmd {
    Create {
        #[arg(long, default_value = "cli")]
        label: String,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    url: Option<String>,
    session: Option<String>,
    key: Option<String>,
    society: Option<i64>,
}

fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("ISMS_CONFIG") {
        return PathBuf::from(p);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("isms")
        .join("config.toml")
}

fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(c: &Config) -> Result<(), CliError> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(
        &path,
        toml::to_string(c).map_err(|e| CliError::Other(e.to_string()))?,
    )?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Other(String),
    #[error("not signed in: run `isms login --session <token>` or `--key <key>`")]
    NotSignedIn,
    #[error("no society chosen: `isms join <id> --handle <h>` or `isms use <id>`")]
    NoSociety,
    #[error("{status} {title}{detail}{code}")]
    Api {
        status: u16,
        title: String,
        detail: String,
        code: String,
    },
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// `1.30` -> 130 cents.
fn credits_to_cents(s: &str) -> Result<i64, CliError> {
    let s = s.trim();
    let (whole, frac) = s.split_once('.').unwrap_or((s, "0"));
    let whole: i64 = whole
        .parse()
        .map_err(|_| CliError::Other(format!("not an amount in credits: {s}")))?;
    let frac = format!("{frac:0<2}");
    let frac: i64 = frac[..2]
        .parse()
        .map_err(|_| CliError::Other(format!("not an amount in credits: {s}")))?;
    Ok(whole * 100 + frac)
}

fn credits(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, (cents % 100).abs())
}

struct Client {
    http: reqwest::Client,
    url: String,
    session: Option<String>,
    key: Option<String>,
    json: bool,
}

impl Client {
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, CliError> {
        let mut r = self.http.request(method, format!("{}{path}", self.url));
        if let Some(k) = &self.key {
            r = r.bearer_auth(k);
        } else if let Some(s) = &self.session {
            r = r
                .header("Cookie", format!("isms_session={s}"))
                .header("X-Requested-With", "isms");
        } else {
            return Err(CliError::NotSignedIn);
        }
        Ok(r)
    }

    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        r: reqwest::RequestBuilder,
    ) -> Result<T, CliError> {
        let response = r.send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            let p: Option<Problem> = serde_json::from_str(&text).ok();
            return Err(match p {
                Some(p) => CliError::Api {
                    status: p.status,
                    title: p.title,
                    detail: p.detail.map(|d| format!(": {d}")).unwrap_or_default(),
                    code: p.code.map(|c| format!(" [{c:?}]")).unwrap_or_default(),
                },
                None => CliError::Api {
                    status: status.as_u16(),
                    title: text,
                    detail: String::new(),
                    code: String::new(),
                },
            });
        }
        if self.json {
            let v: serde_json::Value = serde_json::from_str(&text)?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        self.send(self.request(reqwest::Method::GET, path)?).await
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, CliError> {
        self.send(self.request(reqwest::Method::POST, path)?.json(body))
            .await
    }

    async fn put<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, CliError> {
        self.send(self.request(reqwest::Method::PUT, path)?.json(body))
            .await
    }
}

fn print_committed(c: &Committed, json: bool) {
    if json {
        return;
    }
    for e in &c.events {
        println!("{} #{}", e.kind, e.seq);
    }
    if c.events.is_empty() {
        println!("ok (no events)");
    }
}

fn channel_path(id: i64, channel: &str) -> String {
    if let Some(c) = channel.strip_prefix("dm:") {
        format!("/s/{id}/dm/{c}")
    } else {
        format!("/s/{id}/channels/{channel}/messages")
    }
}

/// One arm per subcommand: a flat dispatcher reads better than a hierarchy here.
#[allow(clippy::too_many_lines)]
async fn run_society(client: &Client, id: i64, cmd: Cmd, json: bool) -> Result<(), CliError> {
    match cmd {
        Cmd::Home => {
            let h: HomeView = client.get(&format!("/s/{id}/home")).await?;
            if !json {
                println!(
                    "{} | epoch {} cycle {} tick {}/{}",
                    h.citizen.handle,
                    h.clock.epoch,
                    h.clock.cycle,
                    h.clock.tick,
                    h.clock.ticks_per_cycle
                );
                println!(
                    "needs  food {:.1}  shelter {:.1}  comfort {:.1}",
                    h.needs.food, h.needs.shelter, h.needs.comfort
                );
                println!(
                    "balance {} cr  pantry {:?}",
                    credits(h.household.balance),
                    h.household.pantry
                );
                println!(
                    "dwelling {}",
                    h.household
                        .dwelling
                        .map_or("none".to_owned(), |d| format!("#{}", d.id))
                );
                for a in &h.labor.allocations {
                    println!(
                        "work  {} ({:?}) {} h {:?}",
                        a.org_name, a.kind, a.hours, a.effort
                    );
                }
                println!("since last seen: {} events", h.since_last_seen.events.len());
                for hl in &h.headlines {
                    println!("  chronicle: {}", hl.text);
                }
            }
        }
        Cmd::Board => {
            let b: NoticeBoardView = client.get(&format!("/s/{id}/notice-board")).await?;
            if !json {
                for o in b.offers {
                    println!("#{}  {}  {}", o.id, o.kind, o.body);
                }
            }
        }
        Cmd::Accept { offer } => {
            let c: Committed = client
                .post(
                    &format!("/s/{id}/offers/{offer}/accept"),
                    &serde_json::json!({}),
                )
                .await?;
            print_committed(&c, json);
        }
        Cmd::Labor {
            cmd:
                LaborCmd::Set {
                    workplace,
                    hours,
                    effort,
                },
        } => {
            let c: Committed = client
                .put(
                    &format!("/s/{id}/labor"),
                    &serde_json::json!({ "allocations": [{ "workplace": workplace, "hours": hours, "effort": effort }] }),
                )
                .await?;
            print_committed(&c, json);
        }
        Cmd::Plan {
            cmd: PlanCmd::Set { food, balance },
        } => {
            let current: PlanView = client.get(&format!("/s/{id}/plan")).await?;
            let mut plan = current.plan;
            if let Some(f) = food {
                plan["keep_food_at_least"] = serde_json::json!(f);
            }
            if let Some(b) = balance {
                plan["keep_balance_at_least"] = serde_json::json!(credits_to_cents(&b)?);
            }
            let c: Committed = client
                .put(
                    &format!("/s/{id}/plan"),
                    &serde_json::json!({ "plan": plan }),
                )
                .await?;
            print_committed(&c, json);
        }
        Cmd::Payslips => {
            let p: PayslipsView = client.get(&format!("/s/{id}/payslips")).await?;
            if !json {
                for s in p.payslips {
                    let amount = s.payload["Paid"]["amount"].as_i64().unwrap_or(0);
                    println!("tick {}  {} cr  event #{}", s.tick, credits(amount), s.seq);
                }
            }
        }
        Cmd::Books { instrument: None } => {
            let b: BooksView = client.get(&format!("/s/{id}/books")).await?;
            if !json {
                for s in b.books {
                    println!(
                        "{:<12} last {:>8}  bid {:>8} x{}  ask {:>8} x{}",
                        s.instrument,
                        s.last_price.map_or("-".into(), credits),
                        s.best_bid.map_or("-".into(), credits),
                        s.bid_depth,
                        s.best_ask.map_or("-".into(), credits),
                        s.ask_depth
                    );
                }
            }
        }
        Cmd::Books {
            instrument: Some(i),
        } => {
            let b: BookView = client.get(&format!("/s/{id}/books/{i}")).await?;
            if !json {
                for l in &b.asks {
                    println!("ask {:>8} x{}", credits(l.price), l.qty);
                }
                for l in &b.bids {
                    println!("bid {:>8} x{}", credits(l.price), l.qty);
                }
                for o in &b.my_orders {
                    println!(
                        "mine #{} {} {}/{} @ {}",
                        o.id,
                        o.side,
                        o.remaining,
                        o.qty,
                        credits(o.limit_price)
                    );
                }
            }
        }
        Cmd::Order {
            side,
            instrument,
            qty,
            at,
            price,
        } => {
            if at != "@" {
                return Err(CliError::Other(
                    "usage: isms order bid food 10 @ 1.30".into(),
                ));
            }
            let c: Committed = client
                .post(
                    &format!("/s/{id}/orders"),
                    &serde_json::json!({ "instrument": instrument, "side": side, "qty": qty, "limit_price": credits_to_cents(&price)? }),
                )
                .await?;
            print_committed(&c, json);
        }
        Cmd::Org { cmd: OrgCmd::List } => {
            let o: OrgsView = client.get(&format!("/s/{id}/orgs")).await?;
            if !json {
                for org in o.orgs {
                    println!(
                        "#{:<4} {:<28} {:?}  treasury {}  employees {}{}",
                        org.id,
                        org.name,
                        org.kind,
                        credits(org.treasury),
                        org.employees,
                        if org.i_manage { "  (you manage)" } else { "" }
                    );
                }
            }
        }
        Cmd::Org {
            cmd:
                OrgCmd::Found {
                    kind,
                    name,
                    workplace,
                },
        } => {
            let body = serde_json::json!({
                "kind": kind,
                "name": name,
                "first_workplace": workplace.map(|w| serde_json::json!({ "kind": w })),
            });
            let c: Committed = client.post(&format!("/s/{id}/orgs"), &body).await?;
            print_committed(&c, json);
            if !json && let Some(org) = c.events.iter().find(|e| e.kind == "OrgFounded") {
                println!("org {}", org.payload["OrgFounded"]["org"]);
            }
        }
        Cmd::Org {
            cmd:
                OrgCmd::Offer {
                    org,
                    workplace,
                    hourly,
                    max_hours,
                    places,
                },
        } => {
            let body = serde_json::json!({
                "workplace": workplace,
                "pay": { "hourly": credits_to_cents(&hourly)? },
                "max_hours": max_hours,
                "term_cycles": null,
                "notice_cycles": 1,
                "places": places,
            });
            let c: Committed = client
                .post(&format!("/s/{id}/orgs/{org}/offers"), &body)
                .await?;
            print_committed(&c, json);
        }
        Cmd::Contracts => {
            let k: ContractsView = client.get(&format!("/s/{id}/contracts")).await?;
            if !json {
                for c in k.contracts {
                    println!("#{} {} {} {}", c.id, c.status, c.role, c.body);
                }
            }
        }
        Cmd::Chronicle { cycle } => {
            let q = cycle.map(|c| format!("?cycle={c}")).unwrap_or_default();
            let ch: ChronicleView = client.get(&format!("/s/{id}/chronicle{q}")).await?;
            if !json {
                println!("The Chronicle, cycle {}", ch.cycle);
                for h in ch.headlines {
                    println!("  tick {:>3}  {}", h.tick, h.text);
                }
            }
        }
        Cmd::Say { channel, body } => {
            let m: MessageView = client
                .post(
                    &channel_path(id, &channel),
                    &serde_json::json!({ "body": body }),
                )
                .await?;
            if !json {
                println!("#{} {}: {}", m.id, m.handle, m.body);
            }
        }
        Cmd::Read { channel } => {
            let m: MessagesView = client.get(&channel_path(id, &channel)).await?;
            if !json {
                for msg in m.messages {
                    println!("#{} tick {} {}: {}", msg.id, msg.tick, msg.handle, msg.body);
                }
            }
        }
        Cmd::Key {
            cmd: KeyCmd::Create { label },
        } => {
            let k: ApiKeyCreated = client
                .post(
                    "/me/api-keys",
                    &serde_json::json!({ "society_id": id, "label": label }),
                )
                .await?;
            if !json {
                println!("{}", k.key);
            }
        }
        Cmd::Watch => watch(client, id).await?,
        Cmd::Login { .. } | Cmd::Use { .. } | Cmd::Me | Cmd::Societies | Cmd::Join { .. } => {
            unreachable!("handled before a society is needed")
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn run(cli: Cli) -> Result<(), CliError> {
    let mut config = load_config();
    let json = cli.json;
    match &cli.cmd {
        Cmd::Login { session, key } => {
            if session.is_none() && key.is_none() {
                return Err(CliError::Other("give --session or --key".into()));
            }
            if let Some(u) = &cli.url {
                config.url = Some(u.clone());
            }
            if let Some(s) = session {
                config.session = Some(s.clone());
                config.key = None;
            }
            if let Some(k) = key {
                config.key = Some(k.clone());
            }
            save_config(&config)?;
            println!("saved {}", config_path().display());
            return Ok(());
        }
        Cmd::Use { society } => {
            config.society = Some(*society);
            save_config(&config)?;
            println!("default society {society}");
            return Ok(());
        }
        _ => {}
    }
    let url = cli
        .url
        .clone()
        .or_else(|| config.url.clone())
        .unwrap_or_else(|| "http://localhost:8080".into());
    let client = Client {
        http: reqwest::Client::new(),
        url: url.trim_end_matches('/').to_owned(),
        session: std::env::var("ISMS_SESSION")
            .ok()
            .or_else(|| config.session.clone()),
        key: std::env::var("ISMS_KEY")
            .ok()
            .or_else(|| config.key.clone()),
        json,
    };
    match cli.cmd {
        Cmd::Me => {
            let me: Me = client.get("/me").await?;
            if !json {
                println!("{} (account {})", me.account.email, me.account.id);
                for c in me.citizenships {
                    println!(
                        "  society {}: citizen {} \"{}\"",
                        c.society_id, c.citizen_id, c.handle
                    );
                }
            }
            Ok(())
        }
        Cmd::Societies => {
            let list: SocietyList = client.get("/societies").await?;
            if !json {
                for s in list.societies {
                    println!(
                        "{}  {}  {}  epoch {} cycle {} tick {}  {} citizens",
                        s.id,
                        s.name,
                        s.display,
                        s.clock.epoch,
                        s.clock.cycle,
                        s.clock.tick,
                        s.population
                    );
                }
            }
            Ok(())
        }
        Cmd::Join { society, handle } => {
            let j: Joined = client
                .post(
                    &format!("/societies/{society}/join"),
                    &serde_json::json!({ "handle": handle }),
                )
                .await?;
            config.society = Some(society);
            save_config(&config)?;
            if !json {
                println!(
                    "citizen {} \"{}\" in society {}{}",
                    j.citizen_id,
                    j.handle,
                    society,
                    if j.created { "" } else { " (already)" }
                );
            }
            Ok(())
        }
        other => {
            let id = cli.society.or(config.society).ok_or(CliError::NoSociety)?;
            run_society(&client, id, other, json).await
        }
    }
}

async fn watch(client: &Client, society: i64) -> Result<(), CliError> {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let ws_url = client
        .url
        .replacen("http://", "ws://", 1)
        .replacen("https://", "wss://", 1);
    let mut req = format!("{ws_url}/s/{society}/stream")
        .into_client_request()
        .map_err(|e| CliError::Other(e.to_string()))?;
    if let Some(k) = &client.key {
        req.headers_mut().insert(
            "Authorization",
            format!("Bearer {k}")
                .parse()
                .map_err(|_| CliError::Other("bad key".into()))?,
        );
    } else if let Some(s) = &client.session {
        req.headers_mut().insert(
            "Cookie",
            format!("isms_session={s}")
                .parse()
                .map_err(|_| CliError::Other("bad session".into()))?,
        );
    } else {
        return Err(CliError::NotSignedIn);
    }
    let (mut socket, _) = tokio_tungstenite::connect_async(req)
        .await
        .map_err(|e| CliError::Other(e.to_string()))?;
    while let Some(msg) = socket.next().await {
        let msg = msg.map_err(|e| CliError::Other(e.to_string()))?;
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            if client.json {
                println!("{text}");
            } else if let Ok(frame) = serde_json::from_str::<StreamFrame>(&text) {
                for e in frame.events {
                    println!("tick {} #{} {}", frame.clock.engine_tick, e.seq, e.kind);
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_takes_a_session_token_that_starts_with_a_hyphen() {
        let cli = Cli::try_parse_from(["isms", "login", "--session", "-Ab_9"]).expect("parses");
        assert!(matches!(cli.cmd, Cmd::Login { session: Some(ref s), .. } if s == "-Ab_9"));
    }

    #[test]
    fn credits_round_trip() {
        assert_eq!(credits_to_cents("1.30").unwrap(), 130);
        assert_eq!(credits_to_cents("8").unwrap(), 800);
        assert_eq!(credits_to_cents("0.5").unwrap(), 50);
        assert_eq!(credits(130), "1.30");
        assert_eq!(credits(-5), "0.05");
        assert!(credits_to_cents("x").is_err());
    }
}
