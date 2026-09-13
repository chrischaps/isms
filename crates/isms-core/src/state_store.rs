//! The state store (GDD §6.3, §5 A2 `administered`, A6 `provision`; TDD §5.5
//! phases 2 and 6 and step 8b, S0.16a). In an administered society every state
//! enterprise's output lands in the state stock and is sold at the published
//! price list. A citizen's standing plan files a purchase request each tick;
//! phase 6 serves the tick's requests in arrival order, each within the stock,
//! the citizen's balance and any ration card (an equal cap per citizen per
//! cycle, Q66). What could not be served is a shortage. Provision (step 8b)
//! tops every citizen's pantry up to the minimum Food ration at zero price.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::constitution::Redistribution;
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::CitizenId;
use crate::kinds::Good;
use crate::ledger::Holder;
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::world::{StateRequest, World};
use std::collections::BTreeMap;

/// The published price of a good, if the price list carries it.
#[must_use]
pub fn list_price(world: &World, good: Good) -> Option<Money> {
    world
        .policy
        .price_list
        .as_ref()
        .and_then(|l| l.get(&good).copied())
}

/// Units of `good` the citizen has requested this tick and not yet been served.
#[must_use]
pub fn pending(world: &World, citizen: CitizenId, good: Good) -> u32 {
    world.state_stock.as_ref().map_or(0, |s| {
        s.requests
            .iter()
            .filter(|r| r.citizen == citizen && r.good == good)
            .map(|r| r.qty)
            .sum()
    })
}

/// Units of `good` the citizen may still buy this cycle under a ration card.
#[must_use]
pub fn ration_remaining(world: &World, citizen: CitizenId, good: Good) -> u32 {
    let Some(cap) = world
        .policy
        .ration_caps
        .as_ref()
        .and_then(|c| c.get(&good).copied())
    else {
        return u32::MAX;
    };
    let issued = world
        .state_stock
        .as_ref()
        .and_then(|s| s.issued_this_cycle.get(&citizen))
        .and_then(|m| m.get(&good).copied())
        .unwrap_or(0);
    cap.saturating_sub(issued)
}

/// `RequestStateStore`: file this tick's request to buy `qty` of `good` at list.
pub fn request_state_store(
    world: &World,
    envelope: &Envelope<Command>,
    good: Good,
    qty: u32,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    if world.state_stock.is_none() {
        return Err(Reject::new(
            RejectCode::NoStore,
            "this society has no state store",
        ));
    }
    if list_price(world, good).is_none() {
        return Err(Reject::new(
            RejectCode::NotOnPriceList,
            format!("{good:?} is not on the price list"),
        ));
    }
    if qty == 0 {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "qty must be positive",
        ));
    }
    if let Some(cap) = world.params.pantry.get(&good) {
        let have = citizen.household.pantry.get(&good).copied().unwrap_or(0)
            + pending(world, citizen.id, good);
        if have + qty > *cap {
            return Err(Reject::new(
                RejectCode::PantryFull,
                format!(
                    "{} holds or awaits {have} {good:?}; the pantry cap is {cap}",
                    citizen.id
                ),
            ));
        }
    }
    Ok(vec![Event::StateStoreRequested {
        citizen: citizen.id,
        good,
        qty,
        tick: envelope.received_at_tick,
    }])
}

/// Phase 2 for an administered society: the plan's Food target and Wares rule
/// become purchase requests at the published price, within the saving floor.
/// The plan's own price limits do not apply: the citizen cannot bargain.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn execute_administered_plan(b: &mut TickBuilder, id: CitizenId) {
    let Some(c) = b.world.citizens.get(&id) else {
        return;
    };
    if c.dormant {
        return;
    }
    let plan = c.plan.clone();
    let params = b.world.params.clone();
    let held = |good: Good, w: &World| {
        w.citizens[&id]
            .household
            .pantry
            .get(&good)
            .copied()
            .unwrap_or(0)
            + pending(w, id, good)
    };
    let room = |good: Good, w: &World| {
        params
            .pantry
            .get(&good)
            .map_or(u32::MAX, |cap| cap.saturating_sub(held(good, w)))
    };
    let affordable = |good: Good, w: &World| {
        let spendable = (w.citizens[&id].household.balance - plan.keep_balance_at_least).max_zero();
        match list_price(w, good) {
            Some(p) if p > Money::ZERO => (spendable.0 / p.0) as u32,
            Some(_) => u32::MAX,
            None => 0,
        }
    };
    // Food up to the plan's target.
    let have = held(Good::Food, &b.world);
    if plan.keep_food_at_least > have {
        let qty = (plan.keep_food_at_least - have)
            .min(affordable(Good::Food, &b.world))
            .min(room(Good::Food, &b.world));
        if qty > 0 {
            crate::plan::run_plan_command(
                b,
                id,
                Command::RequestStateStore {
                    good: Good::Food,
                    qty,
                },
            );
        }
    }
    // Wares when Comfort is low and the balance allows.
    if let Some(rule) = plan.buy_wares_when {
        let c = &b.world.citizens[&id];
        let comfort_points = c.needs.comfort / crate::needs::TENTHS;
        if comfort_points < u16::from(rule.comfort_below)
            && c.household.balance > rule.balance_above
            && pending(&b.world, id, Good::Wares) == 0
        {
            let per = u16::from(params.needs.comfort_per_wares);
            let wanted = u32::from((100 - comfort_points).div_ceil(per)).max(1);
            let qty = wanted
                .min(affordable(Good::Wares, &b.world))
                .min(room(Good::Wares, &b.world));
            if qty > 0 {
                crate::plan::run_plan_command(
                    b,
                    id,
                    Command::RequestStateStore {
                        good: Good::Wares,
                        qty,
                    },
                );
            }
        }
    }
}

/// Phase 6 for an administered society: serve this tick's requests in arrival
/// order at the list price, each within the stock, the buyer's balance and the
/// ration card; record what could not be served.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn phase_6_state_store(b: &mut TickBuilder) {
    let Some(stock) = &b.world.state_stock else {
        return;
    };
    let requests: Vec<StateRequest> = stock.requests.clone();
    if requests.is_empty() {
        return;
    }
    let mut unfilled: BTreeMap<Good, u32> = BTreeMap::new();
    for r in requests {
        let Some(price) = list_price(&b.world, r.good) else {
            *unfilled.entry(r.good).or_insert(0) += r.qty;
            continue;
        };
        let on_hand = crate::ledger::goods_at(&b.world, Holder::StateStock, r.good);
        let balance = b.world.citizens[&r.citizen].household.balance;
        let affordable = if price > Money::ZERO {
            (balance.0 / price.0) as u32
        } else {
            u32::MAX
        };
        let card = ration_remaining(&b.world, r.citizen, r.good);
        let served = r.qty.min(on_hand).min(affordable).min(card);
        if served > 0 {
            let total = Money(price.0 * i64::from(served));
            let explain = Explain::new(
                RuleId::StateStorePrice,
                "min(requested, stock, balance / price, ration left) x list price",
                total,
            )
            .input("requested", r.qty)
            .input("stock", on_hand)
            .input("affordable", affordable.min(r.qty))
            .input("ration_left", card.min(r.qty))
            .input("unit_price", price)
            .input("served", served);
            b.emit(Event::StateStoreSold {
                citizen: r.citizen,
                good: r.good,
                qty: served,
                unit_price: price,
                total,
                explain,
            });
        }
        if served < r.qty {
            *unfilled.entry(r.good).or_insert(0) += r.qty - served;
        }
    }
    if !unfilled.is_empty() {
        b.emit(Event::StateStoreShortage {
            tick: b.tick,
            unfilled,
        });
    }
}

/// Step 8b (provision systems with a state stock): top every active citizen's
/// pantry up to the minimum Food ration at zero price, in citizen order, while
/// the stock lasts (GDD §6.3; Q68).
pub fn cycle_end_8b_provision_ration(b: &mut TickBuilder) {
    if b.rules.capabilities.redistribution != Redistribution::Provision
        || b.world.state_stock.is_none()
    {
        return;
    }
    let Some(ration) = b.world.policy.minimum_food_ration else {
        return;
    };
    let ids: Vec<CitizenId> = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| c.id)
        .collect();
    for id in ids {
        let have = b.world.citizens[&id]
            .household
            .pantry
            .get(&Good::Food)
            .copied()
            .unwrap_or(0);
        let on_hand = crate::ledger::goods_at(&b.world, Holder::StateStock, Good::Food);
        let qty = ration.saturating_sub(have).min(on_hand);
        if qty == 0 {
            continue;
        }
        let explain = Explain::new(
            RuleId::ProvisionRation,
            "min(minimum_food_ration - pantry, stock)",
            qty,
        )
        .input("minimum_food_ration", ration)
        .input("pantry", have)
        .input("stock", on_hand);
        b.emit(Event::RationIssued {
            citizen: id,
            good: Good::Food,
            qty,
            explain,
        });
    }
}
