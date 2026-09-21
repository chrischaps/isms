//! The HTTP API (TDD 10): axum handlers, the router, and the `OpenAPI` document.
//! S1.3 covers account, society discovery, and joining; S1.4 adds the
//! in-society command and query surface.

use crate::actor::{SocietyHandle, envelope};
use crate::auth::{
    Auth, Client, ClientIp, MAGIC_LINK_TTL, SESSION_COOKIE, SESSION_TTL, expiry, hash_token,
    mint_api_key, random_token,
};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, SocietyEntry, clock_of};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use isms_api_types::society::{ArchiveView, ArchivesView};
use isms_api_types::{
    Account, AdminSocietiesView, AdminSocietyView, ApiKeyCreated, ApiKeySummary, CapabilitiesView,
    Citizenship, Clock, Health, JoinRequest, Joined, Lexicon, MagicLinkRequest, MagicLinkSent, Me,
    NewApiKey, Problem, PublicStatsView, SocietyList, SocietySummary, TickSecondsRequest, UpdateMe,
    Welcome,
};
use isms_core::command::Command;
use isms_core::event::{Actor, Event};
use isms_core::ids::CitizenId;
use isms_core::kinds::{CitizenKind, ClientKind};
use isms_core::rules::Rules;
use isms_store::EventStore;
use isms_store::accounts::CitizenRow;
use serde::Deserialize;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

/// Per-IP budget for the auth endpoints (TDD 14): one request per 6 s, burst 10.
const AUTH_IP_PER_SECOND: f64 = 1.0 / 6.0;
const AUTH_IP_BURST: f64 = 10.0;

// -- helpers -------------------------------------------------------------------

pub(crate) fn society(state: &AppState, id: i64) -> ApiResult<SocietyEntry> {
    state
        .society(id)
        .ok_or_else(|| ApiError::NotFound(format!("no society {id}")))
}

/// A `lab` society (ADR-0009) is for synthetic players: it is never listed or
/// served on `/public/*`, so to a visitor it does not exist.
pub(crate) fn is_public(entry: &SocietyEntry) -> bool {
    entry.row.class != "lab"
}

/// The society a spectator route may show, or 404.
pub(crate) fn public_society(state: &AppState, id: i64) -> ApiResult<SocietyEntry> {
    let entry = society(state, id)?;
    if is_public(&entry) {
        Ok(entry)
    } else {
        Err(ApiError::NotFound(format!("no society {id}")))
    }
}

async fn clock(handle: &SocietyHandle) -> Clock {
    clock_of(&*handle.world.read().await)
}

fn valid_handle(handle: &str) -> bool {
    let n = handle.chars().count();
    (2..=24).contains(&n)
        && handle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn normalize_email(email: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    if email.len() < 3 || !email.contains('@') || email.contains(char::is_whitespace) {
        return Err(ApiError::BadRequest("that is not an email address".into()));
    }
    Ok(email)
}

/// The caller's citizen in this society, if any. An API key must belong to it.
pub(crate) async fn citizen_in(
    state: &AppState,
    auth: &Auth,
    society_id: i64,
) -> ApiResult<Option<CitizenRow>> {
    if let Client::ApiKey {
        society_id: key_society,
        ..
    } = auth.client
        && key_society != society_id
    {
        return Err(ApiError::Forbidden(
            "this API key belongs to another society".into(),
        ));
    }
    Ok(state.store.citizen_of(society_id, auth.account_id).await?)
}

/// Reading the game counts as presence (TDD 10.2): a throttled `Seen`.
pub(crate) async fn touch_presence(
    state: &AppState,
    auth: &Auth,
    entry: &SocietyEntry,
) -> ApiResult<()> {
    let Some(citizen) = citizen_in(state, auth, entry.row.id).await? else {
        return Ok(());
    };
    let id = CitizenId(u32::try_from(citizen.citizen_id).unwrap_or(u32::MAX));
    let tick = entry.handle.world.read().await.meta.tick;
    if state.first_sight_this_tick(entry.row.id, id, tick) {
        let env = envelope(Actor::Citizen(id), auth.client_kind(), Command::Seen);
        // A rejection here (e.g. an unknown citizen after a rollover) is not the caller's problem.
        if let Err(reject) = entry.handle.command(env).await? {
            tracing::debug!(society = entry.row.id, citizen = id.0, code = ?reject.code, "seen rejected");
        }
    }
    Ok(())
}

async fn summary(entry: &SocietyEntry) -> SocietySummary {
    let world = entry.handle.world.read().await;
    let clock = clock_of(&world);
    let mut population = 0;
    let mut active_humans = 0;
    let mut householders = 0;
    for c in world.citizens.values() {
        population += 1;
        if c.dormant {
            continue;
        }
        match c.kind {
            CitizenKind::Human => active_humans += 1,
            CitizenKind::Householder => householders += 1,
        }
    }
    let schedule = entry.schedule();
    let next_tick_at = if world.meta.epoch_ended.is_some()
        || schedule.tick_seconds == 0
        || entry.control.is_paused()
    {
        None
    } else {
        Some(schedule.due_at(world.meta.tick))
    };
    SocietySummary {
        id: entry.row.id,
        name: entry.row.name.clone(),
        preset: entry.row.preset.clone(),
        class: entry.row.class.clone(),
        display: world.meta.display.clone(),
        status: entry.row.status.clone(),
        clock,
        population,
        active_humans,
        householders,
        tick_seconds: schedule.tick_seconds,
        next_tick_at,
    }
}

// -- health and docs -----------------------------------------------------------

#[utoipa::path(get, path = "/healthz", summary = "Liveness and how many societies are loaded", security(()), responses((status = 200, body = Health), (status = 503, body = Problem, description = "A society is behind (S1.14)")))]
async fn healthz(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        ok: true,
        societies: state.society_count(),
    })
}

// -- account -------------------------------------------------------------------

#[utoipa::path(
    post, path = "/auth/magic-link", summary = "Request a sign-in link by email (invite code on first sign-in)", security(()), request_body = MagicLinkRequest,
    responses((status = 202, body = MagicLinkSent), (status = 403, body = Problem), (status = 429, body = Problem))
)]
async fn magic_link(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(req): Json<MagicLinkRequest>,
) -> ApiResult<(StatusCode, Json<MagicLinkSent>)> {
    if !state
        .limiter
        .check(&format!("auth:{ip}"), AUTH_IP_PER_SECOND, AUTH_IP_BURST)
    {
        return Err(ApiError::RateLimited);
    }
    let email = normalize_email(&req.email)?;
    let account = if let Some(a) = state.store.account_by_email(&email).await? {
        a
    } else {
        let code = req
            .invite_code
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| {
                ApiError::Forbidden("an invite code is needed to create an account".into())
            })?;
        state
            .store
            .create_account_with_invite(&email, code)
            .await?
            .ok_or_else(|| ApiError::Forbidden("that invite code is not valid".into()))?
    };
    if account.moderation_state != "ok" {
        return Err(ApiError::Forbidden("this account is suspended".into()));
    }
    let token = random_token();
    state
        .store
        .create_magic_link(&hash_token(&token), account.id, expiry(MAGIC_LINK_TTL))
        .await?;
    let url = format!("{}/auth/callback?token={token}", state.base_url);
    state
        .mail
        .send_magic_link(&email, &url)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((StatusCode::ACCEPTED, Json(MagicLinkSent { sent: true })))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct CallbackQuery {
    token: String,
}

#[utoipa::path(
    get, path = "/auth/callback", summary = "Redeem a magic link: sets the session cookie and redirects", security(()), params(CallbackQuery),
    responses((status = 303, description = "Session cookie set; redirect to the app"), (status = 401, body = Problem))
)]
async fn auth_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<CallbackQuery>,
) -> ApiResult<Response> {
    let account_id = state
        .store
        .consume_magic_link(&hash_token(&q.token))
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let session = random_token();
    state
        .store
        .create_session(&hash_token(&session), account_id, expiry(SESSION_TTL))
        .await?;
    let secure = state.base_url.starts_with("https://");
    let cookie = Cookie::build((SESSION_COOKIE, session))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(30))
        .build();
    Ok((
        jar.add(cookie),
        Redirect::to(&format!("{}/", state.base_url)),
    )
        .into_response())
}

#[utoipa::path(post, path = "/auth/logout", summary = "End the browser session", responses((status = 204)), security(("session" = [])))]
async fn logout(State(state): State<AppState>, auth: Auth, jar: CookieJar) -> ApiResult<Response> {
    if auth.client != Client::Web {
        return Err(ApiError::Forbidden("logout is for browser sessions".into()));
    }
    if let Some(token) = jar.get(SESSION_COOKIE) {
        state
            .store
            .delete_session(&hash_token(token.value()))
            .await?;
    }
    let removal = Cookie::build((SESSION_COOKIE, "")).path("/").build();
    Ok((jar.remove(removal), StatusCode::NO_CONTENT).into_response())
}

#[utoipa::path(get, path = "/me", summary = "The signed-in account, its citizenships, and its API keys", responses((status = 200, body = Me), (status = 401, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn me(State(state): State<AppState>, auth: Auth) -> ApiResult<Json<Me>> {
    let account = state
        .store
        .account_by_id(auth.account_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let mut citizenships = Vec::new();
    for c in state.store.citizenships(auth.account_id).await? {
        let citizen_id = u32::try_from(c.citizen_id).unwrap_or(u32::MAX);
        let action_share = state
            .store
            .action_share(c.society_id, citizen_id)
            .await?
            .into_iter()
            .map(|(kind, n)| {
                let kind = kind
                    .as_str()
                    .map_or_else(|| kind.to_string(), str::to_owned);
                (kind, u64::try_from(n).unwrap_or(0))
            })
            .collect();
        let honors = match society(&state, c.society_id) {
            Ok(entry) => entry
                .handle
                .world
                .read()
                .await
                .citizens
                .get(&CitizenId(citizen_id))
                .map_or(0, isms_core::metrics::honors_of),
            Err(_) => 0,
        };
        citizenships.push(Citizenship {
            society_id: c.society_id,
            citizen_id,
            handle: c.handle,
            action_share,
            honors,
        });
    }
    let api_keys = state
        .store
        .list_api_keys(auth.account_id)
        .await?
        .into_iter()
        .map(|k| ApiKeySummary {
            id: k.id,
            label: k.label,
            prefix: k.prefix,
            society_id: k.society_id,
            citizen_id: u32::try_from(k.citizen_id).unwrap_or(u32::MAX),
            created_at: k.created_at,
        })
        .collect();
    let operator = state.operators.contains(&account.email);
    Ok(Json(Me {
        account: Account {
            id: account.id,
            email: account.email,
            consent_version: account.consent_version,
            created_at: account.created_at,
            biography: account.biography,
            operator,
        },
        citizenships,
        api_keys,
    }))
}

// -- operator routes (S1.13c, ADR-0007): the server's operators, named by email --

async fn require_operator(state: &AppState, auth: &Auth) -> ApiResult<()> {
    let account = state
        .store
        .account_by_id(auth.account_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    if state.operators.contains(&account.email) {
        Ok(())
    } else {
        Err(ApiError::Forbidden("operators only".into()))
    }
}

async fn admin_view(state: &AppState, entry: &SocietyEntry) -> AdminSocietyView {
    let statements_close_at = state
        .store
        .latest_archive(entry.row.id)
        .await
        .ok()
        .flatten()
        .filter(|a| a.is_open(chrono::Utc::now()))
        .map(|a| a.closes_at);
    AdminSocietyView {
        summary: summary(entry).await,
        paused: entry.control.is_paused(),
        tick_origin: entry.schedule().tick_origin,
        statements_close_at,
    }
}

#[utoipa::path(get, path = "/admin/societies", summary = "Operator: every society with its clock and whether it is held",
    responses((status = 200, body = AdminSocietiesView), (status = 403, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_societies(
    State(state): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<AdminSocietiesView>> {
    require_operator(&state, &auth).await?;
    let entries: Vec<SocietyEntry> = state
        .societies
        .read()
        .expect("societies lock")
        .values()
        .cloned()
        .collect();
    let mut societies = Vec::with_capacity(entries.len());
    for e in &entries {
        societies.push(admin_view(&state, e).await);
    }
    Ok(Json(AdminSocietiesView { societies }))
}

#[utoipa::path(post, path = "/admin/s/{id}/pause", summary = "Operator: hold the clock; nothing ticks until resumed",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = AdminSocietyView), (status = 403, body = Problem), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_pause(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<AdminSocietyView>> {
    require_operator(&state, &auth).await?;
    let entry = society(&state, id)?;
    entry.control.pause();
    state.store.set_society_status(id, "paused").await?;
    tracing::info!(
        society = id,
        account = auth.account_id,
        "paused by operator"
    );
    Ok(Json(admin_view(&state, &entry).await))
}

#[utoipa::path(post, path = "/admin/s/{id}/resume", summary = "Operator: release the clock with the next tick due one tick length from now (no catch-up)",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = AdminSocietyView), (status = 403, body = Problem), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_resume(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<AdminSocietyView>> {
    require_operator(&state, &auth).await?;
    let entry = society(&state, id)?;
    let next_tick = entry.handle.world.read().await.meta.tick;
    let s = entry.control.resume(next_tick, chrono::Utc::now());
    state
        .store
        .set_society_schedule(
            id,
            i32::try_from(s.tick_seconds).unwrap_or(i32::MAX),
            s.tick_origin,
        )
        .await?;
    state.store.set_society_status(id, "active").await?;
    tracing::info!(
        society = id,
        account = auth.account_id,
        "resumed by operator"
    );
    Ok(Json(admin_view(&state, &entry).await))
}

#[utoipa::path(post, path = "/admin/s/{id}/step", summary = "Operator: resolve exactly one tick while the clock is held",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = AdminSocietyView), (status = 403, body = Problem), (status = 404, body = Problem), (status = 409, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_step(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<AdminSocietyView>> {
    require_operator(&state, &auth).await?;
    let entry = society(&state, id)?;
    if !entry.control.is_paused() {
        return Err(ApiError::BadRequest(
            "pause the society before stepping it".into(),
        ));
    }
    entry.handle.tick().await?;
    Ok(Json(admin_view(&state, &entry).await))
}

#[utoipa::path(post, path = "/admin/s/{id}/tick-seconds", summary = "Operator: a new tick length, re-anchored so the next tick is due one length from now",
    params(("id" = i64, Path, description = "Society id")), request_body = TickSecondsRequest,
    responses((status = 200, body = AdminSocietyView), (status = 403, body = Problem), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_tick_seconds(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<TickSecondsRequest>,
) -> ApiResult<Json<AdminSocietyView>> {
    require_operator(&state, &auth).await?;
    let entry = society(&state, id)?;
    let next_tick = entry.handle.world.read().await.meta.tick;
    let s = entry
        .control
        .set_tick_seconds(req.tick_seconds, next_tick, chrono::Utc::now());
    state
        .store
        .set_society_schedule(
            id,
            i32::try_from(s.tick_seconds).unwrap_or(i32::MAX),
            s.tick_origin,
        )
        .await?;
    tracing::info!(
        society = id,
        tick_seconds = req.tick_seconds,
        "tick length set by operator"
    );
    Ok(Json(admin_view(&state, &entry).await))
}

#[utoipa::path(post, path = "/admin/s/{id}/end-epoch", summary = "Operator: end the epoch now (the only operator command the engine takes)",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = AdminSocietyView), (status = 403, body = Problem), (status = 404, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_end_epoch(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<AdminSocietyView>> {
    require_operator(&state, &auth).await?;
    let entry = society(&state, id)?;
    let env = envelope(
        Actor::System,
        auth.client_kind(),
        Command::EndEpoch {
            reason: "operator".into(),
        },
    );
    entry.handle.command(env).await?.map_err(ApiError::Reject)?;
    tracing::warn!(
        society = id,
        account = auth.account_id,
        "epoch ended by operator"
    );
    Ok(Json(admin_view(&state, &entry).await))
}

#[utoipa::path(patch, path = "/me", summary = "Change what you may about your account: the biography line", request_body = UpdateMe,
    responses((status = 200, body = Me), (status = 400, body = Problem), (status = 401, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn update_me(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<UpdateMe>,
) -> ApiResult<Json<Me>> {
    let biography = req.biography.trim();
    if biography.chars().count() > 140 {
        return Err(ApiError::BadRequest(
            "biography: at most 140 characters".into(),
        ));
    }
    state
        .store
        .set_biography(auth.account_id, biography)
        .await?;
    me(State(state), auth).await
}

#[utoipa::path(post, path = "/admin/s/{id}/new-epoch", summary = "Operator: start the next epoch of an ended society (roster kept, material state reset)",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = AdminSocietyView), (status = 403, body = Problem), (status = 404, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admin_new_epoch(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<AdminSocietyView>> {
    require_operator(&state, &auth).await?;
    let entry = society(&state, id)?;
    // "Start now": the rollover, which also closes the statements window early (S1.15).
    let epoch = crate::runtime::start_next_epoch(&state.store, id, &entry.handle, &entry.control)
        .await?
        .map_err(ApiError::Reject)?;
    tracing::warn!(
        society = id,
        epoch,
        account = auth.account_id,
        "new epoch started by operator"
    );
    Ok(Json(admin_view(&state, &entry).await))
}

// -- spectator routes: no citizenship, no session (TDD 10.2) --------------------

#[utoipa::path(get, path = "/public/societies", summary = "Every public society on this server, for anyone", security(()), responses((status = 200, body = SocietyList)))]
async fn public_societies(State(state): State<AppState>) -> ApiResult<Json<SocietyList>> {
    let entries: Vec<SocietyEntry> = state
        .societies
        .read()
        .expect("societies lock")
        .values()
        .filter(|e| is_public(e))
        .cloned()
        .collect();
    let mut societies = Vec::with_capacity(entries.len());
    for e in &entries {
        societies.push(summary(e).await);
    }
    Ok(Json(SocietyList { societies }))
}

#[utoipa::path(get, path = "/public/s/{id}/stats", summary = "The numbers a society keeps, for anyone", security(()),
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = PublicStatsView), (status = 404, body = Problem)))]
async fn public_stats(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<PublicStatsView>> {
    let entry = public_society(&state, id)?;
    let numbers = crate::society_api::stats_of(&state, &entry, id).await?;
    let world = entry.handle.world.read().await;
    let c = &world.constitution;
    Ok(Json(PublicStatsView {
        clock: numbers.clock,
        name: entry.row.name.clone(),
        display: world.meta.display.clone(),
        preset: entry.row.preset.clone(),
        money: c.has_money(),
        credit: c
            .contracts
            .contains(&isms_core::kinds::ContractKind::Credit),
        orgs: !c.org_kinds.is_empty(),
        live: numbers.live,
        firm_count: numbers.firm_count,
        credit_outstanding: numbers.credit_outstanding,
        last_cycle: numbers.last_cycle,
    }))
}

#[utoipa::path(get, path = "/public/s/{id}/archives", summary = "Past epochs of a society, for anyone: each one's frozen summary and closing statements", security(()),
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = ArchivesView), (status = 404, body = Problem)))]
async fn public_archives(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ArchivesView>> {
    let entry = public_society(&state, id)?;
    Ok(Json(
        crate::society_api::archives_of(&state, &entry, None).await?,
    ))
}

#[utoipa::path(get, path = "/public/s/{id}/archives/{epoch}", summary = "One past epoch of a society, for anyone", security(()),
    params(("id" = i64, Path, description = "Society id"), ("epoch" = u32, Path, description = "The epoch as the clock shows it (1-based)")),
    responses((status = 200, body = ArchiveView), (status = 404, body = Problem)))]
async fn public_archive(
    State(state): State<AppState>,
    Path((id, epoch)): Path<(i64, u32)>,
) -> ApiResult<Json<ArchiveView>> {
    let entry = public_society(&state, id)?;
    Ok(Json(
        crate::society_api::archive_of(&state, &entry, epoch, None).await?,
    ))
}

#[utoipa::path(
    post, path = "/me/api-keys", summary = "Create an API key scoped to one of your citizens", request_body = NewApiKey,
    responses((status = 201, body = ApiKeyCreated), (status = 403, body = Problem)), security(("session" = []))
)]
async fn create_api_key(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<NewApiKey>,
) -> ApiResult<(StatusCode, Json<ApiKeyCreated>)> {
    if auth.client != Client::Web {
        return Err(ApiError::Forbidden(
            "keys are created from a browser session".into(),
        ));
    }
    let label = req.label.trim();
    if label.is_empty() || label.len() > 64 {
        return Err(ApiError::BadRequest("label must be 1-64 characters".into()));
    }
    let citizen = state
        .store
        .citizen_of(req.society_id, auth.account_id)
        .await?
        .ok_or_else(|| {
            ApiError::Forbidden("join the society before creating a key for it".into())
        })?;
    let minted = mint_api_key();
    let row = state
        .store
        .create_api_key(
            auth.account_id,
            req.society_id,
            citizen.citizen_id,
            label,
            &minted.prefix,
            &minted.hash,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiKeyCreated {
            id: row.id,
            label: row.label,
            prefix: row.prefix,
            key: minted.key,
        }),
    ))
}

#[utoipa::path(
    delete, path = "/me/api-keys/{id}", summary = "Revoke an API key", params(("id" = i64, Path, description = "Key id")),
    responses((status = 204), (status = 404, body = Problem)), security(("session" = []))
)]
async fn revoke_api_key(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if state.store.revoke_api_key(id, auth.account_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("no live key {id}")))
    }
}

// -- societies -----------------------------------------------------------------

#[utoipa::path(get, path = "/societies", summary = "Every society on this server", responses((status = 200, body = SocietyList), (status = 401, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn list_societies(
    State(state): State<AppState>,
    _auth: Auth,
) -> ApiResult<Json<SocietyList>> {
    let entries: Vec<SocietyEntry> = state
        .societies
        .read()
        .expect("societies lock")
        .values()
        .cloned()
        .collect();
    let mut societies = Vec::with_capacity(entries.len());
    for e in &entries {
        societies.push(summary(e).await);
    }
    Ok(Json(SocietyList { societies }))
}

#[utoipa::path(
    get, path = "/societies/{id}", summary = "One society and where its clock stands", params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = SocietySummary), (status = 404, body = Problem)), security(("session" = []), ("api_key" = []))
)]
async fn get_society(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<SocietySummary>> {
    let entry = society(&state, id)?;
    touch_presence(&state, &auth, &entry).await?;
    Ok(Json(summary(&entry).await))
}

#[utoipa::path(
    get, path = "/societies/{id}/capabilities", summary = "What exists in this society (constitution-derived capabilities)", params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = CapabilitiesView), (status = 404, body = Problem)), security(("session" = []), ("api_key" = []))
)]
async fn capabilities(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<CapabilitiesView>> {
    let entry = society(&state, id)?;
    touch_presence(&state, &auth, &entry).await?;
    let world = entry.handle.world.read().await;
    let rules = Rules::from_world(&world);
    Ok(Json(CapabilitiesView::from_capabilities(
        clock_of(&world),
        &rules.capabilities,
    )))
}

#[utoipa::path(
    get, path = "/societies/{id}/lexicon", summary = "This society's vocabulary for the shared UI labels", params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = Lexicon), (status = 404, body = Problem)), security(("session" = []), ("api_key" = []))
)]
async fn lexicon(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<Lexicon>> {
    let entry = society(&state, id)?;
    touch_presence(&state, &auth, &entry).await?;
    let entries = isms_core::lexicon::load_lexicon(&state.presets_dir, &entry.row.preset)
        .map_err(|e| ApiError::Internal(format!("lexicon: {e}")))?;
    Ok(Json(Lexicon {
        clock: clock(&entry.handle).await,
        preset: entry.row.preset.clone(),
        entries,
    }))
}

/// Fill the Welcome Brief's placeholders from the live world.
fn render_welcome(template: &str, world: &isms_core::world::World) -> String {
    let endowment = format!("{:.2}", world.params.money.endowment.as_credits_f64());
    let open_slots = world
        .land
        .slots
        .values()
        .filter(|s| s.workplace.is_none())
        .count();
    template
        .replace("{{endowment}}", &endowment)
        .replace("{{open_slots}}", &open_slots.to_string())
}

#[utoipa::path(
    get, path = "/societies/{id}/welcome", summary = "The Welcome Brief, in this society's voice", params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = Welcome), (status = 404, body = Problem)), security(("session" = []), ("api_key" = []))
)]
async fn welcome(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<Welcome>> {
    let entry = society(&state, id)?;
    touch_presence(&state, &auth, &entry).await?;
    let path = state
        .presets_dir
        .join("copy")
        .join(&entry.row.preset)
        .join("welcome.md");
    let template = std::fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(format!("welcome copy {}: {e}", path.display())))?;
    let world = entry.handle.world.read().await;
    Ok(Json(Welcome {
        clock: clock_of(&world),
        markdown: render_welcome(&template, &world),
    }))
}

#[utoipa::path(
    post, path = "/societies/{id}/join", summary = "Become a citizen of this society (idempotent per account)", params(("id" = i64, Path, description = "Society id")), request_body = JoinRequest,
    responses((status = 200, body = Joined, description = "Already a citizen"), (status = 201, body = Joined), (status = 422, body = Problem)),
    security(("session" = []))
)]
async fn join(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<JoinRequest>,
) -> ApiResult<(StatusCode, Json<Joined>)> {
    let entry = society(&state, id)?;
    if let Some(existing) = citizen_in(&state, &auth, id).await? {
        return Ok((
            StatusCode::OK,
            Json(Joined {
                clock: clock(&entry.handle).await,
                citizen_id: u32::try_from(existing.citizen_id).unwrap_or(u32::MAX),
                handle: existing.handle,
                created: false,
            }),
        ));
    }
    let handle = req.handle.trim().to_owned();
    if !valid_handle(&handle) {
        return Err(ApiError::BadRequest(
            "a handle is 2-24 letters, digits, _ or -".into(),
        ));
    }
    if !state
        .limiter
        .check(&format!("join:{}", auth.account_id), 1.0 / 10.0, 3.0)
    {
        return Err(ApiError::RateLimited);
    }
    // Q10: the system issues Join for a person who is not yet a citizen.
    let env = envelope(
        Actor::System,
        auth.client_kind(),
        Command::Join {
            handle: handle.clone(),
            kind: CitizenKind::Human,
        },
    );
    let events = entry
        .handle
        .command(env)
        .await?
        .map_err(ApiError::Reject)?
        .events;
    let citizen = events
        .iter()
        .find_map(|e| match e {
            Event::CitizenJoined { citizen, .. } => Some(*citizen),
            _ => None,
        })
        .ok_or_else(|| ApiError::Internal("Join produced no CitizenJoined".into()))?;
    let row = CitizenRow {
        society_id: id,
        citizen_id: i32::try_from(citizen.0).unwrap_or(i32::MAX),
        account_id: auth.account_id,
        handle: handle.clone(),
    };
    if !state.store.insert_citizen(&row).await? {
        // Lost a race with ourselves; the earlier citizen is the one that counts.
        let existing = state
            .store
            .citizen_of(id, auth.account_id)
            .await?
            .ok_or_else(|| ApiError::Internal("citizen row vanished".into()))?;
        return Ok((
            StatusCode::OK,
            Json(Joined {
                clock: clock(&entry.handle).await,
                citizen_id: u32::try_from(existing.citizen_id).unwrap_or(u32::MAX),
                handle: existing.handle,
                created: false,
            }),
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(Joined {
            clock: clock(&entry.handle).await,
            citizen_id: citizen.0,
            handle,
            created: true,
        }),
    ))
}

// -- OpenAPI and router --------------------------------------------------------

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "session",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE))),
        );
        components.add_security_scheme(
            "api_key",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Isms", description = "The public API of Isms societies. Every client, web or agent, uses it (TDD 10)."),
    servers((url = "/", description = "This server")),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

fn openapi_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(healthz))
        .routes(routes!(magic_link))
        .routes(routes!(auth_callback))
        .routes(routes!(logout))
        .routes(routes!(me, update_me))
        .routes(routes!(public_societies))
        .routes(routes!(public_stats))
        .routes(routes!(public_archives))
        .routes(routes!(public_archive))
        .routes(routes!(admin_societies))
        .routes(routes!(admin_pause))
        .routes(routes!(admin_resume))
        .routes(routes!(admin_step))
        .routes(routes!(admin_tick_seconds))
        .routes(routes!(admin_end_epoch))
        .routes(routes!(admin_new_epoch))
        .routes(routes!(create_api_key))
        .routes(routes!(revoke_api_key))
        .routes(routes!(list_societies))
        .routes(routes!(get_society))
        .routes(routes!(capabilities))
        .routes(routes!(lexicon))
        .routes(routes!(welcome))
        .routes(routes!(join))
        .merge(crate::society_api::routes())
        .merge(crate::assembly::routes())
        .merge(crate::commons::routes())
        .merge(crate::comms::routes())
}

/// The `OpenAPI` document, without a running server (`isms-server openapi`).
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    openapi_router().split_for_parts().1
}

/// The whole app: API routes, `/openapi.json`, `/docs`.
pub fn router(state: AppState) -> axum::Router {
    let (router, api) = openapi_router().split_for_parts();
    router
        .merge(SwaggerUi::new("/docs").url("/openapi.json", api))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(64 * 1024))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

/// Stamp for tests and logs: which client kind an `Auth` maps to.
#[must_use]
pub fn client_kind_of(auth: &Auth) -> ClientKind {
    auth.client_kind()
}
