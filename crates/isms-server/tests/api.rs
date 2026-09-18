//! S1.3 done gate: an integration test per endpoint; joining twice returns the
//! existing citizen; an API-key command records `client_kind = api_key` in the
//! event; the `OpenAPI` document is produced (linted in CI).

use axum_test::TestServer;
use isms_api_types::{
    ApiKeyCreated, CapabilitiesView, Health, Joined, Lexicon, Me, Problem, SocietyList,
    SocietySummary, Welcome,
};
use isms_core::WORKSPACE_PRESETS_DIR;
use isms_core::kinds::ClientKind;
use isms_server::actor::spawn;
use isms_server::mail::MemorySender;
use isms_server::runtime::{SeedSpec, seed_society};
use isms_server::scheduler::{Control, Schedule};
use isms_server::state::{AppState, SocietyEntry};
use isms_store::{EventStore, PgEventStore, load_world};
use serde_json::json;
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BASE: &str = "http://test";

struct Fixture {
    server: TestServer,
    state: AppState,
    handle: isms_server::actor::SocietyHandle,
    mail: Arc<MemorySender>,
    store: PgEventStore,
    society: i64,
}

async fn fixture(pool: PgPool) -> Fixture {
    let store = PgEventStore::from_pool(pool);
    let row = seed_society(
        &store,
        Path::new(WORKSPACE_PRESETS_DIR),
        &SeedSpec {
            name: "freeport-1".into(),
            preset: "freeport".into(),
            class: "canonical".into(),
            seed: 1,
            tick_seconds: 3600,
            cycle_boundary_hour: 4,
            overrides: vec![],
        },
    )
    .await
    .unwrap();
    let loaded = load_world(&store, row.id).await.unwrap();
    let (handle, _task) = spawn(row.id, store.clone(), loaded);
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
    let server = TestServer::builder()
        .save_cookies()
        .build(isms_server::api::router(state.clone()))
        .unwrap();
    Fixture {
        server,
        state,
        handle,
        mail,
        store,
        society: row.id,
    }
}

/// Invite, request a link, follow it: the server now holds a session cookie.
async fn sign_in(f: &Fixture, email: &str) {
    f.store.create_invite_code("code-1", None).await.ok();
    let code = f.store.unused_invite_codes().await.unwrap().pop().unwrap();
    let r = f
        .server
        .post("/auth/magic-link")
        .json(&json!({ "email": email, "invite_code": code }))
        .await;
    assert_eq!(r.status_code(), 202, "{}", r.text());
    let url = f.mail.last_for(email).expect("a link was sent");
    let path = url.strip_prefix(BASE).unwrap();
    let r = f.server.get(path).await;
    assert_eq!(r.status_code(), 303, "{}", r.text());
    assert!(
        r.header("set-cookie")
            .to_str()
            .unwrap()
            .contains("isms_session=")
    );
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn health_and_openapi_need_no_login(pool: PgPool) {
    let f = fixture(pool).await;
    let h: Health = f.server.get("/healthz").await.json();
    assert!(h.ok);
    assert_eq!(h.societies, 1);
    let spec = f.server.get("/openapi.json").await;
    assert_eq!(spec.status_code(), 200);
    let doc: serde_json::Value = spec.json();
    assert!(doc["paths"]["/societies/{id}/join"].is_object());
    assert!(doc["components"]["securitySchemes"]["api_key"].is_object());
    let docs = f.server.get("/docs/").await;
    assert_eq!(docs.status_code(), 200);
    // And the offline document matches the served one.
    let offline = serde_json::to_value(isms_server::api::openapi()).unwrap();
    assert_eq!(offline["paths"], doc["paths"]);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn magic_link_needs_an_invite_the_first_time(pool: PgPool) {
    let f = fixture(pool).await;
    let r = f
        .server
        .post("/auth/magic-link")
        .json(&json!({ "email": "new@example.test" }))
        .await;
    assert_eq!(r.status_code(), 403);
    let p: Problem = r.json();
    assert_eq!(p.status, 403);
    assert_eq!(
        r.header("content-type").to_str().unwrap(),
        "application/problem+json"
    );
    let r = f
        .server
        .post("/auth/magic-link")
        .json(&json!({ "email": "new@example.test", "invite_code": "nope" }))
        .await;
    assert_eq!(r.status_code(), 403);
    assert!(f.mail.last_for("new@example.test").is_none());
    let r = f
        .server
        .post("/auth/magic-link")
        .json(&json!({ "email": "not an email" }))
        .await;
    assert_eq!(r.status_code(), 400);
    // Unauthenticated /me.
    assert_eq!(f.server.get("/me").await.status_code(), 401);
    // A used link is dead.
    sign_in(&f, "new@example.test").await;
    let url = f.mail.last_for("new@example.test").unwrap();
    let r = f.server.get(url.strip_prefix(BASE).unwrap()).await;
    assert_eq!(r.status_code(), 401);
    // A second sign-in needs no invite (the code is spent, the account exists).
    let r = f
        .server
        .post("/auth/magic-link")
        .json(&json!({ "email": "New@Example.test " }))
        .await;
    assert_eq!(r.status_code(), 202);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn auth_endpoints_are_rate_limited_per_ip(pool: PgPool) {
    let f = fixture(pool).await;
    let mut last = 0;
    for _ in 0..12 {
        last = f
            .server
            .post("/auth/magic-link")
            .json(&json!({ "email": "x@example.test" }))
            .await
            .status_code()
            .as_u16();
    }
    assert_eq!(last, 429);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn me_logout_and_csrf(pool: PgPool) {
    let f = fixture(pool).await;
    sign_in(&f, "a@example.test").await;
    let me: Me = f.server.get("/me").await.json();
    assert_eq!(me.account.email, "a@example.test");
    assert!(me.citizenships.is_empty());
    // State-changing web requests need the CSRF header.
    let r = f.server.post("/auth/logout").await;
    assert_eq!(r.status_code(), 403);
    let r = f
        .server
        .post("/auth/logout")
        .add_header("x-requested-with", "isms")
        .await;
    assert_eq!(r.status_code(), 204);
    assert_eq!(f.server.get("/me").await.status_code(), 401);
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn societies_capabilities_lexicon_welcome(pool: PgPool) {
    let f = fixture(pool).await;
    sign_in(&f, "a@example.test").await;
    let list: SocietyList = f.server.get("/societies").await.json();
    assert_eq!(list.societies.len(), 1);
    let s = &list.societies[0];
    assert_eq!(s.preset, "freeport");
    assert_eq!(s.householders, 40);
    assert_eq!(s.clock.cycle, 1);
    assert_eq!(s.clock.tick, 1);
    assert_eq!(s.clock.engine_tick, 0);
    assert!(s.next_tick_at.is_some());
    let one: SocietySummary = f
        .server
        .get(&format!("/societies/{}", f.society))
        .await
        .json();
    assert_eq!(one.id, f.society);
    assert_eq!(f.server.get("/societies/999").await.status_code(), 404);
    let caps: CapabilitiesView = f
        .server
        .get(&format!("/societies/{}/capabilities", f.society))
        .await
        .json();
    assert!(caps.money && caps.order_books && !caps.common_store);
    assert_eq!(caps.rate_limit.per_second, 5);
    let lex: Lexicon = f
        .server
        .get(&format!("/societies/{}/lexicon", f.society))
        .await
        .json();
    assert_eq!(lex.entries["compensation"], "Payslip");
    let w: Welcome = f
        .server
        .get(&format!("/societies/{}/welcome", f.society))
        .await
        .json();
    assert!(w.markdown.starts_with("**Welcome to Freeport.**"));
    assert!(!w.markdown.contains("{{"));
    assert!(w.markdown.contains("credits"));
}

#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn join_is_idempotent_and_api_keys_stamp_their_kind(pool: PgPool) {
    let f = fixture(pool).await;
    sign_in(&f, "a@example.test").await;
    let path = format!("/societies/{}/join", f.society);
    // Bad handle, then a good one.
    let r = f
        .server
        .post(&path)
        .add_header("x-requested-with", "isms")
        .json(&json!({ "handle": "x" }))
        .await;
    assert_eq!(r.status_code(), 400);
    let r = f
        .server
        .post(&path)
        .add_header("x-requested-with", "isms")
        .json(&json!({ "handle": "marlow" }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    let joined: Joined = r.json();
    assert!(joined.created);
    assert_eq!(joined.handle, "marlow");
    // Joining twice returns the existing citizen.
    let r = f
        .server
        .post(&path)
        .add_header("x-requested-with", "isms")
        .json(&json!({ "handle": "someone-else" }))
        .await;
    assert_eq!(r.status_code(), 200);
    let again: Joined = r.json();
    assert!(!again.created);
    assert_eq!(again.citizen_id, joined.citizen_id);
    assert_eq!(again.handle, "marlow");
    let me: Me = f.server.get("/me").await.json();
    assert_eq!(me.citizenships.len(), 1);
    assert_eq!(me.citizenships[0].citizen_id, joined.citizen_id);
    // The web join was stamped as such in the log.
    let events = f.store.read_from(f.society, -1, 100_000).await.unwrap();
    let join_event = events
        .iter()
        .find(|e| e.event.kind() == "CitizenJoined" && e.meta.client_kind == Some(ClientKind::Web))
        .expect("a web CitizenJoined");
    assert_eq!(join_event.meta.tick, 0);

    // API key: created from the browser session, scoped to the citizen.
    let r = f
        .server
        .post("/me/api-keys")
        .add_header("x-requested-with", "isms")
        .json(&json!({ "society_id": f.society, "label": "my agent" }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
    let key: ApiKeyCreated = r.json();
    assert!(key.key.starts_with("isms_"));
    let me: Me = f.server.get("/me").await.json();
    assert_eq!(me.api_keys.len(), 1);
    assert_eq!(me.api_keys[0].label, "my agent");

    // A fresh client with only the key: reading the society issues a Seen
    // stamped api_key (TDD 10.2), exactly once per tick. The engine already
    // counts the join as presence at tick 0, so move the clock first.
    f.handle.tick().await.unwrap();
    let r = f
        .server
        .get(&format!("/societies/{}", f.society))
        .authorization_bearer(&key.key)
        .await;
    assert_eq!(r.status_code(), 200, "{}", r.text());
    let r = f
        .server
        .get(&format!("/societies/{}/welcome", f.society))
        .authorization_bearer(&key.key)
        .await;
    assert_eq!(r.status_code(), 200);
    let events = f.store.read_from(f.society, -1, 100_000).await.unwrap();
    let seen: Vec<_> = events
        .iter()
        .filter(|e| e.event.kind() == "CitizenSeen")
        .collect();
    assert_eq!(seen.len(), 1, "one Seen per tick per citizen");
    assert_eq!(seen[0].meta.tick, 1);
    assert_eq!(seen[0].meta.client_kind, Some(ClientKind::ApiKey));
    assert!(matches!(
        seen[0].event,
        isms_core::event::Event::CitizenSeen {
            client_kind: ClientKind::ApiKey,
            ..
        }
    ));
    // A bad key is refused; a revoked key stops working.
    let r = f
        .server
        .get("/me")
        .authorization_bearer("isms_nope.nope")
        .await;
    assert_eq!(r.status_code(), 401);
    let r = f
        .server
        .delete(&format!("/me/api-keys/{}", key.id))
        .add_header("x-requested-with", "isms")
        .await;
    assert_eq!(r.status_code(), 204);
    let r = f.server.get("/me").authorization_bearer(&key.key).await;
    assert_eq!(r.status_code(), 401);
    let r = f
        .server
        .delete(&format!("/me/api-keys/{}", key.id))
        .add_header("x-requested-with", "isms")
        .await;
    assert_eq!(r.status_code(), 404);
}

/// ADR-0009: a `lab` society is for synthetic players. Anyone signed in on the
/// server can see and join it; a visitor on `/public/*` cannot tell it exists.
#[sqlx::test(migrator = "isms_store::MIGRATOR")]
async fn a_lab_society_is_never_public(pool: PgPool) {
    let f = fixture(pool).await;
    let lab = seed_society(
        &f.store,
        Path::new(WORKSPACE_PRESETS_DIR),
        &SeedSpec {
            name: "lab-1".into(),
            preset: "freeport".into(),
            class: "lab".into(),
            seed: 2,
            tick_seconds: 3600,
            cycle_boundary_hour: 4,
            overrides: vec![],
        },
    )
    .await
    .unwrap();
    let loaded = load_world(&f.store, lab.id).await.unwrap();
    let (handle, _task) = spawn(lab.id, f.store.clone(), loaded);
    f.state.societies.write().unwrap().insert(
        lab.id,
        SocietyEntry {
            row: lab.clone(),
            handle,
            control: Arc::new(Control::new(
                Schedule {
                    tick_seconds: 3600,
                    tick_origin: lab.tick_origin,
                },
                false,
            )),
        },
    );

    // The spectator routes: absent, and 404 by id.
    let public: SocietyList = f.server.get("/public/societies").await.json();
    assert_eq!(public.societies.len(), 1);
    assert_eq!(public.societies[0].class, "canonical");
    assert_eq!(
        f.server
            .get(&format!("/public/s/{}/stats", lab.id))
            .await
            .status_code(),
        404
    );
    assert_eq!(
        f.server
            .get(&format!("/public/s/{}/chronicle", lab.id))
            .await
            .status_code(),
        404
    );
    // The canonical society still answers.
    assert_eq!(
        f.server
            .get(&format!("/public/s/{}/stats", f.society))
            .await
            .status_code(),
        200
    );

    // Signed in: listed, with its class, and joinable.
    sign_in(&f, "lab-tester@example.test").await;
    let list: SocietyList = f.server.get("/societies").await.json();
    let mine: Vec<&SocietySummary> = list.societies.iter().filter(|s| s.id == lab.id).collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].class, "lab");
    let r = f
        .server
        .post(&format!("/societies/{}/join", lab.id))
        .add_header("x-requested-with", "isms")
        .json(&json!({ "handle": "probe" }))
        .await;
    assert_eq!(r.status_code(), 201, "{}", r.text());
}
