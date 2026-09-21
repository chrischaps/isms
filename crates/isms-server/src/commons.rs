//! The Common Store and the Ledger of Contribution on the wire (GDD 6.2,
//! TDD 10.3; S2.7). Two reads and two commands: `GET /s/{id}/store` is the
//! shelves as they stand, the rule that decides when they run short, the
//! caller's entitlement and pending draw, yesterday's service from the log,
//! and the caller's draw record; `GET /s/{id}/ledger` is every citizen's
//! public record, hours exact and output as attributed under the monitoring
//! in force, with sigma stated. `POST`/`DELETE /s/{id}/workplaces/{wid}/position`
//! take and give up a norm position (`JoinWorkplace` / `LeaveWorkplace`,
//! Q62, Q141), which nothing else on the wire could do for a human.

use crate::auth::Auth;
use crate::error::{ApiError, ApiResult};
use crate::society_api::{READ_CAP, event_ref, me_in, send};
use crate::state::{AppState, clock_of};
use crate::viewer::Viewer;
use axum::Json;
use axum::extract::{Path, State};
use isms_api_types::Problem;
use isms_api_types::society::{
    Committed, ContributionRow, ContributionView, StockView, StoreDayView, StoreView,
};
use isms_core::command::{Command, Reject, RejectCode};
use isms_core::constitution::LaborMode;
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::ids::{CitizenId, WorkplaceId};
use isms_core::kinds::Good;
use isms_core::policy::Rationing;
use isms_core::rules::Rules;
use isms_core::world::World;
use isms_core::{norms, orgs, store};
use isms_store::StoredEvent;
use std::collections::BTreeMap;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// The draw record never exceeds this many events.
const PAGE: usize = 200;

// -- the Store -----------------------------------------------------------------

/// The goods the Store rations by need, first; then whatever else is on the shelf.
fn goods_on_the_shelf(world: &World) -> Vec<Good> {
    let mut goods = vec![Good::Food, Good::Wares];
    if let Some(s) = &world.store {
        for g in s.stock.keys() {
            if !goods.contains(g) {
                goods.push(*g);
            }
        }
    }
    goods
}

/// Meter points one unit restores, for the goods drawn by need (`store.rs`).
fn meter_per_unit(world: &World, good: Good) -> Option<u32> {
    match good {
        Good::Food => Some(u32::from(world.params.needs.food_meter_per_unit)),
        Good::Wares => Some(u32::from(world.params.needs.comfort_per_wares)),
        _ => None,
    }
}

/// The rationing rule in force, as `policy.rationing` spells it.
#[must_use]
pub fn rule_in_force(world: &World) -> String {
    let rule = world.policy.rationing.unwrap_or(Rationing::NeedFirst);
    serde_json::to_value(rule)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "need_first".to_owned())
}

#[derive(Default)]
struct DayTally {
    requested: u32,
    served: u32,
    shared: u32,
    shared_with: u32,
    /// Per tick: (requested, served), to count the hours the rule had to decide.
    ticks: BTreeMap<u32, (u32, u32)>,
}

/// What the log says the Store did, per (cycle, good), from `StoreDrawRequested`
/// and `Drew` events: a `Drew` under the surplus rule is a share, any other a draw.
#[must_use]
pub fn store_days(
    requested: &[StoredEvent],
    drew: &[StoredEvent],
    tpc: u32,
) -> BTreeMap<(u32, Good), StoreDayView> {
    let mut days: BTreeMap<(u32, Good), DayTally> = BTreeMap::new();
    for e in requested {
        if let Event::StoreDrawRequested { good, qty, .. } = &e.event {
            let cycle = e.meta.tick / tpc;
            let d = days.entry((cycle, *good)).or_default();
            d.requested += qty;
            d.ticks.entry(e.meta.tick).or_default().0 += qty;
        }
    }
    for e in drew {
        if let Event::Drew { goods, explain, .. } = &e.event {
            let cycle = e.meta.tick / tpc;
            for (good, qty) in goods {
                let d = days.entry((cycle, *good)).or_default();
                if explain.rule == RuleId::StoreSurplusShare {
                    d.shared += qty;
                    d.shared_with += 1;
                } else {
                    d.served += qty;
                    d.ticks.entry(e.meta.tick).or_default().1 += qty;
                }
            }
        }
    }
    days.into_iter()
        .map(|((cycle, good), d)| {
            let rationed_ticks =
                u32::try_from(d.ticks.values().filter(|(r, s)| s < r).count()).unwrap_or(u32::MAX);
            (
                (cycle, good),
                StoreDayView {
                    good,
                    requested: d.requested,
                    served: d.served,
                    short: d.requested.saturating_sub(d.served),
                    rationed_ticks,
                    shared: d.shared,
                    shared_with: d.shared_with,
                },
            )
        })
        .collect()
}

fn active_citizens(world: &World) -> u32 {
    u32::try_from(world.citizens.values().filter(|c| !c.dormant).count()).unwrap_or(u32::MAX)
}

/// The shelves as the caller sees them: stock, this tick's requests, their own
/// entitlement and pending draw, and the share each would get if the day ended now.
#[must_use]
pub fn shelves(world: &World, me: CitizenId) -> Vec<StockView> {
    let citizen = world.citizens.get(&me);
    let active = active_citizens(world);
    goods_on_the_shelf(world)
        .into_iter()
        .map(|good| {
            let stock = world
                .store
                .as_ref()
                .and_then(|s| s.stock.get(&good).copied())
                .unwrap_or(0);
            let requests: Vec<_> = world
                .store
                .as_ref()
                .map(|s| s.requests.iter().filter(|r| r.good == good).collect())
                .unwrap_or_default();
            let requesters = {
                let mut who: Vec<CitizenId> = requests.iter().map(|r| r.citizen).collect();
                who.sort_unstable();
                who.dedup();
                u32::try_from(who.len()).unwrap_or(u32::MAX)
            };
            StockView {
                good,
                stock,
                requested: requests.iter().map(|r| r.qty).sum(),
                requesters,
                my_entitlement: citizen.map_or(0, |c| store::entitlement(world, c, good)),
                my_pending: store::pending(world, me, good),
                meter_per_unit: meter_per_unit(world, good),
                share_if_shared_now: if active == 0 { 0 } else { stock / active },
            }
        })
        .collect()
}

#[utoipa::path(get, path = "/s/{id}/store", summary = "The Common Store: the shelves, the rule in force, your entitlement and pending draw, yesterday's service, your draw record",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = StoreView), (status = 422, body = Problem, description = "No Common Store in this society")),
    security(("session" = []), ("api_key" = [])))]
async fn get_store(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<StoreView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let (epoch, tick, tpc) = {
        let world = entry.handle.world.read().await;
        if world.store.is_none() {
            return Err(ApiError::Reject(Reject::new(
                RejectCode::NoStore,
                "this society has no Common Store",
            )));
        }
        (world.meta.epoch, world.meta.tick, world.ticks_per_cycle())
    };
    let cycle = tick / tpc;
    // Yesterday and today, from the log.
    let since = cycle.saturating_sub(1) * tpc;
    let requested = state
        .store
        .read_kind_since_tick(id, epoch, "StoreDrawRequested", since, READ_CAP)
        .await?;
    let drew = state
        .store
        .read_kind_since_tick(id, epoch, "Drew", since, READ_CAP)
        .await?;
    let all_draws = state.store.read_last_of_kind(id, "Drew", READ_CAP).await?;
    let world = entry.handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let days = store_days(&requested, &drew, tpc);
    let of_cycle = |c: u32| -> Vec<StoreDayView> {
        days.iter()
            .filter(|((cy, _), _)| *cy == c)
            .map(|(_, v)| v.clone())
            .collect()
    };
    let mut my_draws: Vec<_> = all_draws
        .iter()
        .filter(|e| matches!(&e.event, Event::Drew { citizen, .. } if *citizen == me))
        .filter_map(|e| event_ref(&viewer, &world, e))
        .collect();
    if my_draws.len() > PAGE {
        my_draws.drain(..my_draws.len() - PAGE);
    }
    let citizen = world
        .citizens
        .get(&me)
        .ok_or_else(|| ApiError::NotFound("citizen".into()))?;
    Ok(Json(StoreView {
        clock: clock_of(&world),
        rule: rule_in_force(&world),
        stock: shelves(&world, me),
        active_citizens: active_citizens(&world),
        last_cycle: cycle.checked_sub(1),
        yesterday: cycle.checked_sub(1).map(of_cycle).unwrap_or_default(),
        today: of_cycle(cycle),
        my_draws,
        my_pantry: citizen.household.pantry.clone(),
        pantry_capacity: world.params.pantry.clone(),
    }))
}

// -- the Ledger ------------------------------------------------------------------

/// Every citizen's line, active first by hours today (then by handle), then
/// the dormant. Hours are tick-hours over the day's ticks; the attributed
/// figure is the engine's noised one, never the true output (Q55).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn ledger_rows(world: &World, me: CitizenId) -> Vec<ContributionRow> {
    let tpc = f64::from(world.ticks_per_cycle());
    let norm_tick_hours =
        u32::from(world.policy.work_norm_hours.unwrap_or(0)) * world.ticks_per_cycle();
    let today = norms::cycle_contribution(world);
    let mut rows: Vec<ContributionRow> = world
        .citizens
        .values()
        .map(|c| {
            let (tick_hours, attributed) = today.get(&c.id).copied().unwrap_or((0, 0.0));
            let r = &c.contribution;
            ContributionRow {
                citizen: c.id.0,
                handle: c.handle.clone(),
                kind: serde_json::to_value(c.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                dormant: c.dormant,
                is_me: c.id == me,
                hours_today: f64::from(tick_hours) / tpc,
                attributed_today: attributed,
                norm_met_today: norm_tick_hours > 0 && tick_hours >= norm_tick_hours,
                hours_yesterday: f64::from(r.last_cycle_tick_hours) / tpc,
                attributed_yesterday: r.last_cycle_attributed,
                days: r.cycles,
                hours_total: r.tick_hours_total as f64 / tpc,
                attributed_total: r.attributed_total,
                norm_met_days: r.norm_met_cycles,
                honors: isms_core::metrics::honors_of(c),
                workplaces: world
                    .workplaces
                    .values()
                    .filter(|w| w.workers.contains_key(&c.id))
                    .map(|w| w.id.0)
                    .collect(),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.dormant
            .cmp(&b.dormant)
            .then_with(|| b.hours_today.total_cmp(&a.hours_today))
            .then_with(|| a.handle.cmp(&b.handle))
    });
    rows
}

#[utoipa::path(get, path = "/s/{id}/ledger", summary = "The Ledger of Contribution: every citizen's hours (exact) and output (as attributed, sigma stated), the norm met or not, honors; your row marked",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 200, body = ContributionView), (status = 422, body = Problem, description = "Labor is not by norm here")),
    security(("session" = []), ("api_key" = [])))]
async fn get_ledger(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
) -> ApiResult<Json<ContributionView>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let world = entry.handle.world.read().await;
    let rules = Rules::from_world(&world);
    if rules.capabilities.labor != LaborMode::Norm {
        return Err(ApiError::Reject(Reject::new(
            RejectCode::NotInThisSociety,
            "labor here is not by norm: there is no Ledger of Contribution",
        )));
    }
    let monitoring = serde_json::to_value(rules.capabilities.monitoring)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    Ok(Json(ContributionView {
        clock: clock_of(&world),
        norm_hours: world.policy.work_norm_hours.map(u32::from),
        monitoring,
        sigma: rules.capabilities.monitoring_sigma,
        rows: ledger_rows(&world, me),
        least_staffed: orgs::least_staffed(&world).map(|w| w.0),
        max_workers_per_workplace: world.params.labor.max_workers_per_workplace,
        max_workplaces: u32::from(world.params.labor.max_workplaces),
    }))
}

// -- positions -------------------------------------------------------------------

#[utoipa::path(post, path = "/s/{id}/workplaces/{wid}/position", summary = "Take a position at a workplace under the work norm (no contract; Q62)",
    params(("id" = i64, Path, description = "Society id"), ("wid" = u32, Path, description = "Workplace id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem)),
    security(("session" = []), ("api_key" = [])))]
async fn take_position(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, wid)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::JoinWorkplace {
        workplace: WorkplaceId(wid),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

#[utoipa::path(delete, path = "/s/{id}/workplaces/{wid}/position", summary = "Give up a norm position",
    params(("id" = i64, Path, description = "Society id"), ("wid" = u32, Path, description = "Workplace id")),
    responses((status = 200, body = Committed), (status = 422, body = Problem)),
    security(("session" = []), ("api_key" = [])))]
async fn leave_position(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, wid)): Path<(i64, u32)>,
) -> ApiResult<Json<Committed>> {
    let (entry, me) = me_in(&state, &auth, id).await?;
    let cmd = Command::LeaveWorkplace {
        workplace: WorkplaceId(wid),
    };
    Ok(Json(send(&state, &entry, &auth, me, None, cmd).await?))
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_store))
        .routes(routes!(get_ledger))
        .routes(routes!(take_position, leave_position))
}
