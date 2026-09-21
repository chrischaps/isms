//! S1.4 done gate: one integration test per in-society endpoint; the
//! visibility rules on a medium-monitoring fixture (the manager sees the
//! attributed figure, the worker sees `true_output`; equal where sigma = 0);
//! the stream delivers a `Trade` to both parties within one tick.

use axum_test::TestServer;
use isms_api_types::chronicle::MessagesView;
use isms_api_types::society::{
    ArchiveView, ArchivesView, BookView, BooksView, CitizensView, Committed, ContractsView,
    ContributionView, DigestView, ExplainView, HomeView, HouseholdersView, NoticeBoardView,
    OfficesView, OrgLedgerView, OrgView, OrgsView, PayslipsView, PlanView, PricesView,
    ProposalView, ProposalsView, ScoreboardView, StatsView, StoreView, StreamFrame,
};
use isms_api_types::{Joined, Problem, RejectCode};
use isms_core::WORKSPACE_PRESETS_DIR;
use isms_core::command::{Command, Envelope};
use isms_core::event::{Actor, Event};
use isms_core::ids::CitizenId;
use isms_core::kinds::{ClientKind, Good};
use isms_core::ledger::{Asset, Party};
use isms_server::actor::{SocietyHandle, envelope, spawn};
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

/// Q109 (D9): a manager withdraws a job offer through the one withdraw route,
/// with no `on_behalf_of` needed; the worker cannot.
#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn a_manager_withdraws_a_job_offer_and_a_worker_cannot(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let (manager, worker, wp) = firm_with_worker(&f).await;
    let orgs: OrgsView = manager.get(f.id, "/orgs").await;
    let org = orgs
        .orgs
        .iter()
        .find(|o| o.workplaces.iter().any(|w| u64::from(w.id) == wp))
        .expect("the firm");
    let c = manager
        .ok(
            f.id,
            &format!("/orgs/{}/offers", org.id),
            json!({ "workplace": wp, "pay": { "hourly": 700 }, "max_hours": 8, "term_cycles": null, "notice_cycles": 1, "places": 1 }),
        )
        .await;
    let offer = c.events[0].payload["EmploymentOffered"]["offer"]
        .as_u64()
        .unwrap();
    // The org posted it, so the server acts for the org; the engine answers
    // that the worker does not manage it.
    let r = worker.delete(f.id, &format!("/offers/{offer}")).await;
    assert_eq!(r.status_code(), 422, "{}", r.text());
    assert_eq!(r.json::<Problem>().code, Some(RejectCode::NotManager));
    let r = manager.delete(f.id, &format!("/offers/{offer}")).await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    assert_eq!(kinds(&r.json::<Committed>()), vec!["OfferWithdrawn"]);
    let r = manager.delete(f.id, &format!("/offers/{offer}")).await;
    assert_eq!(r.status_code(), 422);
    assert_eq!(r.json::<Problem>().code, Some(RejectCode::UnknownOffer));
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn a_closing_statement_is_kept_until_the_window_closes(pool: PgPool) {
    // S1.15: nothing to sign before the epoch ends; then one statement per
    // citizen, rewritable, in the order they first spoke, readable by anyone;
    // refused with a named code once the window has closed.
    let f = fixture(pool, "freeport").await;
    let a = f.client("ada").await;
    let b = f.client("bo").await;
    let url = format!("/s/{}/closing-statement", f.id);
    let say = async |c: &Client, text: &str| {
        c.server
            .put(&url)
            .add_header("x-requested-with", "isms")
            .json(&json!({ "text": text }))
            .await
    };
    let r = say(&a, "Too soon.").await;
    assert_eq!(r.status_code(), 404, "{}", r.text());

    // The operator ends the epoch by hand: the archive appears, open, with the summary.
    f.handle
        .command(envelope(
            Actor::System,
            ClientKind::Sim,
            Command::EndEpoch {
                reason: "test".into(),
            },
        ))
        .await
        .unwrap()
        .unwrap();
    let v: ArchivesView = a.server.get(&format!("/s/{}/archives", f.id)).await.json();
    assert_eq!(v.archives.len(), 1);
    let ar = &v.archives[0];
    assert_eq!(
        (ar.epoch, ar.reason.as_str(), ar.final_cycle, ar.open),
        (1, "operator", 1, true)
    );
    assert!(
        ar.summary["standings"]
            .as_array()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(ar.summary["aggregates"]["population"].is_number());
    assert!(ar.closing_statements.is_empty());
    assert_eq!(ar.mine, None);

    let r = say(&a, "  We did what we could.  ").await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let r = say(&b, "Bo was here.").await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let r = say(&a, "We did more than we could.").await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let ar: ArchiveView = r.json();
    assert_eq!(ar.mine.as_deref(), Some("We did more than we could."));
    let texts: Vec<&str> = ar
        .closing_statements
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(texts, ["We did more than we could.", "Bo was here."]);
    assert_eq!(ar.closing_statements[0].citizen, a.citizen);
    assert_eq!(ar.closing_statements[0].handle, "ada");
    assert_eq!(say(&a, "   ").await.status_code(), 400);
    assert_eq!(say(&a, &"x".repeat(2001)).await.status_code(), 400);

    // Anyone may read it, without a `mine`; other epochs are 404.
    let anon = TestServer::builder()
        .http_transport()
        .build(f.router.clone())
        .unwrap();
    let p: ArchiveView = anon
        .get(&format!("/public/s/{}/archives/1", f.id))
        .await
        .json();
    assert_eq!(p.closing_statements.len(), 2);
    assert_eq!(p.mine, None);
    for wrong in [0, 2] {
        assert_eq!(
            anon.get(&format!("/public/s/{}/archives/{wrong}", f.id))
                .await
                .status_code(),
            404
        );
    }

    // The window closes (the rollover does this): a late word is refused, the rest stays.
    f.store
        .close_statements(f.id, 0, chrono::Utc::now())
        .await
        .unwrap();
    let r = say(&a, "Late.").await;
    assert_eq!(r.status_code(), 422, "{}", r.text());
    let p: Problem = r.json();
    assert_eq!(p.code, Some(RejectCode::EpochEnded));
    let ar: ArchiveView = a
        .server
        .get(&format!("/s/{}/archives/1", f.id))
        .await
        .json();
    assert!(!ar.open);
    assert_eq!(ar.closing_statements.len(), 2);
    assert_eq!(ar.mine.as_deref(), Some("We did more than we could."));
}

// -- the assembly (S2.5) -------------------------------------------------------

/// The engine's proposal id, from the `Proposed` payload.
fn proposal_id(c: &Committed) -> u32 {
    let e = c
        .events
        .iter()
        .find(|e| e.kind == "Proposed")
        .expect("a Proposed event");
    u32::try_from(e.payload["Proposed"]["proposal"].as_u64().unwrap()).unwrap()
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn the_assembly_proposes_votes_and_closes_through_the_actor(pool: PgPool) {
    let f = fixture(pool, "commune").await;
    let a = f.client("mover").await;
    let b = f.client("second").await;
    let c = f.client("nay").await;
    let d = f.client("absent").await;
    let caps: isms_api_types::CapabilitiesView = {
        let r = a
            .server
            .get(&format!("/societies/{}/capabilities", f.id))
            .await;
        assert_eq!(r.status_code(), 200, "{}", r.text());
        r.json()
    };
    assert!(
        caps.proposal_kinds
            .contains(&isms_core::constitution::ProposalKindTag::PolicyChange)
    );
    assert_eq!(caps.proposers, isms_core::constitution::Proposers::Anyone);

    // d never votes by hand; the standing plan abstains for them at the close.
    let p0: PlanView = d.get(f.id, "/plan").await;
    let mut plan = p0.plan.clone();
    plan["vote_default"] = json!("abstain");
    let r = d.put(f.id, "/plan", json!({ "plan": plan })).await;
    assert_eq!(r.status_code(), 200, "{}", r.text());

    let opened = a
        .ok(
            f.id,
            "/proposals",
            json!({ "title": "Five hours is enough", "text": "The norm is a day's work, not a day.",
                    "kind": { "policy_change": { "patch": { "work_norm_hours": 5 } } } }),
        )
        .await;
    assert_eq!(kinds(&opened), vec!["Proposed"]);
    let pid = proposal_id(&opened);
    let v: ProposalsView = a.get(f.id, "/proposals").await;
    assert_eq!(v.open.len(), 1);
    assert!(v.closed.is_empty());
    assert_eq!(v.electorate, 4);
    let open = &v.open[0];
    assert_eq!(open.id, pid);
    assert_eq!(open.kind_tag, "policy_change");
    assert_eq!(open.by_handle, "mover");
    assert_eq!(open.tally.eligible, 4);
    assert_eq!(open.tally.quorum, 1);
    assert!(open.my_ballot.is_none());
    assert!(open.floor_open);
    assert!(open.org.is_none());

    // Ballots are replaceable until the close.
    let ballot = format!("/proposals/{pid}/ballot");
    for (who, how) in [(&a, "yes"), (&b, "no"), (&c, "no"), (&b, "yes")] {
        let r = who.put(f.id, &ballot, json!({ "ballot": how })).await;
        assert_eq!(r.status_code(), 200, "{}", r.text());
        let c: Committed = r.json();
        assert_eq!(kinds(&c), vec!["Voted"]);
    }
    let one: ProposalView = a.get(f.id, &format!("/proposals/{pid}")).await;
    assert_eq!((one.tally.yes, one.tally.no, one.tally.cast), (2, 1, 3));
    assert_eq!(one.my_ballot.as_deref(), Some("yes"));
    assert_eq!(one.ballots.len(), 3);
    assert!(
        one.ballots
            .iter()
            .any(|b| b.handle == "second" && b.ballot == "yes")
    );

    // The floor: every citizen reads, anyone posts while the vote is open.
    let floor = format!("/channels/assembly:{pid}/messages");
    let r = a
        .post(f.id, &floor, json!({ "body": "Hear me out." }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    let m: MessagesView = d.get(f.id, &floor).await;
    assert_eq!(m.messages.len(), 1);
    assert_eq!(m.messages[0].handle, "mover");
    assert_eq!(
        d.server
            .get(&p(f.id, "/channels/assembly:999/messages"))
            .await
            .status_code(),
        404
    );

    // The cycle ends: the close casts d's default, the quorum holds, the policy moves.
    f.tick(24).await;
    let v: ProposalsView = a.get(f.id, "/proposals").await;
    assert!(v.open.is_empty());
    assert_eq!(v.closed.len(), 1);
    let closed = &v.closed[0];
    assert!(!closed.open);
    let outcome = closed.outcome.as_ref().expect("an outcome");
    assert!(outcome.passed);
    assert_eq!(
        (outcome.tally.yes, outcome.tally.no, outcome.tally.abstain),
        (2, 1, 1)
    );
    assert_eq!(outcome.tally.cast, 4);
    let effects: Vec<&str> = outcome.effects.iter().map(|e| e.kind.as_str()).collect();
    assert!(effects.contains(&"PolicyChanged"), "{effects:?}");
    assert_eq!(f.handle.world.read().await.policy.work_norm_hours, Some(5));
    let one: ProposalView = c.get(f.id, &format!("/proposals/{pid}")).await;
    assert!(one.outcome.is_some());
    assert!(
        one.floor_open,
        "the floor stays open the day after the close"
    );

    // The away digest tells d what the assembly did for them.
    let dg: DigestView = d.get(f.id, "/away-digest?since=0").await;
    let by_default = dg
        .events
        .iter()
        .find(|e| e.kind == "Voted")
        .expect("the default ballot");
    assert_eq!(by_default.payload["Voted"]["by_default"], true);
    assert_eq!(by_default.payload["Voted"]["ballot"], "abstain");
    assert!(dg.events.iter().any(|e| e.kind == "ProposalClosed"));

    // A vote on the closed proposal is the engine's refusal, named.
    let r = a.put(f.id, &ballot, json!({ "ballot": "yes" })).await;
    assert_eq!(reject(&r), RejectCode::UnknownProposal);
    let problem: Problem = r.json();
    let detail = problem.detail.unwrap_or_default();
    assert!(detail.contains("no open proposal"), "{detail}");

    // One day after the close the floor still takes posts; two days after, it does not.
    let r = b
        .post(f.id, &floor, json!({ "body": "Well fought." }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    f.tick(24).await;
    let r = b.post(f.id, &floor, json!({ "body": "Too late." })).await;
    assert_eq!(reject(&r), RejectCode::UnknownProposal);
    let m: MessagesView = c.get(f.id, &floor).await;
    assert_eq!(m.messages.len(), 2, "the minutes stay readable");
    assert_eq!(
        a.server.get(&p(f.id, "/proposals/999")).await.status_code(),
        404
    );
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn offices_stand_approve_and_seat_and_a_householder_is_no_candidate(pool: PgPool) {
    let f = fixture(pool, "commune").await;
    let a = f.client("alder").await;
    let b = f.client("birch").await;
    let c = f.client("cedar").await;
    let o: OfficesView = a.get(f.id, "/offices").await;
    assert_eq!(o.offices.len(), 1);
    let office = &o.offices[0];
    assert_eq!(
        (office.kind.as_str(), office.seats, office.term_cycles),
        ("coordinator", 3, 5)
    );
    assert!(office.holders.is_empty());
    let election = office
        .election
        .as_ref()
        .expect("the epoch opens with an election");
    assert_eq!(election.seats, 3);
    assert!(election.candidates.is_empty());
    assert!(
        election.stand_refusal.is_none(),
        "{:?}",
        election.stand_refusal
    );

    let candidacy = "/offices/coordinator/candidacy";
    let r = a.ok(f.id, candidacy, json!({})).await;
    assert_eq!(kinds(&r), vec!["CandidacyDeclared"]);
    let r = b.ok(f.id, candidacy, json!({})).await;
    assert_eq!(kinds(&r), vec!["CandidacyDeclared"]);
    let r = b.delete(f.id, candidacy).await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let c2: Committed = r.json();
    assert_eq!(kinds(&c2), vec!["CandidacyWithdrawn"]);
    b.ok(f.id, candidacy, json!({})).await;
    let o: OfficesView = b.get(f.id, "/offices").await;
    let election = o.offices[0].election.as_ref().unwrap();
    assert!(election.i_stand);
    assert_eq!(election.candidates.len(), 2);
    assert_eq!(
        a.server
            .post(&p(f.id, "/offices/mayor/candidacy"))
            .add_header("x-requested-with", "isms")
            .await
            .status_code(),
        400
    );

    // Approval ballots name any subset of the candidates, and are replaceable.
    let ballot = "/offices/coordinator/ballot";
    let r = c
        .put(f.id, ballot, json!({ "candidates": [a.citizen] }))
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let r = c
        .put(
            f.id,
            ballot,
            json!({ "candidates": [a.citizen, b.citizen] }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let o: OfficesView = c.get(f.id, "/offices").await;
    let election = o.offices[0].election.as_ref().unwrap();
    assert_eq!(election.ballots_cast, 1);
    assert_eq!(election.my_approvals.len(), 2);
    assert!(election.candidates.iter().all(|x| x.approvals == 1));

    // A householder's id where a candidate or an office-holder is expected is the engine's 422.
    let householder = f
        .handle
        .world
        .read()
        .await
        .citizens
        .values()
        .find(|z| z.kind == isms_core::kinds::CitizenKind::Householder)
        .map(|z| z.id.0)
        .expect("a householder");
    let r = c
        .put(f.id, ballot, json!({ "candidates": [householder] }))
        .await;
    assert_eq!(reject(&r), RejectCode::NotACandidate);
    let r = a
        .post(
            f.id,
            "/proposals",
            json!({ "title": "Out", "kind": { "recall": { "office": "coordinator", "citizen": householder } } }),
        )
        .await;
    assert_eq!(reject(&r), RejectCode::NotAnOfficeHolder);

    // The cycle ends: the two candidates are seated, a third seat stays open.
    f.tick(24).await;
    let o: OfficesView = a.get(f.id, "/offices").await;
    let office = &o.offices[0];
    let seated: Vec<u32> = office.holders.iter().map(|h| h.citizen).collect();
    assert!(
        seated.contains(&a.citizen) && seated.contains(&b.citizen),
        "{seated:?}"
    );
    assert!(office.i_hold);
    let election = office
        .election
        .as_ref()
        .expect("an election for the empty seat");
    assert_eq!(election.seats, 1);
    let o: OfficesView = c.get(f.id, "/offices").await;
    assert!(!o.offices[0].i_hold);
    let dg: DigestView = a.get(f.id, "/away-digest?since=0").await;
    assert!(dg.events.iter().any(|e| e.kind == "OfficeTaken"));

    // A recall of a sitting coordinator is a real motion.
    let r = c
        .ok(
            f.id,
            "/proposals",
            json!({ "title": "Recall birch", "kind": { "recall": { "office": "coordinator", "citizen": b.citizen } } }),
        )
        .await;
    assert_eq!(kinds(&r), vec!["Proposed"]);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn freeport_has_no_assembly(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("trader").await;
    let r = a
        .server
        .get(&p(f.id, "/channels/assembly:1/messages"))
        .await;
    assert_eq!(reject(&r), RejectCode::NotInThisSociety);
    let r = a
        .post(
            f.id,
            "/proposals",
            json!({ "title": "Anything", "text": "Please.", "kind": "resolution" }),
        )
        .await;
    assert_eq!(reject(&r), RejectCode::NotInThisSociety);
    let o: OfficesView = a.get(f.id, "/offices").await;
    assert!(o.offices.is_empty());
    let v: ProposalsView = a.get(f.id, "/proposals").await;
    assert!(v.open.is_empty() && v.closed.is_empty());
    let sb: ScoreboardView = a.get(f.id, "/scoreboard").await;
    assert!(sb.rows.iter().all(|r| r.honors == 0));
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn a_members_disbursement_is_moved_and_voted_over_the_wire(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("founder").await;
    let b = f.client("member").await;
    let x = f.client("outsider").await;
    let c = a
        .ok(
            f.id,
            "/orgs",
            json!({ "kind": "association", "name": "Mutual Aid" }),
        )
        .await;
    let oid = c.events[0].payload["OrgFounded"]["org"].as_u64().unwrap();
    b.ok(f.id, &format!("/orgs/{oid}/join"), json!({})).await;
    a.ok(
        f.id,
        &format!("/orgs/{oid}/members"),
        json!({ "citizen": b.citizen }),
    )
    .await;
    a.ok(
        f.id,
        "/transfers",
        json!({ "to": { "org": oid }, "asset": { "money": 10_000 }, "memo": "dues" }),
    )
    .await;
    // The manager's direct hand in the treasury is withdrawn (S2.4).
    let r = a
        .post(
            f.id,
            "/transfers",
            json!({ "to": { "citizen": b.citizen }, "asset": { "money": 100 }, "memo": "x", "on_behalf_of": oid }),
        )
        .await;
    assert_eq!(reject(&r), RejectCode::NotAuthorized);

    let moved = a
        .ok(
            f.id,
            &format!("/orgs/{oid}/disbursements"),
            json!({ "to": { "citizen": b.citizen }, "asset": { "money": 5_000 }, "text": "for the roof" }),
        )
        .await;
    let pid = proposal_id(&moved);
    // The org's business is its members'.
    let v: ProposalsView = x.get(f.id, "/proposals").await;
    assert!(v.open.is_empty());
    assert_eq!(
        x.server
            .get(&p(f.id, &format!("/proposals/{pid}")))
            .await
            .status_code(),
        404
    );
    let v: ProposalsView = b.get(f.id, "/proposals").await;
    assert_eq!(v.open.len(), 1);
    assert_eq!(v.open[0].kind_tag, "disbursement");
    assert_eq!(v.open[0].org, Some(u32::try_from(oid).unwrap()));
    assert_eq!(v.open[0].tally.eligible, 2);

    let before: HomeView = b.get(f.id, "/home").await;
    let r = b
        .put(
            f.id,
            &format!("/proposals/{pid}/ballot"),
            json!({ "ballot": "yes" }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let mut c: Committed = r.json();
    if !kinds(&c).contains(&"ProposalClosed") {
        // The mover casts no ballot by moving: a second yes makes the majority.
        let r = a
            .put(
                f.id,
                &format!("/proposals/{pid}/ballot"),
                json!({ "ballot": "yes" }),
            )
            .await;
        assert_eq!(r.status_code(), 200, "{}", r.text());
        c = r.json();
    }
    assert!(kinds(&c).contains(&"Disbursed"), "{:?}", kinds(&c));
    let after: HomeView = b.get(f.id, "/home").await;
    assert_eq!(after.household.balance - before.household.balance, 5_000);
    let v: ProposalsView = b.get(f.id, "/proposals").await;
    assert!(v.open.is_empty());
    let closed = v
        .closed
        .iter()
        .find(|q| q.id == pid)
        .expect("closed for the members");
    let outcome = closed.outcome.as_ref().unwrap();
    assert!(outcome.passed);
    assert!(outcome.effects.iter().any(|e| e.kind == "Disbursed"));
    let v: ProposalsView = x.get(f.id, "/proposals").await;
    assert!(v.closed.is_empty(), "an outsider sees no members' vote");
}

// -- the Common Store and the Ledger of Contribution (S2.7) ------------------------

/// S2.7 done gate: the Store's entitlement is the engine's; a norm position
/// taken over the wire puts hours on the Ledger; the Ledger shows output as
/// attributed and never the true figure while sigma > 0; the day's close
/// writes the record and the Store reports yesterday.
#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn the_store_and_the_ledger_of_the_commune(pool: PgPool) {
    let f = fixture(pool, "commune").await;
    let a = f.client("hand").await;

    // The shelves and the rule, with my entitlement as `store::entitlement` computes it.
    let s: StoreView = a.get(f.id, "/store").await;
    assert_eq!(s.rule, "need_first");
    assert_eq!(s.stock[0].good, Good::Food);
    assert_eq!(s.stock[1].good, Good::Wares);
    {
        let w = f.handle.world.read().await;
        let me = w.citizens.get(&CitizenId(a.citizen)).unwrap();
        for row in &s.stock {
            assert_eq!(
                row.my_entitlement,
                isms_core::store::entitlement(&w, me, row.good),
                "{:?}",
                row.good
            );
            assert_eq!(row.my_pending, 0);
        }
        let active = w.citizens.values().filter(|c| !c.dormant).count();
        assert_eq!(s.active_citizens, u32::try_from(active).unwrap());
        assert_eq!(
            s.stock[0].share_if_shared_now,
            s.stock[0].stock / s.active_citizens
        );
    }
    assert!(s.last_cycle.is_none());
    assert!(s.yesterday.is_empty());
    assert!(s.my_draws.is_empty());

    // A norm position, taken over the wire (Q62, Q141): no contract, on the labor view.
    let wid = {
        let w = f.handle.world.read().await;
        w.workplaces
            .values()
            .find(|wp| isms_core::orgs::check_room(&w, wp.id).is_ok())
            .map(|wp| wp.id.0)
            .expect("a workplace with room")
    };
    let taken = a
        .ok(f.id, &format!("/workplaces/{wid}/position"), json!({}))
        .await;
    assert_eq!(kinds(&taken), vec!["Assigned"]);
    let home: HomeView = a.get(f.id, "/home").await;
    assert_eq!(home.labor.positions.len(), 1);
    assert_eq!(home.labor.positions[0].workplace, wid);
    assert!(home.labor.positions[0].contract.is_none());
    assert!(home.labor.employment.is_empty());
    assert_eq!(
        reject(
            &a.post(f.id, &format!("/workplaces/{wid}/position"), json!({}))
                .await
        ),
        RejectCode::AlreadyExists
    );
    let r = a
        .put(
            f.id,
            "/labor",
            json!({ "allocations": [{ "workplace": wid, "hours": 6, "effort": "normal" }] }),
        )
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());

    // Two hours of work: the Ledger has my hours exactly and my output as attributed.
    f.tick(2).await;
    let l: ContributionView = a.get(f.id, "/ledger").await;
    assert_eq!(l.norm_hours, Some(6));
    assert_eq!(l.monitoring, "low");
    assert!(l.sigma > 0.0, "the Commune's monitoring is low, sigma > 0");
    assert_eq!(l.rows.iter().filter(|r| r.is_me).count(), 1);
    let mine = l.rows.iter().find(|r| r.is_me).unwrap();
    assert_eq!(mine.handle, "hand");
    assert_eq!(mine.workplaces, vec![wid]);
    assert!(
        (mine.hours_today - 12.0 / 24.0).abs() < 1e-9,
        "{}",
        mine.hours_today
    );
    assert!(!mine.norm_met_today);
    assert_eq!(mine.days, 0);
    // The true figures live only in the `Produced` events; the Ledger carries the noised sum.
    let produced = f
        .store
        .read_last_of_kind(f.id, "Produced", 10_000)
        .await
        .unwrap();
    let (mut true_sum, mut attributed_sum) = (0.0_f64, 0.0_f64);
    for e in &produced {
        if let Event::Produced { per_worker, .. } = &e.event {
            for w in per_worker {
                if w.citizen == CitizenId(a.citizen) {
                    true_sum += w.true_output;
                    attributed_sum += w.attributed_output;
                }
            }
        }
    }
    assert!(attributed_sum > 0.0, "the hours produced something");
    assert!((mine.attributed_today - attributed_sum).abs() < 1e-9);
    assert!(
        (mine.attributed_today - true_sum).abs() > 1e-9,
        "with sigma > 0 the Ledger never shows the exact output"
    );
    let raw = a.server.get(&p(f.id, "/ledger")).await.text();
    assert!(!raw.contains("true_output"));
    // Active rows come first, by hours today; every dormant row after.
    let first_dormant = l
        .rows
        .iter()
        .position(|r| r.dormant)
        .unwrap_or(l.rows.len());
    assert!(l.rows[..first_dormant].iter().all(|r| !r.dormant));
    assert!(
        l.rows[..first_dormant]
            .windows(2)
            .all(|w| w[0].hours_today >= w[1].hours_today)
    );
    assert!(l.least_staffed.is_some());

    // The day closes: the record has a day, the norm met; the Store reports yesterday.
    f.tick(22).await;
    let l: ContributionView = a.get(f.id, "/ledger").await;
    let mine = l.rows.iter().find(|r| r.is_me).unwrap();
    assert_eq!(mine.days, 1);
    assert!(
        (mine.hours_yesterday - 6.0).abs() < 1e-9,
        "{}",
        mine.hours_yesterday
    );
    assert_eq!(mine.norm_met_days, 1);
    assert!((mine.hours_total - 6.0).abs() < 1e-9);
    let s: StoreView = a.get(f.id, "/store").await;
    assert_eq!(s.last_cycle, Some(0));
    let food = s
        .yesterday
        .iter()
        .find(|d| d.good == Good::Food)
        .expect("someone drew Food yesterday");
    assert!(food.requested > 0);
    assert!(food.served <= food.requested);
    assert_eq!(food.short, food.requested - food.served);
    // My own draws, oldest first, each with its Explain.
    assert!(
        s.my_draws.windows(2).all(|w| w[0].seq < w[1].seq),
        "the draw record is oldest first"
    );
    for d in &s.my_draws {
        assert_eq!(d.kind, "Drew");
        assert!(d.payload["Drew"]["explain"]["rule"].is_string());
    }

    // The Commune's scoreboard ranks the contribution record, not net worth.
    let sb: ScoreboardView = a.get(f.id, "/scoreboard").await;
    assert!(sb.rows.iter().all(|r| r.contribution.is_some()));
    assert!(sb.rows.windows(2).all(|w| {
        w[0].contribution.as_ref().unwrap().hours_total
            >= w[1].contribution.as_ref().unwrap().hours_total
    }));
    let me = sb.rows.iter().find(|r| r.citizen == a.citizen).unwrap();
    assert_eq!(me.contribution.as_ref().unwrap().norm_met_days, 1);

    // Giving the position up.
    let left = a.delete(f.id, &format!("/workplaces/{wid}/position")).await;
    assert_eq!(left.status_code(), 200, "{}", left.text());
    let left: Committed = left.json();
    assert_eq!(kinds(&left), vec!["Unassigned"]);
    let home: HomeView = a.get(f.id, "/home").await;
    assert!(home.labor.positions.is_empty());
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn freeport_has_no_store_and_no_ledger(pool: PgPool) {
    let f = fixture(pool, "freeport").await;
    let a = f.client("trader").await;
    assert_eq!(
        reject(&a.server.get(&p(f.id, "/store")).await),
        RejectCode::NoStore
    );
    assert_eq!(
        reject(&a.server.get(&p(f.id, "/ledger")).await),
        RejectCode::NotInThisSociety
    );
    let wid = f
        .handle
        .world
        .read()
        .await
        .workplaces
        .keys()
        .next()
        .unwrap()
        .0;
    assert_eq!(
        reject(
            &a.post(f.id, &format!("/workplaces/{wid}/position"), json!({}))
                .await
        ),
        RejectCode::NotInThisSociety
    );
    let sb: ScoreboardView = a.get(f.id, "/scoreboard").await;
    assert!(sb.rows.iter().all(|r| r.contribution.is_none()));
    let me: isms_api_types::Me = {
        let r = a.server.get("/me").await;
        assert_eq!(r.status_code(), 200, "{}", r.text());
        r.json()
    };
    assert_eq!(me.citizenships[0].honors, 0);
}
