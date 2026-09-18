//! In-society endpoints (TDD 10.3, S1.4): thin adapters from wire requests to
//! engine commands and from `World` to views. Every handler resolves the
//! caller's citizen, counts the read as presence, and filters through `Viewer`.

use crate::actor::{CommandOk, envelope};
use crate::api::{citizen_in, society, touch_presence};
use crate::auth::Auth;
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, SocietyEntry, clock_of};
use crate::viewer::{Viewer, explains_in};
use crate::views;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use isms_api_types::Problem;
use isms_api_types::society::{
    AcceptRequest, AddWorkplaceRequest, AppointRequest, BookView, BooksView, CitizensView,
    Committed, ContractsView, CreditOfferRequest, DigestView, DividendRequest,
    EmploymentOfferRequest, EventRef, ExplainView, FormerOrg, FoundOrgRequest, Headline, HomeView,
    HouseholdersView, IssueSharesRequest, LeaseOfferRequest, MachinesRequest, MemberRequest,
    NoticeBoardView, OrgLedgerView, OrgView, OrgsView, PaymentMissedView, PayslipsView,
    PlaceOrderRequest, PlanView, PricePoint, PricesView, SaleOfferRequest, ScoreboardView,
    SetLaborRequest, SetPlanRequest, StatsView, TransferRequest, WantedRequest, cents,
    instrument_name, parse_instrument,
};
use isms_core::command::Command;
use isms_core::event::{Actor, Event};
use isms_core::ids::{
    CitizenId, ContractId, DwellingId, OfferId, OrderId, OrgId, SlotId, WorkplaceId,
};
use isms_core::money::Money;
use isms_core::plan::{away_digest, touches};
use isms_core::rules::Rules;
use isms_core::world::{Allocation, Instrument, OfferBody, Side, StandingPlan, World};
use isms_store::StoredEvent;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// Digest and payslip pages never exceed this many events.
const PAGE: usize = 200;
/// How far back the away digest and the tape look, in events read.
const READ_CAP: i64 = 20_000;

// -- helpers -------------------------------------------------------------------

/// The society and the caller's citizen in it (403 for non-citizens), with presence counted.
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

fn event_ref(viewer: &Viewer, world: &World, e: &StoredEvent) -> Option<EventRef> {
    let payload = viewer.view_event(world, &e.event)?;
    Some(EventRef {
        seq: e.seq,
        tick: e.meta.tick,
        cycle: e.meta.cycle,
        epoch: e.meta.epoch,
        kind: e.event.kind().to_owned(),
        payload,
    })
}

/// Send a command as the citizen (or for an org they manage) and report the events.
async fn send(
    state: &AppState,
    entry: &SocietyEntry,
    auth: &Auth,
    citizen: CitizenId,
    on_behalf_of: Option<OrgId>,
    command: Command,
) -> ApiResult<Committed> {
    {
        let world = entry.handle.world.read().await;
        let limit = Rules::from_world(&world).capabilities.rate_limit;
        let key = format!("cmd:{}:{}", entry.row.id, citizen.0);
        if !state
            .limiter
            .check(&key, f64::from(limit.per_second), f64::from(limit.burst))
        {
            return Err(ApiError::RateLimited);
        }
    }
    let mut env = envelope(Actor::Citizen(citizen), auth.client_kind(), command);
    env.on_behalf_of = on_behalf_of;
    let CommandOk {
        first_seq,
        tick,
        cycle,
        events,
    } = entry.handle.command(env).await?.map_err(ApiError::Reject)?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(citizen));
    let refs = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let payload = viewer.view_event(&world, e)?;
            Some(EventRef {
                seq: first_seq.map_or(0, |s| s + i64::try_from(i).unwrap_or(0)),
                tick,
                cycle,
                epoch: world.meta.epoch,
                kind: e.kind().to_owned(),
                payload,
            })
        })
        .collect();
    Ok(Committed {
        clock: clock_of(&world),
        first_seq,
        events: refs,
    })
}

fn side(s: &str) -> ApiResult<Side> {
    match s {
        "bid" => Ok(Side::Bid),
        "ask" => Ok(Side::Ask),
        other => Err(ApiError::BadRequest(format!(
            "side must be bid or ask, not {other}"
        ))),
    }
}

fn instrument(s: &str) -> ApiResult<Instrument> {
    parse_instrument(s).ok_or_else(|| ApiError::BadRequest(format!("unknown instrument {s}")))
}

fn org_of(n: Option<u32>) -> Option<OrgId> {
    n.map(OrgId)
}

// -- me in society -------------------------------------------------------------

#[utoipa::path(get, path = "/s/{id}/home", summary = "The Situation view: household, labor, needs, plan, what happened while you were away",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = HomeView), (status = 403, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn home(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<HomeView>> {
    let entry = society(&state, id)?;
    let row = citizen_in(&state, &auth, id)
        .await?
        .ok_or_else(|| ApiError::Forbidden("join this society first".into()))?;
    let me = CitizenId(u32::try_from(row.citizen_id).unwrap_or(u32::MAX));
    // Read where the last session ended before this one counts as presence.
    let (since, epoch) = {
        let world = entry.handle.world.read().await;
        (
            world.citizens.get(&me).map_or(0, |c| c.last_seen_tick),
            world.meta.epoch,
        )
    };
    touch_presence(&state, &auth, &entry).await?;
    let stored = state
        .store
        .read_since_tick(id, epoch, since, READ_CAP)
        .await?;
    let headlines = crate::comms::latest_headlines(&state, id, 5)
        .await?
        .into_iter()
        .map(|h| Headline {
            seq: h.seq,
            tick: h.tick,
            text: h.text,
        })
        .collect();
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let raw: Vec<Event> = stored.iter().map(|e| e.event.clone()).collect();
    let mut digest: Vec<EventRef> = away_digest(me, &raw)
        .into_iter()
        .filter_map(|i| event_ref(&viewer, &world, &stored[i]))
        .collect();
    if digest.len() > PAGE {
        digest.drain(..digest.len() - PAGE);
    }
    let citizen =
        views::citizen_self(&world, me).ok_or_else(|| ApiError::NotFound("citizen".into()))?;
    let plan = world
        .citizens
        .get(&me)
        .map(|c| serde_json::to_value(&c.plan).unwrap_or_default())
        .unwrap_or_default();
    Ok(Json(HomeView {
        clock: clock_of(&world),
        citizen,
        household: views::household(&world, me)
            .ok_or_else(|| ApiError::NotFound("citizen".into()))?,
        needs: views::needs(&world, me).ok_or_else(|| ApiError::NotFound("citizen".into()))?,
        labor: views::labor(&world, &viewer, me)
            .ok_or_else(|| ApiError::NotFound("citizen".into()))?,
        plan,
        since_last_seen: DigestView {
            since_tick: since,
            events: digest,
        },
        headlines,
        society: views::pulse(&world),
    }))
}

#[utoipa::path(get, path = "/s/{id}/plan", summary = "Your standing plan", params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = PlanView)), security(("session" = []), ("api_key" = [])))]
async fn get_plan(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<PlanView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    let plan = world
        .citizens
        .get(&me)
        .map(|c| serde_json::to_value(&c.plan).unwrap_or_default())
        .unwrap_or_default();
    Ok(Json(PlanView {
        clock: clock_of(&world),
        plan,
    }))
}

#[utoipa::path(put, path = "/s/{id}/plan", summary = "Replace your standing plan", params(("id" = i64, Path, description = "Society id")),
    request_body = SetPlanRequest, responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn set_plan(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<SetPlanRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let plan: StandingPlan =
        serde_json::from_value(req.plan).map_err(|e| ApiError::BadRequest(format!("plan: {e}")))?;
    let cmd = Command::SetStandingPlan {
        plan: Box::new(plan),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(put, path = "/s/{id}/labor", summary = "Set your labor allocation (hours and effort per workplace)",
    params(("id" = i64, Path, description = "Society id")), request_body = SetLaborRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn set_labor(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<SetLaborRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let allocations = req
        .allocations
        .into_iter()
        .map(|a| Allocation {
            workplace: WorkplaceId(a.workplace),
            hours: a.hours,
            effort: a.effort,
        })
        .collect();
    let cmd = Command::SetLabor { allocations };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(get, path = "/s/{id}/payslips", summary = "Your payslips, newest last, each with its Explain",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = PayslipsView)), security(("session" = []), ("api_key" = [])))]
async fn payslips(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<PayslipsView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let stored = state.store.read_last_of_kind(id, "Paid", READ_CAP).await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let mut payslips: Vec<EventRef> = stored
        .iter()
        .filter(|e| touches(&e.event, me))
        .filter_map(|e| event_ref(&viewer, &world, e))
        .collect();
    if payslips.len() > PAGE {
        payslips.drain(..payslips.len() - PAGE);
    }
    Ok(Json(PayslipsView {
        clock: clock_of(&world),
        payslips,
    }))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct SinceQuery {
    /// Engine tick to start from; default: your last seen tick.
    since: Option<u32>,
}

#[utoipa::path(get, path = "/s/{id}/away-digest", summary = "Everything that touched you since a tick",
    params(("id" = i64, Path, description = "Society id"), SinceQuery), responses((status = 200, body = DigestView)), security(("session" = []), ("api_key" = [])))]
async fn digest(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Query(q): Query<SinceQuery>,
) -> ApiResult<Json<DigestView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let since = match q.since {
        Some(s) => s,
        None => entry
            .handle
            .world
            .read()
            .await
            .citizens
            .get(&me)
            .map_or(0, |c| c.last_seen_tick),
    };
    let epoch = entry.handle.world.read().await.meta.epoch;
    let stored = state
        .store
        .read_since_tick(id, epoch, since, READ_CAP)
        .await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let raw: Vec<Event> = stored.iter().map(|e| e.event.clone()).collect();
    let mut events: Vec<EventRef> = away_digest(me, &raw)
        .into_iter()
        .filter_map(|i| event_ref(&viewer, &world, &stored[i]))
        .collect();
    if events.len() > PAGE {
        events.drain(..events.len() - PAGE);
    }
    Ok(Json(DigestView {
        since_tick: since,
        events,
    }))
}

// -- market --------------------------------------------------------------------

#[utoipa::path(get, path = "/s/{id}/books", summary = "Every instrument with last price and depth",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = BooksView)), security(("session" = []), ("api_key" = [])))]
async fn books(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<BooksView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    Ok(Json(BooksView {
        clock: clock_of(&world),
        price_index: world.price_index,
        books: views::instruments(&world)
            .into_iter()
            .map(|i| views::book_summary(&world, i))
            .collect(),
    }))
}

#[utoipa::path(get, path = "/s/{id}/books/{instrument}", summary = "One order book: depth, your orders, the recent tape",
    params(("id" = i64, Path, description = "Society id"), ("instrument" = String, Path, description = "food, wares, ... or share:<org>")),
    responses((status = 200, body = BookView), (status = 400, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn book(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, name)): Path<(i64, String)>,
) -> ApiResult<Json<BookView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let i = instrument(&name)?;
    let trades = state.store.read_last_of_kind(id, "Trade", 500).await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let mut view = views::book(&world, &viewer, i);
    let wanted = serde_json::to_value(i).unwrap_or_default();
    let mut tape: Vec<EventRef> = trades
        .iter()
        .filter(|e| matches!(&e.event, Event::Trade { instrument, .. } if serde_json::to_value(instrument).unwrap_or_default() == wanted))
        .filter_map(|e| event_ref(&viewer, &world, e))
        .collect();
    if tape.len() > 50 {
        tape.drain(..tape.len() - 50);
    }
    view.tape = tape;
    Ok(Json(view))
}

#[utoipa::path(post, path = "/s/{id}/orders", summary = "Place a limit order (money or goods are escrowed)",
    params(("id" = i64, Path, description = "Society id")), request_body = PlaceOrderRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn place_order(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<PlaceOrderRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::PlaceOrder {
        instrument: instrument(&req.instrument)?,
        side: side(&req.side)?,
        qty: req.qty,
        limit_price: Money(req.limit_price),
        expires_tick: req.expires_tick,
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct OnBehalfQuery {
    on_behalf_of: Option<u32>,
}

#[utoipa::path(delete, path = "/s/{id}/orders/{oid}", summary = "Cancel an order and release its escrow",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Order id"), OnBehalfQuery),
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn cancel_order(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Query(q): Query<OnBehalfQuery>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::CancelOrder {
        order: OrderId(oid),
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(q.on_behalf_of), cmd).await?,
    ))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct WindowQuery {
    /// Ticks of history; default one cycle.
    window: Option<u32>,
}

#[utoipa::path(get, path = "/s/{id}/prices", summary = "Per-tick VWAP per instrument over a window of ticks",
    params(("id" = i64, Path, description = "Society id"), WindowQuery), responses((status = 200, body = PricesView)), security(("session" = []), ("api_key" = [])))]
async fn prices(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Query(q): Query<WindowQuery>,
) -> ApiResult<Json<PricesView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    let (now, tpc, epoch) = {
        let world = entry.handle.world.read().await;
        (world.meta.tick, world.ticks_per_cycle(), world.meta.epoch)
    };
    let window = q.window.unwrap_or(tpc).clamp(1, 10 * tpc);
    let since = now.saturating_sub(window);
    let stored = state
        .store
        .read_kind_since_tick(id, epoch, "TickResolved", since, i64::from(window) + 1)
        .await?;
    let mut points = Vec::new();
    for e in &stored {
        if let Event::TickResolved { tick, vwap, .. } = &e.event {
            for (i, p) in vwap {
                points.push(PricePoint {
                    tick: *tick,
                    instrument: instrument_name(*i),
                    vwap: cents(*p),
                });
            }
        }
    }
    let world = entry.handle.world.read().await;
    Ok(Json(PricesView {
        clock: clock_of(&world),
        window,
        points,
    }))
}

// -- orgs ----------------------------------------------------------------------

#[utoipa::path(get, path = "/s/{id}/orgs", summary = "Every organization", params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = OrgsView)), security(("session" = []), ("api_key" = [])))]
async fn orgs(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<OrgsView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let founded = state
        .store
        .read_last_of_kind(id, "OrgFounded", READ_CAP)
        .await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let former = founded
        .iter()
        .filter_map(|e| match &e.event {
            Event::OrgFounded { org, name, .. } if !world.orgs.contains_key(org) => {
                Some(FormerOrg {
                    id: org.0,
                    name: name.clone(),
                    epoch: e.meta.epoch,
                })
            }
            _ => None,
        })
        .collect();
    Ok(Json(OrgsView {
        clock: clock_of(&world),
        orgs: views::orgs(&world, &viewer),
        former,
        recipes: views::recipes(&world),
        founding: views::founding(&world),
        slots: views::slots(&world),
    }))
}

#[utoipa::path(post, path = "/s/{id}/orgs", summary = "Found an organization (a firm costs the founding fee and 20 Materials)",
    params(("id" = i64, Path, description = "Society id")), request_body = FoundOrgRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn found_org(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<FoundOrgRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::FoundOrg {
        kind: req.kind,
        name: req.name,
        first_workplace: req.first_workplace.map(|f| (f.kind, f.slot.map(SlotId))),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(get, path = "/s/{id}/orgs/{oid}", summary = "One organization; managers see per-worker figures",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")),
    responses((status = 200, body = OrgView), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn get_org(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
) -> ApiResult<Json<OrgView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let mut org = views::org(&world, &viewer, OrgId(oid))
        .ok_or_else(|| ApiError::NotFound(format!("no org {oid}")))?;
    // D7: the flag alone says nothing; its manager and owners get the last missed payday.
    if org.payment_missed && (org.i_manage || org.my_shares > 0 || org.members.contains(&me.0)) {
        org.last_payment_missed = state
            .store
            .read_last_of_kind(id, "PaymentMissed", 500)
            .await?
            .into_iter()
            .find_map(|e| match &e.event {
                Event::PaymentMissed {
                    citizen,
                    org: o,
                    owed,
                    paid,
                    ..
                } if o.0 == oid => Some(PaymentMissedView {
                    seq: e.seq,
                    epoch: e.meta.epoch,
                    cycle: e.meta.cycle,
                    citizen: citizen.0,
                    handle: world
                        .citizens
                        .get(citizen)
                        .map(|c| c.handle.clone())
                        .unwrap_or_default(),
                    owed: cents(*owed),
                    paid: cents(*paid),
                }),
                _ => None,
            });
    }
    Ok(Json(org))
}

/// Does the payload name this org anywhere (`{"org": n}` as a party or as a field)?
fn mentions_org(v: &serde_json::Value, oid: u32) -> bool {
    match v {
        serde_json::Value::Object(m) => {
            if m.get("org").and_then(serde_json::Value::as_u64) == Some(u64::from(oid)) {
                return true;
            }
            m.values().any(|x| mentions_org(x, oid))
        }
        serde_json::Value::Array(a) => a.iter().any(|x| mentions_org(x, oid)),
        _ => false,
    }
}

/// The event kinds that move an org's treasury or its escrow, or turn its
/// Materials into a dwelling (D6).
const LEDGER_KINDS: [&str; 8] = [
    "Trade",
    "SaleAccepted",
    "Transferred",
    "Paid",
    "PaymentMissed",
    "DividendPaid",
    "OrderPlaced",
    "DwellingBuilt",
];

#[utoipa::path(get, path = "/s/{id}/orgs/{oid}/ledger", summary = "Managers and owners: what moved the treasury, oldest first",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")),
    responses((status = 200, body = OrgLedgerView), (status = 403, body = Problem), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn org_ledger(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
) -> ApiResult<Json<OrgLedgerView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let mut stored = Vec::new();
    for kind in LEDGER_KINDS {
        stored.extend(state.store.read_last_of_kind(id, kind, 500).await?);
    }
    stored.sort_by_key(|e| e.seq);
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let org = views::org(&world, &viewer, OrgId(oid))
        .ok_or_else(|| ApiError::NotFound(format!("no org {oid}")))?;
    if !org.i_manage && org.my_shares == 0 && !org.members.contains(&me.0) {
        return Err(ApiError::Forbidden(
            "the ledger is for the org's manager, owners and members".into(),
        ));
    }
    let entries = stored
        .iter()
        .filter(|e| serde_json::to_value(&e.event).is_ok_and(|v| mentions_org(&v, oid)))
        .filter_map(|e| event_ref(&viewer, &world, e))
        .collect();
    Ok(Json(OrgLedgerView {
        clock: clock_of(&world),
        entries,
    }))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/offers", summary = "Manager: post a job offer for one of the org's workplaces",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = EmploymentOfferRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn offer_employment(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<EmploymentOfferRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::OfferEmployment {
        org: OrgId(oid),
        workplace: WorkplaceId(req.workplace),
        pay: req.pay,
        max_hours: req.max_hours,
        term_cycles: req.term_cycles,
        notice_cycles: req.notice_cycles,
        places: req.places,
    };
    Ok(Json(
        send(&state, &entry, &auth, me, Some(OrgId(oid)), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/workplaces", summary = "Manager: add a workplace (20 Materials from the org)",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = AddWorkplaceRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn add_workplace(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<AddWorkplaceRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::AddWorkplace {
        org: OrgId(oid),
        kind: req.kind,
        slot: req.slot.map(SlotId),
    };
    Ok(Json(
        send(&state, &entry, &auth, me, Some(OrgId(oid)), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/machines", summary = "Manager: install or uninstall machines at a workplace",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = MachinesRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn machines(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<MachinesRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = match req.action.as_str() {
        "install" => Command::InstallMachines {
            org: OrgId(oid),
            workplace: WorkplaceId(req.workplace),
            qty: req.qty,
        },
        "uninstall" => Command::UninstallMachines {
            org: OrgId(oid),
            workplace: WorkplaceId(req.workplace),
            qty: req.qty,
        },
        other => {
            return Err(ApiError::BadRequest(format!(
                "action must be install or uninstall, not {other}"
            )));
        }
    };
    Ok(Json(
        send(&state, &entry, &auth, me, Some(OrgId(oid)), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/dividend", summary = "Controlling owner: declare a per-share dividend for this cycle",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = DividendRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn dividend(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<DividendRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::DeclareDividend {
        org: OrgId(oid),
        per_share: Money(req.per_share),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/shares", summary = "Controlling owner: issue new shares to the org's own registry",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = IssueSharesRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn issue_shares(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<IssueSharesRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::IssueShares {
        org: OrgId(oid),
        qty: req.qty,
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/manager", summary = "Controlling owner: appoint (or vacate) the manager",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = AppointRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn appoint(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<AppointRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::AppointManager {
        org: OrgId(oid),
        citizen: req.citizen.map(CitizenId),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/members", summary = "Manager: admit a citizen who asked to join",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = MemberRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn admit(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<MemberRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::AdmitMember {
        org: OrgId(oid),
        citizen: CitizenId(req.citizen),
    };
    Ok(Json(
        send(&state, &entry, &auth, me, Some(OrgId(oid)), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/join", summary = "Ask to join a member organization",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn request_membership(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::RequestMembership { org: OrgId(oid) };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/leave", summary = "Leave a member organization",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn leave_org(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::LeaveOrg { org: OrgId(oid) };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

// -- notice board, offers, contracts, transfers --------------------------------

#[utoipa::path(get, path = "/s/{id}/notice-board", summary = "Every live offer: jobs, sales, wanted, credit, leases",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = NoticeBoardView)), security(("session" = []), ("api_key" = [])))]
async fn notice_board(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<NoticeBoardView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    Ok(Json(NoticeBoardView {
        clock: clock_of(&world),
        offers: views::offers(&world),
    }))
}

#[utoipa::path(post, path = "/s/{id}/offers/sale", summary = "Offer goods, shares, or a dwelling for sale (escrowed)",
    params(("id" = i64, Path, description = "Society id")), request_body = SaleOfferRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn offer_sale(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<SaleOfferRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::OfferSale {
        asset: req.asset,
        price: req.price,
        to: req.to,
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/offers/wanted", summary = "Post a wanted ad", params(("id" = i64, Path, description = "Society id")),
    request_body = WantedRequest, responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn post_wanted(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<WantedRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::PostWanted {
        good: req.good,
        qty: req.qty,
        max_price: Money(req.max_price),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/offers/credit", summary = "Offer a loan (the principal is escrowed)",
    params(("id" = i64, Path, description = "Society id")), request_body = CreditOfferRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn offer_credit(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<CreditOfferRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::OfferCredit {
        to: req.to,
        principal: Money(req.principal),
        rate_per_cycle_bp: req.rate_per_cycle_bp,
        term_cycles: req.term_cycles,
        collateral: req.collateral,
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/offers/lease", summary = "Offer a dwelling to let",
    params(("id" = i64, Path, description = "Society id")), request_body = LeaseOfferRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn offer_lease(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<LeaseOfferRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::OfferLease {
        asset: req.asset,
        rent_per_cycle: Money(req.rent_per_cycle),
        term_cycles: req.term_cycles,
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/offers/{oid}/accept", summary = "Accept an offer of any kind",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Offer id")), request_body = AcceptRequest,
    responses((status = 200, body = Committed), (status = 404, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn accept_offer(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<AcceptRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let offer = OfferId(oid);
    let body = entry
        .handle
        .world
        .read()
        .await
        .offers
        .get(&offer)
        .map(|o| o.body.clone());
    let cmd = match body {
        Some(OfferBody::Employment { .. }) => Command::AcceptEmployment { offer },
        Some(OfferBody::Sale { .. }) => Command::AcceptSale { offer },
        Some(OfferBody::Credit { .. }) => Command::AcceptCredit { offer },
        Some(OfferBody::Lease { .. }) => Command::AcceptLease { offer },
        Some(OfferBody::Wanted { .. }) => {
            return Err(ApiError::BadRequest(
                "a wanted ad is answered with a sale offer to its poster".into(),
            ));
        }
        #[allow(unreachable_patterns)]
        Some(_) => {
            return Err(ApiError::BadRequest(
                "this offer kind is not accepted here".into(),
            ));
        }
        None => return Err(ApiError::NotFound(format!("no offer {oid}"))),
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(delete, path = "/s/{id}/offers/{oid}", summary = "Withdraw your sale offer or wanted ad",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Offer id"), OnBehalfQuery),
    responses((status = 200, body = Committed), (status = 404, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn withdraw_offer(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Query(q): Query<OnBehalfQuery>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let offer = OfferId(oid);
    let body = entry
        .handle
        .world
        .read()
        .await
        .offers
        .get(&offer)
        .map(|o| o.body.clone());
    let cmd = match body {
        Some(OfferBody::Sale { .. }) => Command::CancelSale { offer },
        Some(OfferBody::Wanted { .. }) => Command::RemoveWanted { offer },
        Some(_) => {
            return Err(ApiError::BadRequest(
                "only sale offers and wanted ads can be withdrawn".into(),
            ));
        }
        None => return Err(ApiError::NotFound(format!("no offer {oid}"))),
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(q.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(get, path = "/s/{id}/contracts", summary = "Your contracts and those of orgs you manage",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = ContractsView)), security(("session" = []), ("api_key" = [])))]
async fn contracts(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<ContractsView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    Ok(Json(ContractsView {
        clock: clock_of(&world),
        contracts: views::contracts(&world, &viewer),
    }))
}

#[utoipa::path(post, path = "/s/{id}/contracts/{cid}/terminate", summary = "End an employment contract or a lease (notice rules apply)",
    params(("id" = i64, Path, description = "Society id"), ("cid" = u32, Path, description = "Contract id")), request_body = AcceptRequest,
    responses((status = 200, body = Committed), (status = 404, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn terminate(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, cid)): Path<(i64, u32)>,
    Json(req): Json<AcceptRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let contract = ContractId(cid);
    let kind = entry
        .handle
        .world
        .read()
        .await
        .contracts
        .get(&contract)
        .map(|k| serde_json::to_value(&k.body).unwrap_or_default());
    let cmd = match kind {
        Some(v) if v.get("employment").is_some() => Command::TerminateEmployment { contract },
        Some(v) if v.get("lease").is_some() => Command::EndLease { contract },
        Some(_) => return Err(ApiError::BadRequest("this contract runs to term".into())),
        None => return Err(ApiError::NotFound(format!("no contract {cid}"))),
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/transfers", summary = "Give money or goods to a citizen or an org",
    params(("id" = i64, Path, description = "Society id")), request_body = TransferRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn transfer(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<TransferRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::Transfer {
        to: req.to,
        asset: req.asset,
        memo: req.memo,
    };
    Ok(Json(
        send(&state, &entry, &auth, me, org_of(req.on_behalf_of), cmd).await?,
    ))
}

#[utoipa::path(post, path = "/s/{id}/dwellings/{did}/move-in", summary = "Occupy a dwelling you own",
    params(("id" = i64, Path, description = "Society id"), ("did" = u32, Path, description = "Dwelling id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn move_in(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, did)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::MoveIn {
        dwelling: DwellingId(did),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/dwellings/{did}/move-out", summary = "Leave a dwelling you own",
    params(("id" = i64, Path, description = "Society id"), ("did" = u32, Path, description = "Dwelling id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn move_out(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, did)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::MoveOut {
        dwelling: DwellingId(did),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

// -- society -------------------------------------------------------------------

#[utoipa::path(get, path = "/s/{id}/stats", summary = "System-appropriate live metrics and the last cycle's aggregates",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = StatsView)), security(("session" = []), ("api_key" = [])))]
async fn stats(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<StatsView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    Ok(Json(stats_of(&state, &entry, id).await?))
}

/// The numbers a society keeps; shared with the spectator route (S1.13).
pub(crate) async fn stats_of(
    state: &AppState,
    entry: &SocietyEntry,
    id: i64,
) -> ApiResult<StatsView> {
    let last = state.store.read_last_of_kind(id, "CycleClosed", 1).await?;
    let last_cycle = last
        .first()
        .and_then(|e| serde_json::to_value(&e.event).ok())
        .and_then(|v| {
            v.get("CycleClosed")
                .and_then(|c| c.get("aggregates"))
                .cloned()
        });
    let world = entry.handle.world.read().await;
    Ok(StatsView {
        clock: clock_of(&world),
        last_cycle,
        live: views::pulse(&world),
        firm_count: views::firm_count(&world),
        credit_outstanding: views::credit_outstanding(&world),
    })
}

#[utoipa::path(get, path = "/s/{id}/citizens", summary = "Public profiles and flags of every citizen",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = CitizensView)), security(("session" = []), ("api_key" = [])))]
async fn citizens(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<CitizensView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    Ok(Json(CitizensView {
        clock: clock_of(&world),
        citizens: views::citizens_public(&world),
    }))
}

#[utoipa::path(get, path = "/s/{id}/scoreboard", summary = "The scoreboard this society keeps (Freeport: net worth, firm valuation, self-made)",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = ScoreboardView)), security(("session" = []), ("api_key" = [])))]
async fn scoreboard(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<ScoreboardView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    Ok(Json(ScoreboardView {
        clock: clock_of(&world),
        rows: views::scoreboard(&world),
    }))
}

#[utoipa::path(get, path = "/s/{id}/householders", summary = "The published householder script (transparency)",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = HouseholdersView)), security(("session" = []), ("api_key" = [])))]
async fn householders(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<HouseholdersView>> {
    let (entry, _me) = me_in(&state, &auth, id).await?;
    let path = state.presets_dir.join("..").join("docs").join("SCRIPT.md");
    let markdown = std::fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(format!("SCRIPT.md at {}: {e}", path.display())))?;
    let world = entry.handle.world.read().await;
    Ok(Json(HouseholdersView {
        clock: clock_of(&world),
        markdown,
    }))
}

#[utoipa::path(get, path = "/s/{id}/explain/{seq}", summary = "One event as you may see it, with every Explain inside it",
    params(("id" = i64, Path, description = "Society id"), ("seq" = i64, Path, description = "Event log position")),
    responses((status = 200, body = ExplainView), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn explain(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, seq)): Path<(i64, i64)>,
) -> ApiResult<Json<ExplainView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let stored = state
        .store
        .read_one(id, seq)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("no event {seq}")))?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let event = event_ref(&viewer, &world, &stored)
        .ok_or_else(|| ApiError::NotFound(format!("no event {seq}")))?;
    let explains = explains_in(&event.payload);
    Ok(Json(ExplainView {
        clock: clock_of(&world),
        event,
        explains,
    }))
}

/// All in-society routes, to merge into the app router.
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(home))
        .routes(routes!(get_plan, set_plan))
        .routes(routes!(set_labor))
        .routes(routes!(payslips))
        .routes(routes!(digest))
        .routes(routes!(books))
        .routes(routes!(book))
        .routes(routes!(place_order))
        .routes(routes!(cancel_order))
        .routes(routes!(prices))
        .routes(routes!(orgs, found_org))
        .routes(routes!(get_org))
        .routes(routes!(org_ledger))
        .routes(routes!(offer_employment))
        .routes(routes!(add_workplace))
        .routes(routes!(machines))
        .routes(routes!(dividend))
        .routes(routes!(issue_shares))
        .routes(routes!(appoint))
        .routes(routes!(admit))
        .routes(routes!(request_membership))
        .routes(routes!(leave_org))
        .routes(routes!(notice_board))
        .routes(routes!(offer_sale))
        .routes(routes!(post_wanted))
        .routes(routes!(offer_credit))
        .routes(routes!(offer_lease))
        .routes(routes!(accept_offer))
        .routes(routes!(withdraw_offer))
        .routes(routes!(contracts))
        .routes(routes!(terminate))
        .routes(routes!(transfer))
        .routes(routes!(move_in))
        .routes(routes!(move_out))
        .routes(routes!(stats))
        .routes(routes!(citizens))
        .routes(routes!(scoreboard))
        .routes(routes!(householders))
        .routes(routes!(explain))
        .routes(routes!(crate::stream::stream))
}

#[allow(dead_code)]
fn _unused(_: StatusCode) {}
