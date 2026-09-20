//! The assembly on the wire (TDD 10.3, S2.5): proposals and ballots, offices
//! and elections, and a member-owned org's disbursement vote. Thin adapters
//! from wire requests to the engine's `Propose`, `Vote`, `Stand`, `Withdraw`
//! and `Approve`, and from `World` (open) and the log (closed) to views. The
//! floor threads (`assembly:<pid>`) live in `comms.rs`; what a citizen may see
//! of a proposal is decided here once (`may_see`) and read there too.

use crate::actor::envelope;
use crate::auth::Auth;
use crate::error::{ApiError, ApiResult};
use crate::names::Directory;
use crate::society_api::{READ_CAP, event_ref, me_in, send};
use crate::state::{AppState, clock_of};
use crate::viewer::Viewer;
use axum::Json;
use axum::extract::{Path, State};
use isms_api_types::Problem;
use isms_api_types::society::{
    ApproveRequest, BallotRequest, BallotView, CandidateView, Committed, DisbursementRequest,
    ElectionView, HolderView, OfficeView, OfficesView, ProposalOutcome, ProposalView,
    ProposalsView, ProposeRequest, TallyView,
};
use isms_core::command::Command;
use isms_core::constitution::OfficeKind;
use isms_core::event::{Actor, Event};
use isms_core::governance::{assembly_tally, electorate};
use isms_core::ids::{CitizenId, OrgId, ProposalId};
use isms_core::kinds::CitizenKind;
use isms_core::ledger::Party;
use isms_core::offices;
use isms_core::world::{Proposal, ProposalKind, Tally, World};
use isms_store::StoredEvent;
use std::collections::BTreeMap;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

// -- what the log remembers of a proposal ----------------------------------------

/// A proposal as its `Proposed` event recorded it: enough to show a closed
/// one and to know when its floor closes.
#[derive(Clone, Debug)]
pub struct Opened {
    pub by: CitizenId,
    pub title: String,
    pub text: String,
    pub kind: ProposalKind,
    pub closes_cycle: u32,
    pub tick: u32,
}

/// Every proposal opened this epoch, by id, from the log.
pub async fn proposed_this_epoch(
    state: &AppState,
    id: i64,
    epoch: u32,
) -> ApiResult<BTreeMap<ProposalId, Opened>> {
    let stored = state
        .store
        .read_kind_since_tick(id, epoch, "Proposed", 0, READ_CAP)
        .await?;
    Ok(stored
        .iter()
        .filter_map(|e| match &e.event {
            Event::Proposed {
                proposal,
                by,
                title,
                text,
                kind,
                closes_cycle,
            } => Some((
                *proposal,
                Opened {
                    by: *by,
                    title: title.clone(),
                    text: text.clone(),
                    kind: *kind,
                    closes_cycle: *closes_cycle,
                    tick: e.meta.tick,
                },
            )),
            _ => None,
        })
        .collect())
}

/// The org whose members vote on this kind; `None` for the assembly's own.
#[must_use]
pub fn org_of_kind(kind: &ProposalKind) -> Option<OrgId> {
    match kind {
        ProposalKind::Admission { org, .. } | ProposalKind::Disbursement { org, .. } => Some(*org),
        _ => None,
    }
}

/// Whether `me` may read a proposal of this kind: the assembly's are every
/// citizen's; a members' vote is the org's members' and its manager's.
#[must_use]
pub fn may_see(world: &World, me: CitizenId, kind: &ProposalKind) -> bool {
    match org_of_kind(kind) {
        None => true,
        Some(org) => world
            .orgs
            .get(&org)
            .is_some_and(|o| o.members.contains(&me) || o.manager == Some(me)),
    }
}

/// Whether the floor of a proposal takes posts: while it is open and for one
/// cycle after its closing cycle (TDD 12).
#[must_use]
pub fn floor_open(world: &World, open: bool, closes_cycle: u32) -> bool {
    open || world.cycle_of(world.meta.tick) <= closes_cycle + 1
}

// -- the away digest ---------------------------------------------------------------

/// Whether a governance event is about `me` (the engine's `touches` stops
/// at the economy): a ballot cast for them at the close, a proposal they
/// could see closing, an honor, a seat taken or lost, a disbursement to them,
/// and an election opening that they could stand in.
#[must_use]
pub fn concerns(
    world: &World,
    me: CitizenId,
    event: &Event,
    opened: &BTreeMap<ProposalId, Opened>,
) -> bool {
    match event {
        Event::Voted {
            citizen,
            by_default,
            ..
        } => *citizen == me && *by_default,
        Event::ProposalClosed { proposal, .. } => opened
            .get(proposal)
            .is_some_and(|o| may_see(world, me, &o.kind)),
        Event::Honored { citizen, .. }
        | Event::OfficeTaken { citizen, .. }
        | Event::OfficeVacated { citizen, .. } => *citizen == me,
        Event::Disbursed { to, .. } => *to == Party::Citizen(me),
        Event::ElectionOpened { .. } => world
            .citizens
            .get(&me)
            .is_some_and(|c| c.kind == CitizenKind::Human),
        _ => false,
    }
}

/// The indices of `events` that belong in `me`'s away digest: what the plan
/// says touches them, plus the assembly's news (`concerns`).
#[must_use]
pub fn digest_indices(
    world: &World,
    me: CitizenId,
    events: &[Event],
    opened: &BTreeMap<ProposalId, Opened>,
) -> Vec<usize> {
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| isms_core::plan::touches(e, me) || concerns(world, me, e, opened))
        .map(|(i, _)| i)
        .collect()
}

// -- views -------------------------------------------------------------------------

fn tally_view(t: &Tally) -> TallyView {
    TallyView {
        yes: t.yes,
        no: t.no,
        abstain: t.abstain,
        cast: t.cast,
        quorum: t.quorum,
        eligible: t.eligible,
    }
}

fn name_of<T: serde::Serialize>(v: T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn handle_of(world: &World, c: CitizenId) -> String {
    world
        .citizens
        .get(&c)
        .map(|z| z.handle.clone())
        .unwrap_or_default()
}

/// The count on an open proposal right now: the assembly's against the
/// active humans (Q117), a members' vote against the membership (Q122).
fn tally_now(world: &World, p: &Proposal) -> Tally {
    match org_of_kind(&p.kind).and_then(|o| world.orgs.get(&o)) {
        Some(o) => isms_core::bank::admission_tally(&p.ballots, o),
        None => assembly_tally(
            &p.ballots,
            u32::try_from(electorate(world).len()).unwrap_or(u32::MAX),
            world.params.population.quorum_fraction,
        ),
    }
}

/// An open proposal, from the world.
fn open_view(world: &World, me: CitizenId, p: &Proposal) -> ProposalView {
    let mut ballots: Vec<BallotView> = p
        .ballots
        .iter()
        .map(|(c, b)| BallotView {
            citizen: c.0,
            handle: handle_of(world, *c),
            ballot: name_of(b),
        })
        .collect();
    ballots.sort_by_key(|b| b.citizen);
    ProposalView {
        id: p.id.0,
        by: p.by.0,
        by_handle: handle_of(world, p.by),
        title: p.title.clone(),
        text: p.text.clone(),
        kind: serde_json::to_value(p.kind).unwrap_or_default(),
        kind_tag: name_of(p.kind.tag()),
        org: org_of_kind(&p.kind).map(|o| o.0),
        opened_tick: p.opened_tick,
        closes_cycle: p.closes_cycle,
        open: true,
        tally: tally_view(&tally_now(world, p)),
        ballots,
        my_ballot: p.ballots.get(&me).map(name_of),
        floor_open: true,
        outcome: None,
    }
}

/// A closed proposal, from its `Proposed` and `ProposalClosed` events and
/// the effects that name it.
fn closed_view(
    world: &World,
    viewer: &Viewer,
    id: ProposalId,
    o: &Opened,
    closed: &StoredEvent,
    effects: &[StoredEvent],
) -> Option<ProposalView> {
    let Event::ProposalClosed { passed, tally, .. } = &closed.event else {
        return None;
    };
    let effects = effects
        .iter()
        .filter(|e| match &e.event {
            Event::PolicyChanged { proposal, .. } => *proposal == Some(id),
            Event::Honored { proposal, .. } | Event::Disbursed { proposal, .. } => *proposal == id,
            _ => false,
        })
        .filter_map(|e| event_ref(viewer, world, e))
        .collect();
    Some(ProposalView {
        id: id.0,
        by: o.by.0,
        by_handle: handle_of(world, o.by),
        title: o.title.clone(),
        text: o.text.clone(),
        kind: serde_json::to_value(o.kind).unwrap_or_default(),
        kind_tag: name_of(o.kind.tag()),
        org: org_of_kind(&o.kind).map(|x| x.0),
        opened_tick: o.tick,
        closes_cycle: o.closes_cycle,
        open: false,
        tally: tally_view(tally),
        ballots: Vec::new(),
        my_ballot: None,
        floor_open: floor_open(world, false, o.closes_cycle),
        outcome: Some(ProposalOutcome {
            passed: *passed,
            closed_seq: closed.seq,
            closed_cycle: closed.meta.cycle,
            tally: tally_view(tally),
            effects,
        }),
    })
}

/// Every proposal closed this epoch that `me` may see, oldest first.
async fn closed_this_epoch(
    state: &AppState,
    id: i64,
    epoch: u32,
    world: &World,
    me: CitizenId,
    opened: &BTreeMap<ProposalId, Opened>,
) -> ApiResult<Vec<ProposalView>> {
    let closes = state
        .store
        .read_kind_since_tick(id, epoch, "ProposalClosed", 0, READ_CAP)
        .await?;
    let mut effects = Vec::new();
    for kind in ["PolicyChanged", "Honored", "Disbursed"] {
        effects.extend(
            state
                .store
                .read_kind_since_tick(id, epoch, kind, 0, READ_CAP)
                .await?,
        );
    }
    effects.sort_by_key(|e| e.seq);
    let viewer = Viewer::new(world, Some(me));
    Ok(closes
        .iter()
        .filter_map(|e| {
            let Event::ProposalClosed { proposal, .. } = &e.event else {
                return None;
            };
            let o = opened.get(proposal)?;
            if !may_see(world, me, &o.kind) {
                return None;
            }
            closed_view(world, &viewer, *proposal, o, e, &effects)
        })
        .collect())
}

fn office_kind(s: &str) -> ApiResult<OfficeKind> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|_| ApiError::BadRequest(format!("no office called {s}")))
}

/// Every office on the constitution, in its order, with its holders and any
/// open election, as `me` sees them.
fn offices_view(world: &World, me: CitizenId, auth: &Auth) -> Vec<OfficeView> {
    let dir = Directory::from_world(world, Some(me));
    world
        .constitution
        .offices
        .iter()
        .map(|spec| {
            let holders = world
                .offices
                .holders
                .get(&spec.kind)
                .map(|h| {
                    h.iter()
                        .map(|h| HolderView {
                            citizen: h.citizen.0,
                            handle: handle_of(world, h.citizen),
                            term_ends_cycle: h.term_ends_cycle,
                        })
                        .collect()
                })
                .unwrap_or_default();
            let election = world.offices.elections.get(&spec.kind).map(|e| {
                let i_stand = e.candidates.contains(&me);
                // The engine is the eligibility oracle (Q134): a dry run of
                // `Stand` says whether and why not, in its own words.
                let stand_refusal = if i_stand {
                    None
                } else {
                    let mut env = envelope(
                        Actor::Citizen(me),
                        auth.client_kind(),
                        Command::Stand { office: spec.kind },
                    );
                    env.received_at_tick = world.meta.tick;
                    offices::stand(world, &env, spec.kind)
                        .err()
                        .map(|r| dir.reject(&r).message)
                };
                ElectionView {
                    seats: e.seats,
                    opened_cycle: e.opened_cycle,
                    closes_cycle: e.closes_cycle,
                    candidates: offices::ranked(world, e)
                        .into_iter()
                        .map(|(c, approvals)| CandidateView {
                            citizen: c.0,
                            handle: handle_of(world, c),
                            approvals,
                        })
                        .collect(),
                    ballots_cast: u32::try_from(e.approvals.len()).unwrap_or(u32::MAX),
                    my_approvals: e
                        .approvals
                        .get(&me)
                        .map(|set| set.iter().map(|c| c.0).collect())
                        .unwrap_or_default(),
                    i_stand,
                    stand_refusal,
                }
            });
            OfficeView {
                kind: name_of(spec.kind),
                seats: spec.seats,
                term_cycles: spec.term_cycles,
                consecutive: spec.consecutive,
                recall: name_of(spec.recall),
                holders,
                i_hold: world.offices.holds(me, spec.kind),
                election,
                short_since: world.offices.short_since.get(&spec.kind).copied(),
            }
        })
        .collect()
}

// -- routes --------------------------------------------------------------------------

#[utoipa::path(get, path = "/s/{id}/proposals", summary = "The assembly: open proposals with their tallies and your ballot, and those closed this epoch with their outcomes",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = ProposalsView), (status = 403, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn proposals(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<ProposalsView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let epoch = entry.handle.world.read().await.meta.epoch;
    let opened = proposed_this_epoch(&state, id, epoch).await?;
    let world = entry.handle.world.read().await;
    let closed = closed_this_epoch(&state, id, epoch, &world, me, &opened).await?;
    let open = world
        .proposals
        .values()
        .filter(|p| may_see(&world, me, &p.kind))
        .map(|p| open_view(&world, me, p))
        .collect();
    Ok(Json(ProposalsView {
        clock: clock_of(&world),
        open,
        closed,
        electorate: u32::try_from(electorate(&world).len()).unwrap_or(u32::MAX),
        quorum_fraction: world.params.population.quorum_fraction,
        open_per_citizen: world.params.governance.open_proposals_per_citizen,
        my_vote_default: world
            .citizens
            .get(&me)
            .map(|c| serde_json::to_value(c.plan.vote_default).unwrap_or_default())
            .unwrap_or_default(),
    }))
}

#[utoipa::path(post, path = "/s/{id}/proposals", summary = "Open a proposal before the assembly; it closes at the end of this cycle",
    params(("id" = i64, Path, description = "Society id")), request_body = ProposeRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn propose(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<ProposeRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    if matches!(req.kind, ProposalKind::Disbursement { .. }) {
        return Err(ApiError::BadRequest(
            "a disbursement is moved at /orgs/{oid}/disbursements".into(),
        ));
    }
    let cmd = Command::Propose {
        title: req.title,
        text: req.text,
        kind: req.kind,
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(get, path = "/s/{id}/proposals/{pid}", summary = "One proposal, open or closed this epoch",
    params(("id" = i64, Path, description = "Society id"), ("pid" = u32, Path, description = "Proposal id")),
    responses((status = 200, body = ProposalView), (status = 404, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn proposal(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, pid)): Path<(i64, u32)>,
) -> ApiResult<Json<ProposalView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let wanted = ProposalId(pid);
    let epoch = {
        let world = entry.handle.world.read().await;
        if let Some(p) = world.proposals.get(&wanted) {
            if !may_see(&world, me, &p.kind) {
                return Err(ApiError::NotFound(format!("no proposal {pid}")));
            }
            return Ok(Json(open_view(&world, me, p)));
        }
        world.meta.epoch
    };
    let opened = proposed_this_epoch(&state, id, epoch).await?;
    let world = entry.handle.world.read().await;
    closed_this_epoch(&state, id, epoch, &world, me, &opened)
        .await?
        .into_iter()
        .find(|p| p.id == pid)
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("no proposal {pid}")))
}

#[utoipa::path(put, path = "/s/{id}/proposals/{pid}/ballot", summary = "Cast or replace your ballot on an open proposal (a members' vote included)",
    params(("id" = i64, Path, description = "Society id"), ("pid" = u32, Path, description = "Proposal id")), request_body = BallotRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn ballot(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, pid)): Path<(i64, u32)>,
    Json(req): Json<BallotRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::Vote {
        proposal: ProposalId(pid),
        ballot: req.ballot,
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(get, path = "/s/{id}/offices", summary = "Every office: its rule, who sits, and the election open for it",
    params(("id" = i64, Path, description = "Society id")), responses((status = 200, body = OfficesView), (status = 403, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn offices(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<OfficesView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    Ok(Json(OfficesView {
        clock: clock_of(&world),
        offices: offices_view(&world, me, &auth),
    }))
}

#[utoipa::path(post, path = "/s/{id}/offices/{kind}/candidacy", summary = "Stand in the open election for an office",
    params(("id" = i64, Path, description = "Society id"), ("kind" = String, Path, description = "coordinator, planning_committee, legislator, union_steward or bank_board")),
    responses((status = 200, body = Committed), (status = 400, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn stand(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, kind)): Path<(i64, String)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let office = office_kind(&kind)?;
    let cmd = Command::Stand { office };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(delete, path = "/s/{id}/offices/{kind}/candidacy", summary = "Withdraw your candidacy",
    params(("id" = i64, Path, description = "Society id"), ("kind" = String, Path, description = "The office")),
    responses((status = 200, body = Committed), (status = 400, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn withdraw(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, kind)): Path<(i64, String)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let office = office_kind(&kind)?;
    let cmd = Command::Withdraw { office };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(put, path = "/s/{id}/offices/{kind}/ballot", summary = "Cast or replace your approval ballot: any subset of the candidates",
    params(("id" = i64, Path, description = "Society id"), ("kind" = String, Path, description = "The office")), request_body = ApproveRequest,
    responses((status = 200, body = Committed), (status = 400, body = Problem), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn approve(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, kind)): Path<(i64, String)>,
    Json(req): Json<ApproveRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let office = office_kind(&kind)?;
    let cmd = Command::Approve {
        office,
        candidates: req.candidates.into_iter().map(CitizenId).collect(),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/orgs/{oid}/disbursements", summary = "Member: move the org's money or goods to someone, subject to the members' vote",
    params(("id" = i64, Path, description = "Society id"), ("oid" = u32, Path, description = "Org id")), request_body = DisbursementRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem)), security(("session" = []), ("api_key" = [])))]
async fn disburse(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, oid)): Path<(i64, u32)>,
    Json(req): Json<DisbursementRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::Propose {
        title: String::new(),
        text: req.text,
        kind: ProposalKind::Disbursement {
            org: OrgId(oid),
            to: req.to,
            asset: req.asset,
        },
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

/// The assembly's routes, to merge into the app router.
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(proposals, propose))
        .routes(routes!(proposal))
        .routes(routes!(ballot))
        .routes(routes!(offices))
        .routes(routes!(stand, withdraw))
        .routes(routes!(approve))
        .routes(routes!(disburse))
}
