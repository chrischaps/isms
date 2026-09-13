//! The Common Store (GDD §6.2, §5 A2 `none`/A3 `need`; TDD §5.5 phases 2 and
//! 6, S0.15). In a moneyless society every workplace's output lands in the
//! store; a citizen's standing plan files a draw request each tick for the
//! units that would bring their meters to full, and phase 6 resolves the
//! tick's requests against the stock: everyone when it suffices, else by the
//! society's rationing rule. Need-first serves the largest request first; ties
//! are broken by the tick's RNG (TDD §5.9). `equal_shortfall` and `lottery`
//! arrive with S0.15b and fall back to need-first until then.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::CitizenId;
use crate::kinds::Good;
use crate::needs::{FULL, TENTHS};
use crate::params::Params;
use crate::policy::Rationing;
use crate::tick::TickBuilder;
use crate::world::{Citizen, StoreRequest, World};
use rand::seq::SliceRandom;
use std::collections::BTreeMap;

/// Meter tenths one unit of `good` restores, for the goods the store rations
/// by need. Other goods have no draw entitlement in Phase 0.
fn tenths_per_unit(params: &Params, good: Good) -> Option<u16> {
    match good {
        Good::Food => Some(u16::from(params.needs.food_meter_per_unit) * TENTHS),
        Good::Wares => Some(u16::from(params.needs.comfort_per_wares) * TENTHS),
        _ => None,
    }
}

/// Units of `good` the citizen has requested this tick and not yet drawn.
#[must_use]
pub fn pending(world: &World, citizen: CitizenId, good: Good) -> u32 {
    world.store.as_ref().map_or(0, |s| {
        s.requests
            .iter()
            .filter(|r| r.citizen == citizen && r.good == good)
            .map(|r| r.qty)
            .sum()
    })
}

/// Draw entitlement (GDD §6.2): the units that would bring the meter to full,
/// less what the pantry holds and what is already requested this tick, capped
/// by the pantry room (Q61).
#[must_use]
pub fn entitlement(world: &World, citizen: &Citizen, good: Good) -> u32 {
    let Some(per) = tenths_per_unit(&world.params, good) else {
        return 0;
    };
    let meter = match good {
        Good::Food => citizen.needs.food,
        _ => citizen.needs.comfort,
    };
    let shortfall = FULL.saturating_sub(meter);
    let wanted = u32::from(shortfall.div_ceil(per));
    let have = citizen.household.pantry.get(&good).copied().unwrap_or(0)
        + pending(world, citizen.id, good);
    let by_need = wanted.saturating_sub(have);
    let room = world
        .params
        .pantry
        .get(&good)
        .map_or(u32::MAX, |cap| cap.saturating_sub(have));
    by_need.min(room)
}

/// `RequestStoreDraw`: file this tick's request for `qty` units of `good`.
pub fn request_store_draw(
    world: &World,
    envelope: &Envelope<Command>,
    good: Good,
    qty: u32,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    if world.store.is_none() {
        return Err(Reject::new(
            RejectCode::NoStore,
            "this society has no Common Store",
        ));
    }
    if qty == 0 {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "qty must be positive",
        ));
    }
    let allowed = entitlement(world, citizen, good);
    if qty > allowed {
        return Err(Reject::new(
            RejectCode::OverEntitlement,
            format!(
                "{} may draw {allowed} {good:?} this tick, not {qty}",
                citizen.id
            ),
        ));
    }
    Ok(vec![Event::StoreDrawRequested {
        citizen: citizen.id,
        good,
        qty,
        tick: envelope.received_at_tick,
    }])
}

/// Phase 2 for a Common Store society: file the Food and Wares draws the
/// citizen is entitled to. Runs through `handle` like every plan action (Q6).
pub fn execute_store_plan(b: &mut TickBuilder, id: CitizenId) {
    for good in [Good::Food, Good::Wares] {
        let Some(c) = b.world.citizens.get(&id) else {
            return;
        };
        if c.dormant {
            return;
        }
        let qty = entitlement(&b.world, c, good);
        if qty > 0 {
            crate::plan::run_plan_command(b, id, Command::RequestStoreDraw { good, qty });
        }
    }
}

/// One resolved request: how much was asked and how much the store served.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Served {
    pub citizen: CitizenId,
    pub requested: u32,
    pub served: u32,
}

/// Need-first (GDD §6.2): the largest request is served first; ties are in
/// RNG order. Every request is served in full while the stock lasts.
#[must_use]
pub fn need_first(rng: &mut impl rand::Rng, requests: &[StoreRequest], stock: u32) -> Vec<Served> {
    let mut order: Vec<&StoreRequest> = requests.iter().collect();
    order.shuffle(rng);
    order.sort_by_key(|r| std::cmp::Reverse(r.qty));
    let mut remaining = stock;
    order
        .into_iter()
        .map(|r| {
            let served = r.qty.min(remaining);
            remaining -= served;
            Served {
                citizen: r.citizen,
                requested: r.qty,
                served,
            }
        })
        .collect()
}

/// Phase 6 for a Common Store society: resolve this tick's requests, good by
/// good. When the stock covers every request they are served in arrival
/// order; otherwise the society's rationing rule decides.
pub fn phase_6_store(b: &mut TickBuilder) {
    let Some(store) = &b.world.store else {
        return;
    };
    if store.requests.is_empty() {
        return;
    }
    let rule = b.world.policy.rationing.unwrap_or(Rationing::NeedFirst);
    let mut by_good: BTreeMap<Good, Vec<StoreRequest>> = BTreeMap::new();
    for r in &store.requests {
        by_good.entry(r.good).or_default().push(*r);
    }
    for (good, requests) in by_good {
        let stock = b
            .world
            .store
            .as_ref()
            .and_then(|s| s.stock.get(&good).copied())
            .unwrap_or(0);
        let total: u32 = requests.iter().map(|r| r.qty).sum();
        let (served, rule_id) = if total <= stock {
            (
                requests
                    .iter()
                    .map(|r| Served {
                        citizen: r.citizen,
                        requested: r.qty,
                        served: r.qty,
                    })
                    .collect::<Vec<_>>(),
                RuleId::StoreDrawNeedFirst,
            )
        } else {
            match rule {
                // S0.15b implements the other two rules; until then every
                // shortage is rationed need-first (T8 default).
                Rationing::NeedFirst | Rationing::EqualShortfall | Rationing::Lottery => (
                    need_first(&mut b.rng, &requests, stock),
                    RuleId::StoreDrawNeedFirst,
                ),
            }
        };
        for s in served.into_iter().filter(|s| s.served > 0) {
            let explain = Explain::new(rule_id, "min(requested, remaining stock)", s.served)
                .input("requested", s.requested)
                .input("stock", stock)
                .input("served", s.served);
            b.emit(Event::Drew {
                citizen: s.citizen,
                goods: BTreeMap::from([(good, s.served)]),
                explain,
            });
        }
    }
}
