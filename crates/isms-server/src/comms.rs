//! Chronicle reader and free-text channels (TDD 10.3 Society and Comms rows,
//! TDD 12, D12): the Square, org channels with membership checks, and DMs.

use crate::api::{citizen_in, society, touch_presence};
use crate::auth::Auth;
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, SocietyEntry, clock_of};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use isms_api_types::Problem;
use isms_api_types::chronicle::{
    ChronicleView, HeadlineView, MessageView, MessagesView, PostMessage,
};
use isms_core::ids::{CitizenId, OrgId};
use isms_core::ledger::Party;
use isms_core::world::{ContractBody, ContractStatus, World};
use isms_store::projections::{HeadlineRow, MessageRow};
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// Longest message body, in bytes.
const MAX_BODY: usize = 2_000;
/// Messages per page.
const PAGE: i64 = 100;
/// Message budget per citizen: one per 2 s, burst 10.
const MSG_PER_SECOND: f64 = 0.5;
const MSG_BURST: f64 = 10.0;

async fn me_in(state: &AppState, auth: &Auth, id: i64) -> ApiResult<(SocietyEntry, CitizenId)> {
    let entry = society(state, id)?;
    let row = citizen_in(state, auth, id)
        .await?
        .ok_or_else(|| ApiError::Forbidden("join this society first".into()))?;
    touch_presence(state, auth, &entry).await?;
    Ok((
        entry,
        CitizenId(u32::try_from(row.citizen_id).unwrap_or(u32::MAX)),
    ))
}

fn headline_view(h: HeadlineRow) -> HeadlineView {
    HeadlineView {
        seq: h.source_event_seq,
        ordinal: h.ordinal,
        cycle: u32::try_from(h.cycle).unwrap_or(0) + 1,
        tick: u32::try_from(h.tick).unwrap_or(0),
        text: h.headline,
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct CycleQuery {
    /// 1-based cycle; default the current one.
    cycle: Option<u32>,
}

#[utoipa::path(get, path = "/s/{id}/chronicle", summary = "One cycle's edition of the Chronicle",
    params(("id" = i64, Path, description = "Society id"), CycleQuery),
    responses((status = 200, body = ChronicleView)), security(("session" = []), ("api_key" = [])))]
async fn chronicle(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Query(q): Query<CycleQuery>,
) -> ApiResult<Json<ChronicleView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    Ok(Json(chronicle_view(&state, &entry, id, q.cycle).await?))
}

/// One edition; shared with the spectator route (S1.13).
async fn chronicle_view(
    state: &AppState,
    entry: &SocietyEntry,
    id: i64,
    cycle: Option<u32>,
) -> ApiResult<ChronicleView> {
    let clock = clock_of(&*entry.handle.world.read().await);
    let cycle = cycle.unwrap_or(clock.cycle).max(1);
    let rows = state
        .store
        .headlines_for_cycle(id, i32::try_from(cycle - 1).unwrap_or(i32::MAX))
        .await?;
    Ok(ChronicleView {
        clock,
        cycle,
        headlines: rows.into_iter().map(headline_view).collect(),
    })
}

#[utoipa::path(get, path = "/public/s/{id}/chronicle", summary = "One edition of the Chronicle, for anyone", security(()),
    params(("id" = i64, Path, description = "Society id"), CycleQuery),
    responses((status = 200, body = ChronicleView), (status = 404, body = Problem)))]
async fn public_chronicle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<CycleQuery>,
) -> ApiResult<Json<ChronicleView>> {
    let entry = crate::api::public_society(&state, id)?;
    Ok(Json(chronicle_view(&state, &entry, id, q.cycle).await?))
}

/// The latest headlines, for the Situation view.
pub async fn latest_headlines(state: &AppState, id: i64, n: i64) -> ApiResult<Vec<HeadlineView>> {
    Ok(state
        .store
        .latest_headlines(id, n)
        .await?
        .into_iter()
        .map(headline_view)
        .collect())
}

/// Who may read and write a channel (TDD 12).
fn can_access(world: &World, me: CitizenId, channel: &str) -> bool {
    if channel == "square" {
        return true;
    }
    if let Some(org) = channel.strip_prefix("org:") {
        let Some(org) = org.parse::<u32>().ok().map(OrgId) else {
            return false;
        };
        let Some(o) = world.orgs.get(&org) else {
            return false;
        };
        if o.manager == Some(me) || o.members.contains(&me) {
            return true;
        }
        return world.contracts.values().any(|k| {
            k.status == ContractStatus::Active
                && matches!(k.body, ContractBody::Employment { org: o2, .. } if o2 == org)
                && (k.parties.0 == Party::Citizen(me) || k.parties.1 == Party::Citizen(me))
        });
    }
    if let Some(pair) = channel.strip_prefix("dm:") {
        return pair
            .split(':')
            .filter_map(|s| s.parse::<u32>().ok())
            .any(|c| c == me.0);
    }
    false
}

fn dm_channel(a: CitizenId, b: CitizenId) -> String {
    let (lo, hi) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
    format!("dm:{lo}:{hi}")
}

async fn read_channel(
    state: &AppState,
    entry: &SocietyEntry,
    me: CitizenId,
    channel: &str,
    after: i64,
) -> ApiResult<MessagesView> {
    let world = entry.handle.world.read().await;
    if !can_access(&world, me, channel) {
        return Err(ApiError::Forbidden("you are not in this channel".into()));
    }
    let rows = state
        .store
        .messages(entry.row.id, channel, after, PAGE)
        .await?;
    let messages = rows
        .into_iter()
        .map(|m: MessageRow| MessageView {
            id: m.id,
            channel: m.channel,
            sender: u32::try_from(m.sender_citizen).unwrap_or(u32::MAX),
            handle: world
                .citizens
                .get(&CitizenId(
                    u32::try_from(m.sender_citizen).unwrap_or(u32::MAX),
                ))
                .map(|c| c.handle.clone())
                .unwrap_or_default(),
            body: m.body,
            tick: u32::try_from(m.tick).unwrap_or(0),
            created_at: m.created_at,
        })
        .collect();
    Ok(MessagesView {
        clock: clock_of(&world),
        channel: channel.to_owned(),
        messages,
    })
}

async fn post_channel(
    state: &AppState,
    entry: &SocietyEntry,
    me: CitizenId,
    channel: &str,
    body: &str,
) -> ApiResult<MessageView> {
    let body = body.trim();
    if body.is_empty() || body.len() > MAX_BODY {
        return Err(ApiError::BadRequest(format!(
            "a message is 1 to {MAX_BODY} bytes"
        )));
    }
    let key = format!("msg:{}:{}", entry.row.id, me.0);
    if !state.limiter.check(&key, MSG_PER_SECOND, MSG_BURST) {
        return Err(ApiError::RateLimited);
    }
    let (tick, handle) = {
        let world = entry.handle.world.read().await;
        if !can_access(&world, me, channel) {
            return Err(ApiError::Forbidden("you are not in this channel".into()));
        }
        (
            world.meta.tick,
            world
                .citizens
                .get(&me)
                .map(|c| c.handle.clone())
                .unwrap_or_default(),
        )
    };
    let sender = i32::try_from(me.0).unwrap_or(i32::MAX);
    let row = state
        .store
        .insert_message(
            entry.row.id,
            channel,
            sender,
            body,
            i32::try_from(tick).unwrap_or(i32::MAX),
        )
        .await?;
    Ok(MessageView {
        id: row.id,
        channel: row.channel,
        sender: me.0,
        handle,
        body: row.body,
        tick,
        created_at: row.created_at,
    })
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct AfterQuery {
    /// Return messages with an id above this.
    after: Option<i64>,
}

#[utoipa::path(get, path = "/s/{id}/channels/{channel}/messages", summary = "Read the Square or an org channel",
    params(("id" = i64, Path, description = "Society id"), ("channel" = String, Path, description = "square or org:<org id>"), AfterQuery),
    responses((status = 200, body = MessagesView), (status = 403, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn read_messages(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, channel)): Path<(i64, String)>,
    Query(q): Query<AfterQuery>,
) -> ApiResult<Json<MessagesView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    if channel.starts_with("dm:") {
        return Err(ApiError::BadRequest("DMs live under /dm/{citizen}".into()));
    }
    Ok(Json(
        read_channel(&state, &entry, me, &channel, q.after.unwrap_or(0)).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/channels/{channel}/messages", summary = "Post to the Square or an org channel",
    params(("id" = i64, Path, description = "Society id"), ("channel" = String, Path, description = "square or org:<org id>")),
    request_body = PostMessage, responses((status = 201, body = MessageView), (status = 403, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn post_message(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, channel)): Path<(i64, String)>,
    Json(req): Json<PostMessage>,
) -> ApiResult<(StatusCode, Json<MessageView>)> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    if channel.starts_with("dm:") {
        return Err(ApiError::BadRequest("DMs live under /dm/{citizen}".into()));
    }
    let m = post_channel(&state, &entry, me, &channel, &req.body).await?;
    Ok((StatusCode::CREATED, Json(m)))
}

#[utoipa::path(get, path = "/s/{id}/dm/{citizen}", summary = "Read your direct messages with a citizen",
    params(("id" = i64, Path, description = "Society id"), ("citizen" = u32, Path, description = "The other citizen"), AfterQuery),
    responses((status = 200, body = MessagesView)), security(("session" = []), ("api_key" = [])))]
async fn read_dm(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, other)): Path<(i64, u32)>,
    Query(q): Query<AfterQuery>,
) -> ApiResult<Json<MessagesView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let channel = dm_channel(me, CitizenId(other));
    Ok(Json(
        read_channel(&state, &entry, me, &channel, q.after.unwrap_or(0)).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/dm/{citizen}", summary = "Send a direct message (logged; players are told so)",
    params(("id" = i64, Path, description = "Society id"), ("citizen" = u32, Path, description = "The other citizen")),
    request_body = PostMessage, responses((status = 201, body = MessageView), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn post_dm(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, other)): Path<(i64, u32)>,
    Json(req): Json<PostMessage>,
) -> ApiResult<(StatusCode, Json<MessageView>)> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    if other == me.0 {
        return Err(ApiError::BadRequest("that is you".into()));
    }
    if !entry
        .handle
        .world
        .read()
        .await
        .citizens
        .contains_key(&CitizenId(other))
    {
        return Err(ApiError::NotFound(format!("no citizen {other}")));
    }
    let channel = dm_channel(me, CitizenId(other));
    let m = post_channel(&state, &entry, me, &channel, &req.body).await?;
    Ok((StatusCode::CREATED, Json(m)))
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(chronicle))
        .routes(routes!(public_chronicle))
        .routes(routes!(read_messages, post_message))
        .routes(routes!(read_dm, post_dm))
}
