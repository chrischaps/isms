//! The Coordinator workspace on the wire (GDD 6.2 Governance, 8.2; TDD
//! 10.3; S2.8): the coordinator's three powers as routes, and the Plan as
//! published for everyone to read. `PUT /s/{id}/offices/coordinator/plan`
//! publishes targets (`SetPlan`, targets only under a direct assembly);
//! `POST /s/{id}/workplaces` and `DELETE /s/{id}/workplaces/{wid}` open and
//! close a workplace of the collective (`OpenWorkplace` / `CloseWorkplace`,
//! Q120); `GET /s/{id}/plan/published` is every workplace with its target
//! beside what it made, the land slots, and what a workplace costs against
//! what the Store holds (Q143). A non-holder's command is the engine's own
//! `NotAnOfficeHolder`, named on the way out like every refusal.

use crate::auth::Auth;
use crate::error::{ApiError, ApiResult};
use crate::society_api::{me_in, send};
use crate::state::{AppState, clock_of};
use axum::Json;
use axum::extract::{Path, State};
use isms_api_types::Problem;
use isms_api_types::society::{
    Committed, OpenWorkplaceRequest, PlanTargetView, PublishPlanRequest, PublishedPlanView,
    SlotView,
};
use isms_core::command::{Command, Reject, RejectCode};
use isms_core::constitution::{Governance, OfficeKind};
use isms_core::event::{Actor, Event};
use isms_core::ids::{CitizenId, SlotId, WorkplaceId};
use isms_core::kinds::{Good, OrgKind, WorkplaceKind};
use isms_core::rules::Rules;
use isms_core::world::World;
use std::collections::BTreeMap;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// Every workplace as the Plan sees it, in id order.
#[must_use]
pub fn plan_targets(world: &World) -> Vec<PlanTargetView> {
    world
        .workplaces
        .values()
        .map(|wp| {
            let org = world.orgs.get(&wp.org);
            PlanTargetView {
                workplace: wp.id.0,
                org: wp.org.0,
                org_name: org.map(|o| o.name.clone()).unwrap_or_default(),
                collective: org.is_some_and(|o| o.kind == OrgKind::Collective),
                kind: wp.kind,
                slot: wp.slot.map(|s| s.0),
                workers: u32::try_from(wp.workers.len()).unwrap_or(u32::MAX),
                machines: wp.machines,
                target: wp.target,
                cycle_output: wp.cycle_output,
                last_cycle_output: wp.last_cycle_output,
                last_fulfillment: wp.last_fulfillment,
            }
        })
        .collect()
}

/// The land, slot by slot, and the kinds it does not limit.
#[must_use]
pub fn land(world: &World) -> (Vec<SlotView>, Vec<WorkplaceKind>) {
    let slots = world
        .land
        .slots
        .iter()
        .map(|(id, s)| SlotView {
            id: id.0,
            kind: s.kind,
            workplace: s.workplace.map(|w| w.0),
        })
        .collect();
    let unlimited = WorkplaceKind::ALL
        .into_iter()
        .filter(|k| !world.land.is_limited(*k))
        .collect();
    (slots, unlimited)
}

/// Materials on the Common Store's shelf, the founding stock a coordinator draws on.
fn store_materials(world: &World) -> u32 {
    world
        .store
        .as_ref()
        .and_then(|s| s.stock.get(&Good::Materials).copied())
        .unwrap_or(0)
}

/// Where a Plan exists: a direct assembly's advisory one, or the Committee's.
/// `Ok(true)` when it is advisory.
fn has_a_plan(world: &World) -> Result<bool, ApiError> {
    let rules = Rules::from_world(world);
    let advisory = world.constitution.governance == Governance::Direct;
    if !advisory && !rules.capabilities.administered_prices {
        return Err(ApiError::Reject(Reject::new(
            RejectCode::NotInThisSociety,
            "nobody publishes a Plan in this society",
        )));
    }
    Ok(advisory)
}

#[utoipa::path(get, path = "/s/{id}/plan/published", summary = "The Plan as published: every workplace's target beside what it made, the land slots, and what a workplace costs against the Store",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = PublishedPlanView), (status = 422, body = Problem, description = "No Plan is published in this society")),
    security(("session" = []), ("api_key" = [])))]
async fn published_plan(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<PublishedPlanView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let epoch = {
        let world = entry.handle.world.read().await;
        has_a_plan(&world)?;
        world.meta.epoch
    };
    // The last publication this epoch, from the log: the world keeps the
    // targets, not who published them or when.
    let last = state
        .store
        .read_last_of_kind(id, "PlanPublished", 1)
        .await?
        .into_iter()
        .find(|e| e.meta.epoch == epoch);
    let world = entry.handle.world.read().await;
    let advisory = has_a_plan(&world)?;
    let (slots, unlimited_kinds) = land(&world);
    let published_by = last.as_ref().and_then(|e| match &e.event {
        Event::PlanPublished {
            by: Actor::Citizen(c),
            ..
        } => Some(c.0),
        _ => None,
    });
    Ok(Json(PublishedPlanView {
        clock: clock_of(&world),
        i_coordinate: coordinates(&world, me),
        advisory,
        published_cycle: last.as_ref().map(|e| e.meta.cycle),
        published_tick: last.as_ref().map(|e| e.meta.tick),
        published_by,
        targets: plan_targets(&world),
        slots,
        unlimited_kinds,
        collective: world
            .orgs
            .values()
            .find(|o| o.kind == OrgKind::Collective)
            .map(|o| o.id.0),
        founding_materials: world.params.founding.materials,
        store_materials: store_materials(&world),
        max_workplaces: u32::from(world.params.labor.max_workplaces),
    }))
}

#[utoipa::path(put, path = "/s/{id}/offices/coordinator/plan", summary = "Coordinator: publish the Plan (targets per workplace, units a day; advisory under a direct assembly)",
    params(("id" = i64, Path, description = "Society id")), request_body = PublishPlanRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem, description = "Not a coordinator, an unknown workplace, or a target below zero")),
    security(("session" = []), ("api_key" = [])))]
async fn publish_plan(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<PublishPlanRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let targets: BTreeMap<WorkplaceId, f64> = req
        .targets
        .into_iter()
        .map(|(w, t)| (WorkplaceId(w), t))
        .collect();
    let cmd = Command::SetPlan {
        targets,
        materials_split: None,
        price_list: None,
        wage_grades: None,
        ration_caps: None,
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(post, path = "/s/{id}/workplaces", summary = "Coordinator: open a workplace of the collective on a land slot; the founding Materials come out of the Common Store",
    params(("id" = i64, Path, description = "Society id")), request_body = OpenWorkplaceRequest,
    responses((status = 200, body = Committed), (status = 422, body = Problem, description = "Not a coordinator, no free slot, or too few Materials")),
    security(("session" = []), ("api_key" = [])))]
async fn open_workplace(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    Json(req): Json<OpenWorkplaceRequest>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::OpenWorkplace {
        kind: req.kind,
        slot: req.slot.map(SlotId),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(delete, path = "/s/{id}/workplaces/{wid}", summary = "Coordinator: close a workplace of the collective; its workers are unassigned this hour and its slot freed",
    params(("id" = i64, Path, description = "Society id"), ("wid" = u32, Path, description = "Workplace id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem, description = "Not a coordinator, or not the collective's workplace")),
    security(("session" = []), ("api_key" = [])))]
async fn close_workplace(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, wid)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::CloseWorkplace {
        workplace: WorkplaceId(wid),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

/// Whether `citizen` sits as Coordinator: the holder check the workspace mounts on.
#[must_use]
pub fn coordinates(world: &World, citizen: CitizenId) -> bool {
    world.offices.holds(citizen, OfficeKind::Coordinator)
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(published_plan))
        .routes(routes!(publish_plan))
        .routes(routes!(open_workplace))
        .routes(routes!(close_workplace))
}
