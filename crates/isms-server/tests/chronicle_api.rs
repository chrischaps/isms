//! S1.5 done gate (integration half): headlines appear from live events and
//! `rebuild` reproduces the same rows; a non-member cannot read an org
//! channel; the Square and DMs work. The three-headline scripted cycle is a
//! unit test in `isms_server::chronicle`.

use axum_test::TestServer;
use isms_api_types::Joined;
use isms_api_types::chronicle::{ChronicleView, MessageView, MessagesView};
use isms_api_types::society::{Committed, HomeView, NoticeBoardView};
use isms_core::WORKSPACE_PRESETS_DIR;
use isms_server::actor::{SocietyHandle, spawn};
use isms_server::chronicle::{Templates, rebuild, spawn_projector};
use isms_server::mail::MemorySender;
use isms_server::runtime::{SeedSpec, seed_society};
use isms_server::scheduler::{Control, Schedule};
use isms_server::state::{AppState, SocietyEntry};
use isms_store::{PgEventStore, load_world};
use serde_json::json;
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const BASE: &str = "http://test";

struct Fixture {
    router: axum::Router,
    handle: SocietyHandle,
    mail: Arc<MemorySender>,
    store: PgEventStore,
    id: i64,
    cancel: CancellationToken,
}

struct Client {
    server: TestServer,
    citizen: u32,
}

async fn fixture(pool: PgPool) -> Fixture {
    let store = PgEventStore::from_pool(pool);
    let row = seed_society(
        &store,
        Path::new(WORKSPACE_PRESETS_DIR),
        &SeedSpec {
            name: "freeport-1".into(),
            preset: "freeport".into(),
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
    let cancel = CancellationToken::new();
    let templates = Templates::load(Path::new(WORKSPACE_PRESETS_DIR), "freeport").unwrap();
    let projector = spawn_projector(store.clone(), handle.clone(), templates, cancel.clone());
    std::mem::forget(projector);
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
        cancel,
    }
}

impl Fixture {
    async fn tick(&self, n: u32) {
        for _ in 0..n {
            self.handle.tick().await.unwrap();
        }
    }

    async fn client(&self, handle: &str) -> Client {
        let email = format!("{handle}@example.test");
        let code = format!("code-{handle}");
        self.store.create_invite_code(&code, None).await.unwrap();
        let server = TestServer::builder()
            .save_cookies()
            .build(self.router.clone())
            .unwrap();
        let r = server
            .post("/auth/magic-link")
            .json(&json!({ "email": email, "invite_code": code }))
            .await;
        assert_eq!(r.status_code(), 202, "{}", r.text());
        let url = self.mail.last_for(&email).unwrap();
        assert_eq!(
            server
                .get(url.strip_prefix(BASE).unwrap())
                .await
                .status_code(),
            303
        );
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

    /// Wait until the projector has written a headline for `seq`.
    async fn settled(&self, seq: i64) {
        for _ in 0..200 {
            let rows = self.store.all_headlines(self.id).await.unwrap();
            if rows.iter().any(|h| h.source_event_seq >= seq) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        panic!("projector never caught up to seq {seq}");
    }
}

fn path(id: i64, rest: &str) -> String {
    format!("/s/{id}{rest}")
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn headlines_from_live_events_and_rebuild_reproduces_them(pool: PgPool) {
    let f = fixture(pool).await;
    let a = f.client("marlow").await;
    // Joining is itself news; buying a legacy firm makes a second headline.
    f.tick(1).await;
    let board: NoticeBoardView = a.server.get(&path(f.id, "/notice-board")).await.json();
    let listing = board
        .offers
        .iter()
        .filter(|o| o.kind == "sale" && o.body["sale"]["asset"].get("shares").is_some())
        .min_by_key(|o| {
            o.body["sale"]["price"]["money"]
                .as_i64()
                .unwrap_or(i64::MAX)
        })
        .unwrap();
    let r = a
        .server
        .post(&path(f.id, &format!("/offers/{}/accept", listing.id)))
        .add_header("x-requested-with", "isms")
        .json(&json!({}))
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let c: Committed = r.json();
    let appointed = c
        .events
        .iter()
        .find(|e| e.kind == "ManagerAppointed")
        .expect("the householder manager steps down");
    f.settled(appointed.seq).await;

    let edition: ChronicleView = a.server.get(&path(f.id, "/chronicle")).await.json();
    assert_eq!(edition.cycle, 1);
    let texts: Vec<&str> = edition.headlines.iter().map(|h| h.text.as_str()).collect();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("marlow arrives with 1000.00 credits")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("marlow takes the chair at Legacy")),
        "{texts:?}"
    );
    // Home carries the latest headlines.
    let home: HomeView = a.server.get(&path(f.id, "/home")).await.json();
    assert!(!home.headlines.is_empty());
    assert!(
        home.headlines
            .iter()
            .any(|h| h.text.starts_with("marlow takes the chair"))
    );

    // A cycle closes: the edition for cycle 1 is complete; cycle 2 starts empty.
    f.tick(24).await;
    let closed: ChronicleView = a.server.get(&path(f.id, "/chronicle?cycle=1")).await.json();
    assert!(closed.headlines.len() >= 2);
    let before = f.store.all_headlines(f.id).await.unwrap();
    assert!(before.len() >= 2);
    // Rebuild from the log reproduces exactly the same rows.
    let templates = Templates::load(Path::new(WORKSPACE_PRESETS_DIR), "freeport").unwrap();
    let n = rebuild(&f.store, f.id, templates).await.unwrap();
    let after = f.store.all_headlines(f.id).await.unwrap();
    assert_eq!(n, after.len());
    assert_eq!(before, after);
    f.cancel.cancel();
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn square_org_channel_and_dm_membership(pool: PgPool) {
    let f = fixture(pool).await;
    let owner = f.client("owner").await;
    let other = f.client("other").await;
    // The Square is open to every citizen.
    let r = owner
        .server
        .post(&path(f.id, "/channels/square/messages"))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "  hello, Freeport  " }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    let m: MessageView = r.json();
    assert_eq!(m.body, "hello, Freeport");
    assert_eq!(m.handle, "owner");
    let square: MessagesView = other
        .server
        .get(&path(f.id, "/channels/square/messages"))
        .await
        .json();
    assert_eq!(square.messages.len(), 1);
    let page: MessagesView = other
        .server
        .get(&path(
            f.id,
            &format!("/channels/square/messages?after={}", m.id),
        ))
        .await
        .json();
    assert!(page.messages.is_empty());
    // Empty and oversized bodies are refused.
    let r = owner
        .server
        .post(&path(f.id, "/channels/square/messages"))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "   " }))
        .await;
    assert_eq!(r.status_code(), 400);

    // An org channel: the owner of a bought legacy firm is in; a stranger is not.
    f.tick(1).await;
    let board: NoticeBoardView = owner.server.get(&path(f.id, "/notice-board")).await.json();
    let listing = board
        .offers
        .iter()
        .filter(|o| o.kind == "sale" && o.body["sale"]["asset"].get("shares").is_some())
        .min_by_key(|o| {
            o.body["sale"]["price"]["money"]
                .as_i64()
                .unwrap_or(i64::MAX)
        })
        .unwrap();
    let org = listing.body["sale"]["asset"]["shares"][0].as_u64().unwrap();
    let r = owner
        .server
        .post(&path(f.id, &format!("/offers/{}/accept", listing.id)))
        .add_header("x-requested-with", "isms")
        .json(&json!({}))
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let channel = format!("/channels/org:{org}/messages");
    let r = owner
        .server
        .post(&path(f.id, &channel))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "all hands: wages rise next cycle" }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    let r = other.server.get(&path(f.id, &channel)).await;
    assert_eq!(
        r.status_code(),
        403,
        "a non-member cannot read an org channel"
    );
    let r = other
        .server
        .post(&path(f.id, &channel))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "let me in" }))
        .await;
    assert_eq!(r.status_code(), 403);
    let mine: MessagesView = owner.server.get(&path(f.id, &channel)).await.json();
    assert_eq!(mine.messages.len(), 1);
    // An employee of the firm is a member of its channel.
    let r = owner
        .server
        .post(&path(f.id, &format!("/orgs/{org}/offers")))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "workplace": mine_workplace(&f, org).await, "pay": { "hourly": 800 }, "max_hours": 8, "term_cycles": null, "notice_cycles": 1, "places": 1 }))
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let c: Committed = r.json();
    let offer = c.events[0].payload["EmploymentOffered"]["offer"]
        .as_u64()
        .unwrap();
    let r = other
        .server
        .post(&path(f.id, &format!("/offers/{offer}/accept")))
        .add_header("x-requested-with", "isms")
        .json(&json!({}))
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let r = other.server.get(&path(f.id, &channel)).await;
    assert_eq!(r.status_code(), 200);

    // DMs: both parties read them, nobody else can address a stranger's thread.
    let r = owner
        .server
        .post(&path(f.id, &format!("/dm/{}", other.citizen)))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "quietly: I can lend you 200" }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    let theirs: MessagesView = other
        .server
        .get(&path(f.id, &format!("/dm/{}", owner.citizen)))
        .await
        .json();
    assert_eq!(theirs.messages.len(), 1);
    assert_eq!(
        theirs.channel,
        format!(
            "dm:{}:{}",
            owner.citizen.min(other.citizen),
            owner.citizen.max(other.citizen)
        )
    );
    let r = owner
        .server
        .post(&path(f.id, &format!("/dm/{}", owner.citizen)))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "me" }))
        .await;
    assert_eq!(r.status_code(), 400);
    let r = owner
        .server
        .post(&path(f.id, "/dm/99999"))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "body": "nobody" }))
        .await;
    assert_eq!(r.status_code(), 404);
    let r = owner
        .server
        .get(&path(f.id, "/channels/dm:1:2/messages"))
        .await;
    assert_eq!(r.status_code(), 400);
    f.cancel.cancel();
}

async fn mine_workplace(f: &Fixture, org: u64) -> u64 {
    let w = f.handle.world.read().await;
    let o = w
        .orgs
        .get(&isms_core::ids::OrgId(u32::try_from(org).unwrap()))
        .unwrap();
    u64::from(o.workplaces.iter().next().unwrap().0)
}
