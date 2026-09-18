//! S1.4 done gate: one integration test per in-society endpoint; the
//! visibility rules on a medium-monitoring fixture (the manager sees the
//! attributed figure, the worker sees `true_output`; equal where sigma = 0);
//! the stream delivers a `Trade` to both parties within one tick.

use axum_test::TestServer;
use isms_api_types::society::{
    BookView, BooksView, CitizensView, Committed, ContractsView, DigestView, ExplainView, HomeView,
    HouseholdersView, NoticeBoardView, OrgLedgerView, OrgView, OrgsView, PayslipsView, PlanView,
    PricesView, ScoreboardView, StatsView, StreamFrame,
};
use isms_api_types::{Joined, Problem, RejectCode};
use isms_core::WORKSPACE_PRESETS_DIR;
use isms_core::command::{Command, Envelope};
use isms_core::event::Actor;
use isms_core::ids::CitizenId;
use isms_core::kinds::{ClientKind, Good};
use isms_core::ledger::{Asset, Party};
use isms_server::actor::{SocietyHandle, spawn};
use isms_server::mail::MemorySender;
use isms_server::runtime::{SeedSpec, seed_society};
use isms_server::scheduler::{Control, Schedule};
use isms_server::state::{AppState, SocietyEntry};
use isms_store::{PgEventStore, load_world};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BASE: &str = "http://test";

struct Fixture {
    router: axum::Router,
    handle: SocietyHandle,
    mail: Arc<MemorySender>,
    store: PgEventStore,
    id: i64,
    invites: std::sync::Mutex<u32>,
}

/// A signed-in, joined client.
struct Client {
    server: TestServer,
    citizen: u32,
}

async fn fixture(pool: PgPool, preset: &str) -> Fixture {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let store = PgEventStore::from_pool(pool);
    let row = seed_society(
        &store,
        Path::new(WORKSPACE_PRESETS_DIR),
        &SeedSpec {
            name: format!("{}-1", preset.replace('/', "-")),
            preset: preset.into(),
            class: "canonical".into(),
            seed: 1,
            tick_seconds: 3600,
            cycle_boundary_hour: 4,
            overrides: vec![(
                "params.population.collapse_enabled".to_owned(),
                toml::Value::Boolean(false),
            )],
        },
    )
    .await
    .unwrap();
    let loaded = load_world(&store, row.id).await.unwrap();
    let (handle, task) = spawn(row.id, store.clone(), loaded);
    std::mem::forget(task);
    let mut societies = BTreeMap::new();
    societies.insert(
        row.id,
        SocietyEntry {
            row: row.clone(),
            handle: handle.clone(),
            control: Arc::new(Control::new(
                Schedule {
                    tick_seconds: 3600,
                    tick_origin: row.tick_origin,
                },
                false,
            )),
        },
    );
    let mail = Arc::new(MemorySender::default());
    let state = AppState::new(
        store.clone(),
        societies,
        PathBuf::from(WORKSPACE_PRESETS_DIR),
        mail.clone(),
        BASE.into(),
        std::collections::BTreeSet::new(),
    );
    Fixture {
        router: isms_server::api::router(state),
        handle,
        mail,
        store,
        id: row.id,
        invites: std::sync::Mutex::new(0),
    }
}

impl Fixture {
    async fn tick(&self, n: u32) {
        for _ in 0..n {
            self.handle.tick().await.unwrap();
        }
    }

    /// Sign in a new account (invite minted here) and join with `handle`.
    async fn client(&self, handle: &str) -> Client {
        let n = {
            let mut g = self.invites.lock().unwrap();
            *g += 1;
            *g
        };
        let email = format!("{handle}@example.test");
        let code = format!("code-{n}");
        self.store.create_invite_code(&code, None).await.unwrap();
        let server = TestServer::builder()
            .save_cookies()
            .http_transport()
            .build(self.router.clone())
            .unwrap();
        let r = server
            .post("/auth/magic-link")
            .json(&json!({ "email": email, "invite_code": code }))
            .await;
        assert_eq!(r.status_code(), 202, "{}", r.text());
        let url = self.mail.last_for(&email).unwrap();
        let r = server.get(url.strip_prefix(BASE).unwrap()).await;
        assert_eq!(r.status_code(), 303);
        let r = server
            .post(&format!("/societies/{}/join", self.id))
            .add_header("x-requested-with", "isms")
            .json(&json!({ "handle": handle }))
            .await;
        assert_eq!(r.status_code(), 201, "{}", r.text());
        let joined: Joined = r.json();
        Client {
            server,
            citizen: joined.citizen_id,
        }
    }

    /// Move goods from legacy orgs to a citizen, as their householder managers
    /// could (engine commands straight to the actor: the test crane).
    /// Ticks forward until enough of the good exists somewhere.
    async fn grant(&self, good: Good, qty: u32, to: u32) {
        let mut remaining = qty;
        for _ in 0..200 {
            let sources: Vec<(isms_core::ids::OrgId, CitizenId, u32)> = {
                let w = self.handle.world.read().await;
                let mut v: Vec<_> = w
                    .orgs
                    .values()
                    .filter(|o| o.name.starts_with("Legacy") && o.manager.is_some())
                    .map(|o| {
                        (
                            o.id,
                            o.manager.unwrap(),
                            o.inventory.get(&good).copied().unwrap_or(0),
                        )
                    })
                    .filter(|(_, _, held)| *held > 0)
                    .collect();
                v.sort_by(|a, b| b.2.cmp(&a.2));
                v
            };
            for (org, manager, held) in sources {
                if remaining == 0 {
                    return;
                }
                let take = held.min(remaining);
                let tick = self.handle.world.read().await.meta.tick;
                let env = Envelope {
                    actor: Actor::Citizen(manager),
                    on_behalf_of: Some(org),
                    client_kind: ClientKind::Sim,
                    received_at_tick: tick,
                    command: Command::Transfer {
                        to: Party::Citizen(CitizenId(to)),
                        asset: Asset::Good(good, take),
                        memo: "test grant".into(),
                    },
                };
                if self.handle.command(env).await.unwrap().is_ok() {
                    remaining -= take;
                }
            }
            if remaining == 0 {
                return;
            }
            self.tick(1).await;
        }
        panic!("could not gather {qty} {good:?} from the legacy orgs");
    }
}

fn p(id: i64, rest: &str) -> String {
    format!("/s/{id}{rest}")
}

impl Client {
    async fn get<T: serde::de::DeserializeOwned>(&self, id: i64, rest: &str) -> T {
        let r = self.server.get(&p(id, rest)).await;
        assert_eq!(r.status_code(), 200, "GET {rest}: {}", r.text());
        r.json()
    }
    async fn post(&self, id: i64, rest: &str, body: Value) -> axum_test::TestResponse {
        self.server
            .post(&p(id, rest))
            .add_header("x-requested-with", "isms")
            .json(&body)
            .await
    }
    async fn put(&self, id: i64, rest: &str, body: Value) -> axum_test::TestResponse {
        self.server
            .put(&p(id, rest))
            .add_header("x-requested-with", "isms")
            .json(&body)
            .await
    }
    async fn delete(&self, id: i64, rest: &str) -> axum_test::TestResponse {
        self.server
            .delete(&p(id, rest))
            .add_header("x-requested-with", "isms")
            .await
    }
    async fn ok(&self, id: i64, rest: &str, body: Value) -> Committed {
        let r = self.post(id, rest, body).await;
        assert_eq!(r.status_code(), 200, "POST {rest}: {}", r.text());
        r.json()
    }
}

fn kinds(c: &Committed) -> Vec<&str> {
    c.events.iter().map(|e| e.kind.as_str()).collect()
}

fn reject(r: &axum_test::TestResponse) -> RejectCode {
    assert_eq!(r.status_code(), 422, "{}", r.text());
    let p: Problem = r.json();
    p.code.expect("a reject code")
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn home_plan_labor_payslips_digest(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("marlow").await;
    let home: HomeView = a.get(f.id, "/home").await;
    assert_eq!(home.citizen.handle, "marlow");
    assert!((home.needs.food - 100.0).abs() < f64::EPSILON);
    assert!(home.household.balance > 0);
    assert_eq!(home.plan["keep_food_at_least"], 24);
    assert!(home.labor.allocations.is_empty());
    assert_eq!(home.society.population, 41);
    // Non-citizens are refused.
    let other = TestServer::new(f.router.clone()).unwrap();
    assert_eq!(
        other.get(&format!("/s/{}/home", f.id)).await.status_code(),
        401
    );

    // Plan round trip.
    let mut plan = home.plan.clone();
    plan["keep_food_at_least"] = json!(30);
    let c: Committed = {
        let r = a.put(f.id, "/plan", json!({ "plan": plan })).await;
        assert_eq!(r.status_code(), 200, "{}", r.text());
        r.json()
    };
    assert_eq!(kinds(&c), vec!["PlanChanged"]);
    assert!(c.first_seq.is_some());
    let p: PlanView = a.get(f.id, "/plan").await;
    assert_eq!(p.plan["keep_food_at_least"], 30);

    // Legacy firms post their offers during the first tick.
    f.tick(1).await;
    let board: NoticeBoardView = a.get(f.id, "/notice-board").await;
    let job = board
        .offers
        .iter()
        .find(|o| o.kind == "employment")
        .expect("a legacy job offer");
    let c = a
        .ok(f.id, &format!("/offers/{}/accept", job.id), json!({}))
        .await;
    assert!(kinds(&c).contains(&"EmploymentAccepted"), "{:?}", kinds(&c));
    let workplace = job.body["employment"]["workplace"].as_u64().unwrap();
    let r = a
        .put(
            f.id,
            "/labor",
            json!({ "allocations": [{ "workplace": workplace, "hours": 9, "effort": "normal" }] }),
        )
        .await;
    let code = reject(&r);
    assert!(
        matches!(code, RejectCode::OverContractHours | RejectCode::OverBudget),
        "{code:?}"
    );
    let r = a
        .put(
            f.id,
            "/labor",
            json!({ "allocations": [{ "workplace": workplace, "hours": 8, "effort": "normal" }] }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let home: HomeView = a.get(f.id, "/home").await;
    assert_eq!(home.labor.allocations.len(), 1);
    assert_eq!(home.labor.employment.len(), 1);

    // A full cycle: Home shows what touched me while away (read it first: any
    // read counts as presence and moves the digest window), then the payslip.
    f.tick(24).await;
    let home: HomeView = a.get(f.id, "/home").await;
    assert!(home.since_last_seen.events.iter().any(|e| e.kind == "Paid"));
    assert_eq!(home.clock.cycle, 2);
    let slips: PayslipsView = a.get(f.id, "/payslips").await;
    assert!(!slips.payslips.is_empty());
    let slip = slips.payslips.last().unwrap();
    assert_eq!(slip.kind, "Paid");
    let ex: ExplainView = a.get(f.id, &format!("/explain/{}", slip.seq)).await;
    assert!(!ex.explains.is_empty());
    assert!(ex.explains[0]["rule"].is_string());
    let d: DigestView = a.get(f.id, "/away-digest?since=0").await;
    assert!(d.events.iter().any(|e| e.kind == "Paid"));
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn market_books_orders_prices(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("trader").await;
    f.tick(2).await;
    let books: BooksView = a.get(f.id, "/books").await;
    let food = books.books.iter().find(|b| b.instrument == "food").unwrap();
    let last = food.last_price.expect("food has a price from the start");
    let r = a
        .post(
            f.id,
            "/orders",
            json!({ "instrument": "food", "side": "bid", "qty": 2, "limit_price": last * 2 }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let c: Committed = r.json();
    assert_eq!(c.events[0].kind, "OrderPlaced");
    let oid = c.events[0].payload["OrderPlaced"]["order"]["id"]
        .as_u64()
        .expect("order id");
    let book: BookView = a.get(f.id, "/books/food").await;
    assert!(!book.asks.is_empty() || !book.tape.is_empty());
    // Either the bid rests (cancel releases it) or it crossed a legacy ask.
    let r = a.delete(f.id, &format!("/orders/{oid}")).await;
    if r.status_code() == 200 {
        let c: Committed = r.json();
        assert_eq!(c.events[0].kind, "OrderCancelled");
    } else {
        assert_eq!(reject(&r), RejectCode::UnknownOrder);
        assert!(book.tape.iter().any(|t| t.kind == "Trade"));
    }
    let r = a
        .post(
            f.id,
            "/orders",
            json!({ "instrument": "share:999", "side": "ask", "qty": 1, "limit_price": 1 }),
        )
        .await;
    assert_eq!(r.status_code(), 422);
    let r = a
        .post(
            f.id,
            "/orders",
            json!({ "instrument": "gold", "side": "ask", "qty": 1, "limit_price": 1 }),
        )
        .await;
    assert_eq!(r.status_code(), 400);
    let prices: PricesView = a.get(f.id, "/prices?window=24").await;
    assert_eq!(prices.window, 24);
    assert!(prices.points.iter().any(|p| p.instrument == "food"));
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn orgs_found_and_manage(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("founder").await;
    f.tick(24).await;
    f.grant(Good::Materials, 20, a.citizen).await;
    let r = a
        .post(
            f.id,
            "/orgs",
            json!({ "kind": "firm", "name": "Iron & Sons", "first_workplace": { "kind": "mine" } }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let c: Committed = r.json();
    assert!(kinds(&c).contains(&"OrgFounded"), "{:?}", kinds(&c));
    let oid = c.events[0].payload["OrgFounded"]["org"].as_u64().unwrap();
    let org: OrgView = a.get(f.id, &format!("/orgs/{oid}")).await;
    assert!(org.i_manage);
    assert_eq!(org.my_shares, 100);
    assert_eq!(org.workplaces.len(), 1);
    let wp = org.workplaces[0].id;
    let orgs: OrgsView = a.get(f.id, "/orgs").await;
    assert!(orgs.orgs.iter().any(|o| o.name == "Iron & Sons"));
    let c = a
        .ok(
            f.id,
            &format!("/orgs/{oid}/offers"),
            json!({ "workplace": wp, "pay": { "hourly": 800 }, "max_hours": 8, "term_cycles": null, "notice_cycles": 1, "places": 2 }),
        )
        .await;
    assert!(kinds(&c).contains(&"EmploymentOffered"));
    let c = a
        .ok(f.id, &format!("/orgs/{oid}/shares"), json!({ "qty": 10 }))
        .await;
    assert!(kinds(&c).contains(&"SharesIssued"));
    let r = a
        .post(
            f.id,
            &format!("/orgs/{oid}/machines"),
            json!({ "workplace": wp, "qty": 1, "action": "install" }),
        )
        .await;
    assert_eq!(r.status_code(), 422); // no Machines in the org's inventory
    let r = a
        .post(
            f.id,
            &format!("/orgs/{oid}/workplaces"),
            json!({ "kind": "mine" }),
        )
        .await;
    assert_eq!(r.status_code(), 422); // no Materials left in the org
    let r = a
        .post(
            f.id,
            &format!("/orgs/{oid}/dividend"),
            json!({ "per_share": 1 }),
        )
        .await;
    assert!(
        r.status_code() == 200 || r.status_code() == 422,
        "{}",
        r.text()
    );
    let r = a
        .post(
            f.id,
            &format!("/orgs/{oid}/manager"),
            json!({ "citizen": a.citizen }),
        )
        .await;
    assert!(
        r.status_code() == 200 || r.status_code() == 422,
        "{}",
        r.text()
    );
    // A stranger cannot act for the org.
    let b = f.client("stranger").await;
    let r = b
        .post(f.id, &format!("/orgs/{oid}/offers"), json!({ "workplace": wp, "pay": { "hourly": 1 }, "max_hours": 8, "term_cycles": null, "notice_cycles": 1, "places": 1 }))
        .await;
    assert_eq!(reject(&r), RejectCode::NotManager);
    let r = b.post(f.id, &format!("/orgs/{oid}/join"), json!({})).await;
    assert_eq!(r.status_code(), 422); // firms have no membership
    assert_eq!(
        a.server
            .get(&format!("/s/{}/orgs/9999", f.id))
            .await
            .status_code(),
        404
    );
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn offers_contracts_transfers_dwellings(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("lender").await;
    let b = f.client("borrower").await;
    // Wanted ad up and down.
    let c = a
        .ok(
            f.id,
            "/offers/wanted",
            json!({ "good": "wares", "qty": 2, "max_price": 500 }),
        )
        .await;
    assert_eq!(kinds(&c), vec!["WantedPosted"]);
    let wid = c.events[0].payload["WantedPosted"]["offer"]
        .as_u64()
        .unwrap();
    let r = a.delete(f.id, &format!("/offers/{wid}")).await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    // A sale of what you do not have is refused with the engine's code.
    let r = a
        .post(
            f.id,
            "/offers/sale",
            json!({ "asset": { "good": ["food", 1] }, "price": { "money": 100 } }),
        )
        .await;
    assert_eq!(reject(&r), RejectCode::InsufficientGoods);
    // Credit: offer, accept, see the contract on both sides, transfer money.
    let c = a
        .ok(
            f.id,
            "/offers/credit",
            json!({ "to": { "citizen": b.citizen }, "principal": 1000, "rate_per_cycle_bp": 200, "term_cycles": 5, "collateral": null }),
        )
        .await;
    assert_eq!(kinds(&c), vec!["CreditOffered"]);
    let cid_offer = c.events[0].payload["CreditOffered"]["offer"]
        .as_u64()
        .unwrap();
    let c = b
        .ok(f.id, &format!("/offers/{cid_offer}/accept"), json!({}))
        .await;
    assert!(kinds(&c).contains(&"CreditAccepted"));
    let ka: ContractsView = a.get(f.id, "/contracts").await;
    let kb: ContractsView = b.get(f.id, "/contracts").await;
    assert_eq!(ka.contracts.len(), 1);
    assert_eq!(kb.contracts.len(), 1);
    assert_eq!(ka.contracts[0].role, "party");
    assert!(ka.contracts[0].body.get("credit").is_some());
    let cid = ka.contracts[0].id;
    let r = a
        .post(f.id, &format!("/contracts/{cid}/terminate"), json!({}))
        .await;
    assert_eq!(r.status_code(), 400);
    let c = b
        .ok(
            f.id,
            "/transfers",
            json!({ "to": { "citizen": a.citizen }, "asset": { "money": 100 }, "memo": "thanks" }),
        )
        .await;
    assert_eq!(kinds(&c), vec!["Transferred"]);
    // Housing: take a legacy lease (posted during the first tick); Home shows it.
    f.tick(1).await;
    let board: NoticeBoardView = b.get(f.id, "/notice-board").await;
    let lease = board
        .offers
        .iter()
        .find(|o| o.kind == "lease")
        .expect("a legacy lease offer");
    let c = b
        .ok(f.id, &format!("/offers/{}/accept", lease.id), json!({}))
        .await;
    assert!(kinds(&c).contains(&"LeaseAccepted"));
    let home: HomeView = b.get(f.id, "/home").await;
    let d = home.household.dwelling.expect("housed");
    assert!(d.rent_per_cycle.is_some());
    let r = b
        .post(f.id, &format!("/dwellings/{}/move-in", d.id), json!({}))
        .await;
    assert_eq!(r.status_code(), 422); // a tenant, not the owner
    let kb: ContractsView = b.get(f.id, "/contracts").await;
    let lease_contract = kb
        .contracts
        .iter()
        .find(|k| k.body.get("lease").is_some())
        .unwrap();
    let c = b
        .ok(
            f.id,
            &format!("/contracts/{}/terminate", lease_contract.id),
            json!({}),
        )
        .await;
    assert!(kinds(&c).contains(&"LeaseEnded"));
    // Q107 (D3, D10): an offer that is not on the board is the engine's own
    // unknown_offer, and one that was on it once is said to be taken.
    let r = a.post(f.id, "/offers/424242/accept", json!({})).await;
    assert_eq!(r.status_code(), 422, "{}", r.text());
    let p: Problem = r.json();
    assert_eq!(p.code, Some(RejectCode::UnknownOffer));
    assert_eq!(p.detail.as_deref(), Some("There is no such offer"));
    f.grant(Good::Food, 1, a.citizen).await;
    let c = a
        .ok(
            f.id,
            "/offers/sale",
            json!({ "asset": { "good": ["food", 1] }, "price": { "money": 100 } }),
        )
        .await;
    let sale = c.events[0].payload["SaleOffered"]["offer"]
        .as_u64()
        .unwrap();
    b.ok(f.id, &format!("/offers/{sale}/accept"), json!({}))
        .await;
    let r = b
        .post(f.id, &format!("/offers/{sale}/accept"), json!({}))
        .await;
    assert_eq!(r.status_code(), 422, "{}", r.text());
    let p: Problem = r.json();
    assert_eq!(p.code, Some(RejectCode::UnknownOffer));
    assert_eq!(
        p.detail.as_deref(),
        Some("That offer was taken or withdrawn")
    );
    // D2: every refusal names things for its reader.
    let r = a
        .put(
            f.id,
            "/labor",
            json!({ "allocations": [{ "workplace": 999_999, "hours": 1, "effort": "normal" }] }),
        )
        .await;
    assert_eq!(r.status_code(), 422, "{}", r.text());
    let p: Problem = r.json();
    assert_eq!(p.code, Some(RejectCode::UnknownWorkplace));
    assert_eq!(p.detail.as_deref(), Some("There is no such workplace"));
    let r = a
        .post(
            f.id,
            "/transfers",
            json!({ "to": { "citizen": a.citizen }, "asset": { "money": 1 }, "memo": "" }),
        )
        .await;
    assert_eq!(r.status_code(), 422, "{}", r.text());
    assert_eq!(
        r.json::<Problem>().detail.as_deref(),
        Some("Cannot transfer to yourself")
    );
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn society_pages(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("reader").await;
    let s: StatsView = a.get(f.id, "/stats").await;
    assert!(s.last_cycle.is_none());
    assert_eq!(s.live.population, 41);
    assert_eq!(s.firm_count, 18);
    let c: CitizensView = a.get(f.id, "/citizens").await;
    assert_eq!(c.citizens.len(), 41);
    assert!(c.citizens.iter().any(|z| z.handle == "reader"));
    // A cycle closes: aggregates exist and the counts still agree.
    f.tick(24).await;
    let s: StatsView = a.get(f.id, "/stats").await;
    let last = s.last_cycle.expect("a closed cycle");
    assert!(last["need_fulfillment_rate"].is_number());
    let c: CitizensView = a.get(f.id, "/citizens").await;
    assert_eq!(s.live.population, u32::try_from(c.citizens.len()).unwrap());
    let sb: ScoreboardView = a.get(f.id, "/scoreboard").await;
    assert_eq!(
        sb.rows.len(),
        c.citizens.iter().filter(|z| !z.dormant).count()
    );
    assert!(sb.rows.windows(2).all(|w| w[0].net_worth >= w[1].net_worth));
    let me = sb.rows.iter().find(|r| r.citizen == a.citizen).unwrap();
    assert!(me.self_made <= 0);
    let h: HouseholdersView = a.get(f.id, "/householders").await;
    assert!(h.markdown.to_lowercase().contains("householder"));
    assert_eq!(
        a.server
            .get(&format!("/s/{}/explain/999999", f.id))
            .await
            .status_code(),
        404
    );
}

/// Find the latest `Produced` at `workplace` and return its seq.
async fn last_produced(f: &Fixture, workplace: u64) -> i64 {
    let events = f
        .store
        .read_last_of_kind(f.id, "Produced", 2000)
        .await
        .unwrap();
    events
        .iter()
        .rev()
        .find(|e| {
            let v = serde_json::to_value(&e.event).unwrap();
            v["Produced"]["workplace"] == json!(workplace)
        })
        .map(|e| e.seq)
        .expect("a Produced event at the workplace")
}

/// A human founds a mine and hires another human who works a full-time day.
async fn firm_with_worker(f: &Fixture) -> (Client, Client, u64) {
    let manager = f.client("boss").await;
    let worker = f.client("hand").await;
    f.tick(24).await;
    f.grant(Good::Materials, 20, manager.citizen).await;
    let c = manager
        .ok(
            f.id,
            "/orgs",
            json!({ "kind": "firm", "name": "Deep Cut", "first_workplace": { "kind": "mine" } }),
        )
        .await;
    let oid = c.events[0].payload["OrgFounded"]["org"].as_u64().unwrap();
    let org: OrgView = manager.get(f.id, &format!("/orgs/{oid}")).await;
    let wp = u64::from(org.workplaces[0].id);
    let c = manager
        .ok(
            f.id,
            &format!("/orgs/{oid}/offers"),
            json!({ "workplace": wp, "pay": { "hourly": 800 }, "max_hours": 8, "term_cycles": null, "notice_cycles": 1, "places": 1 }),
        )
        .await;
    let offer = c.events[0].payload["EmploymentOffered"]["offer"]
        .as_u64()
        .unwrap();
    worker
        .ok(f.id, &format!("/offers/{offer}/accept"), json!({}))
        .await;
    let r = worker
        .put(
            f.id,
            "/labor",
            json!({ "allocations": [{ "workplace": wp, "hours": 8, "effort": "normal" }] }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    f.tick(2).await;
    (manager, worker, wp)
}

fn per_worker(ex: &ExplainView) -> Vec<Value> {
    ex.event.payload["Produced"]["per_worker"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn visibility_manager_sees_attributed_worker_sees_true(pool: PgPool) {
    let f = fixture(pool, "test/freeport-medium-monitoring").await;
    let (manager, worker, wp) = firm_with_worker(&f).await;
    let seq = last_produced(&f, wp).await;
    let w: ExplainView = worker.get(f.id, &format!("/explain/{seq}")).await;
    let m: ExplainView = manager.get(f.id, &format!("/explain/{seq}")).await;
    let wv = per_worker(&w);
    let mv = per_worker(&m);
    assert_eq!(wv.len(), 1);
    assert_eq!(mv.len(), 1);
    assert!(
        wv[0]["true_output"].is_number(),
        "worker sees the true figure"
    );
    assert!(wv[0]["attributed_output"].is_number());
    assert!(
        mv[0]["true_output"].is_null(),
        "manager does not see the true figure"
    );
    assert!(mv[0]["attributed_output"].is_number());
    assert_eq!(wv[0]["attributed_output"], mv[0]["attributed_output"]);
    // Somebody else sees the workplace total but nobody's figures.
    let other = f.client("passerby").await;
    let o: ExplainView = other.get(f.id, &format!("/explain/{seq}")).await;
    assert!(per_worker(&o).is_empty());
    assert!(o.event.payload["Produced"]["units"].is_number());
    // The org page hides the manager's per-worker figures from others too.
    let org_id = {
        let w = f.handle.world.read().await;
        w.workplaces
            .get(&isms_core::ids::WorkplaceId(u32::try_from(wp).unwrap()))
            .unwrap()
            .org
            .0
    };
    let seen_by_other: OrgView = other.get(f.id, &format!("/orgs/{org_id}")).await;
    assert!(
        seen_by_other.workplaces[0].workers[0]
            .attributed_this_cycle
            .is_none()
    );
    let seen_by_manager: OrgView = manager.get(f.id, &format!("/orgs/{org_id}")).await;
    assert!(
        seen_by_manager.workplaces[0].workers[0]
            .attributed_this_cycle
            .is_some()
    );
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn visibility_in_real_freeport_the_two_figures_are_equal(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let (manager, worker, wp) = firm_with_worker(&f).await;
    let seq = last_produced(&f, wp).await;
    let w: ExplainView = worker.get(f.id, &format!("/explain/{seq}")).await;
    let m: ExplainView = manager.get(f.id, &format!("/explain/{seq}")).await;
    let wv = per_worker(&w);
    let mv = per_worker(&m);
    assert_eq!(wv[0]["true_output"], wv[0]["attributed_output"]);
    assert_eq!(
        mv[0]["true_output"], mv[0]["attributed_output"],
        "sigma = 0: nothing to hide"
    );
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn stream_delivers_a_trade_to_both_parties(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let seller = f.client("seller").await;
    let buyer = f.client("buyer").await;
    // The seller buys a legacy firm outright from its standing share listing
    // (S0.12: the householder manager steps down); its shares are then the one
    // instrument no householder trades on the book.
    f.tick(1).await;
    let board: NoticeBoardView = seller.get(f.id, "/notice-board").await;
    let listing = board
        .offers
        .iter()
        .filter(|o| o.kind == "sale" && o.body["sale"]["asset"].get("shares").is_some())
        .min_by_key(|o| {
            o.body["sale"]["price"]["money"]
                .as_i64()
                .unwrap_or(i64::MAX)
        })
        .expect("a legacy firm for sale");
    let org = listing.body["sale"]["asset"]["shares"][0].as_u64().unwrap();
    let c = seller
        .ok(f.id, &format!("/offers/{}/accept", listing.id), json!({}))
        .await;
    assert!(kinds(&c).contains(&"SaleAccepted"), "{:?}", kinds(&c));
    assert!(kinds(&c).contains(&"ManagerAppointed"), "{:?}", kinds(&c));
    let instrument = format!("share:{org}");

    let mut ws_seller = seller
        .server
        .get_websocket(&format!("/s/{}/stream", f.id))
        .await
        .into_websocket()
        .await;
    let mut ws_buyer = buyer
        .server
        .get_websocket(&format!("/s/{}/stream", f.id))
        .await
        .into_websocket()
        .await;
    let hello: StreamFrame = ws_seller.receive_json().await;
    assert!(hello.events.is_empty());
    let _: StreamFrame = ws_buyer.receive_json().await;

    seller
        .ok(
            f.id,
            "/orders",
            json!({ "instrument": instrument, "side": "ask", "qty": 10, "limit_price": 100 }),
        )
        .await;
    let c = buyer
        .ok(
            f.id,
            "/orders",
            json!({ "instrument": instrument, "side": "bid", "qty": 10, "limit_price": 100 }),
        )
        .await;
    assert!(kinds(&c).contains(&"Trade"), "{:?}", kinds(&c));
    // Both parties see the Trade without a tick passing.
    let mut saw = (false, false);
    for _ in 0..6 {
        let fr: StreamFrame = ws_seller.receive_json().await;
        if fr.events.iter().any(|e| e.kind == "Trade") {
            saw.0 = true;
            break;
        }
    }
    for _ in 0..6 {
        let fr: StreamFrame = ws_buyer.receive_json().await;
        if fr.events.iter().any(|e| e.kind == "Trade") {
            saw.1 = true;
            break;
        }
    }
    assert_eq!(saw, (true, true));
    let book: BookView = buyer.get(f.id, &format!("/books/{instrument}")).await;
    assert_eq!(book.last_price, Some(100));
    // A tick frame carries only the viewer own delta.
    f.tick(1).await;
    let mut own = false;
    for _ in 0..4 {
        let fr: StreamFrame = ws_buyer.receive_json().await;
        if let Some(t) = fr.events.iter().find(|e| e.kind == "TickResolved") {
            let deltas = t.payload["TickResolved"]["citizen_deltas"]
                .as_array()
                .unwrap();
            assert_eq!(deltas.len(), 1);
            assert_eq!(deltas[0]["citizen"], json!(buyer.citizen));
            own = true;
            break;
        }
    }
    assert!(own);
}

/// D6: a Builder turns Materials into dwellings the org owns; the org view
/// lists them, the recipe says its units are dwellings, the ledger carries
/// the build, and money resting in the org's bids shows as escrow (D5).
#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn a_builders_dwellings_show_on_the_org_and_its_ledger(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let manager = f.client("mason").await;
    let worker = f.client("hod").await;
    f.tick(24).await;
    f.grant(Good::Materials, 40, manager.citizen).await;
    let c = manager
        .ok(
            f.id,
            "/orgs",
            json!({ "kind": "firm", "name": "Brick and Beam", "first_workplace": { "kind": "builder" } }),
        )
        .await;
    let oid = c.events[0].payload["OrgFounded"]["org"].as_u64().unwrap();
    manager
        .ok(
            f.id,
            "/transfers",
            json!({ "to": { "org": oid }, "asset": { "good": ["materials", 20] }, "memo": "stock" }),
        )
        .await;
    let all: OrgsView = manager.get(f.id, "/orgs").await;
    let builder = all
        .recipes
        .iter()
        .find(|r| r.workplace_kind == "builder")
        .expect("a builder recipe");
    assert!(builder.produces_asset, "a builder's units are dwellings");
    assert!(
        !all.recipes
            .iter()
            .any(|r| r.workplace_kind == "farm" && r.produces_asset)
    );
    let org: OrgView = manager.get(f.id, &format!("/orgs/{oid}")).await;
    assert!(org.dwellings.is_empty());
    let wp = u64::from(org.workplaces[0].id);
    let c = manager
        .ok(
            f.id,
            &format!("/orgs/{oid}/offers"),
            json!({ "workplace": wp, "pay": { "hourly": 100 }, "max_hours": 8, "term_cycles": null, "notice_cycles": 1, "places": 1 }),
        )
        .await;
    let offer = c.events[0].payload["EmploymentOffered"]["offer"]
        .as_u64()
        .unwrap();
    worker
        .ok(f.id, &format!("/offers/{offer}/accept"), json!({}))
        .await;
    let r = worker
        .put(
            f.id,
            "/labor",
            json!({ "allocations": [{ "workplace": wp, "hours": 8, "effort": "normal" }] }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    // Money in resting bids is escrow, not treasury.
    manager
        .ok(
            f.id,
            "/transfers",
            json!({ "to": { "org": oid }, "asset": { "money": 5000 }, "memo": "float" }),
        )
        .await;
    manager
        .ok(
            f.id,
            "/orders",
            json!({ "instrument": "ore", "side": "bid", "qty": 1, "limit_price": 1, "on_behalf_of": oid }),
        )
        .await;
    let org: OrgView = manager.get(f.id, &format!("/orgs/{oid}")).await;
    assert_eq!(org.escrow, 1, "one cent rests in the bid");
    assert_eq!(org.treasury, 4999);
    let mut built = false;
    for _ in 0..4 {
        f.tick(24).await;
        let org: OrgView = manager.get(f.id, &format!("/orgs/{oid}")).await;
        if !org.dwellings.is_empty() {
            built = true;
            let d = &org.dwellings[0];
            assert_eq!(d.occupant, None);
            assert_eq!(d.offer, None);
            assert_eq!(d.owner, json!({ "org": oid }));
            break;
        }
    }
    assert!(
        built,
        "eight hours a day of building makes a dwelling within four days"
    );
    let ledger: OrgLedgerView = manager.get(f.id, &format!("/orgs/{oid}/ledger")).await;
    assert!(
        ledger.entries.iter().any(|e| e.kind == "DwellingBuilt"),
        "the ledger carries the build: {:?}",
        ledger
            .entries
            .iter()
            .map(|e| e.kind.as_str())
            .collect::<Vec<_>>()
    );
    // A dwelling the org owns can be let on its behalf, and the view says so.
    let d = manager
        .get::<OrgView>(f.id, &format!("/orgs/{oid}"))
        .await
        .dwellings[0]
        .id;
    manager
        .ok(
            f.id,
            "/offers/lease",
            json!({ "asset": { "dwelling": d }, "rent_per_cycle": 800, "term_cycles": null, "on_behalf_of": oid }),
        )
        .await;
    let org: OrgView = manager.get(f.id, &format!("/orgs/{oid}")).await;
    assert!(
        org.dwellings[0].offer.is_some(),
        "the lease offer shows on the dwelling"
    );
}
