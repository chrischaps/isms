//! The standing plan (GDD §9.3; TDD §5.5 phase 2, §6 `StandingPlan`) and
//! dormancy (step 8k). The executor issues ordinary commands through `handle`
//! against the scratch world with `ClientKind::Plan`, so its actions are
//! ordinary events (Q6). Rejections are silently skipped: the plan does what it
//! can.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen, handle};
use crate::constitution::{Governance, LaborMode};
use crate::event::{Actor, Event};
use crate::ids::{CitizenId, Tick};
use crate::kinds::{CitizenKind, ClientKind, Good};
use crate::market::{last_price, open_bid_qty};
use crate::money::Money;
use crate::needs::TENTHS;
use crate::tick::TickBuilder;
use crate::world::{
    Instrument, LaborPlan, OrderSource, Refresh, Side, StandingPlan, VoteDefault, World,
};

/// `SetStandingPlan`: full replace, validated against capabilities.
pub fn set_standing_plan(
    world: &World,
    envelope: &Envelope<Command>,
    plan: &StandingPlan,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let c = &world.constitution;
    let money = c.has_money();
    let order_books = c.pricing == crate::constitution::Pricing::Market;
    let bad = |what: &str| {
        Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{what} does not exist in this society"),
        ))
    };
    match plan.labor {
        LaborPlan::Explicit if c.labor == LaborMode::Assigned => {
            return bad("an explicit labor plan");
        }
        LaborPlan::AcceptAssignment if c.labor != LaborMode::Assigned => {
            return bad("accepting assignments");
        }
        LaborPlan::FollowNorm if c.labor != LaborMode::Norm => return bad("following a work norm"),
        _ => {}
    }
    if !money && (plan.keep_balance_at_least != Money::ZERO || plan.max_food_price.is_some()) {
        return bad("a money rule");
    }
    if !money
        && plan
            .buy_wares_when
            .is_some_and(|r| r.balance_above != Money::ZERO || r.max_price.is_some())
    {
        return bad("a money rule");
    }
    if !order_books && !plan.standing_orders.is_empty() {
        return bad("a standing order");
    }
    if plan
        .standing_orders
        .iter()
        .any(|o| o.qty == 0 || o.limit_price <= Money::ZERO)
    {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "standing orders need a positive quantity and price",
        ));
    }
    if matches!(plan.vote_default, VoteDefault::Follow(_)) && c.governance == Governance::None {
        return bad("a vote default");
    }
    if let VoteDefault::Follow(who) = plan.vote_default
        && !world.citizens.contains_key(&who)
    {
        return Err(Reject::new(
            RejectCode::UnknownCitizen,
            format!("no citizen {who}"),
        ));
    }
    Ok(vec![Event::PlanChanged {
        citizen: citizen.id,
        plan: Box::new(plan.clone()),
    }])
}

/// `last x 1.25`, rounded to the cent.
#[must_use]
pub fn default_max_price(last: Money) -> Money {
    Money(last.0 + (last.0 + 2) / 4)
}

/// The price a plan bid is placed at: the best resting ask when one rests
/// (capped at `limit`), else the last price (capped). Bidding at `limit`
/// itself would let every incoming ask fill at the bidder's ceiling and ratchet
/// the last price upward each tick (Q44).
fn bid_price(world: &World, instrument: Instrument, last: Money, limit: Money) -> Money {
    let best_ask = crate::market::depth(world, instrument, Side::Ask)
        .first()
        .map(|(p, _)| *p);
    best_ask.map_or(last.min(limit), |a| a.min(limit))
}

fn plan_envelope(citizen: CitizenId, command: Command, tick: Tick) -> Envelope<Command> {
    Envelope {
        actor: Actor::Citizen(citizen),
        on_behalf_of: None,
        client_kind: ClientKind::Plan,
        received_at_tick: tick,
        command,
    }
}

/// Run one citizen's plan against the scratch world: commands go through
/// `handle`; their events are emitted as ordinary events.
pub(crate) fn run_plan_command(b: &mut TickBuilder, citizen: CitizenId, command: Command) -> bool {
    let env = plan_envelope(citizen, command, b.tick);
    match handle(&b.world, b.rules, &env) {
        Ok(events) => {
            for e in events {
                b.emit(e);
            }
            true
        }
        Err(_) => false,
    }
}

/// Phase 2: the same plan fields drive bids in market systems and store draws
/// in Common Store systems (TDD §6), in the tick's shuffled order. State-store
/// requests arrive with S0.16.
pub fn phase_2_standing_plans(b: &mut TickBuilder, order: &[CitizenId]) {
    if b.rules.capabilities.order_books {
        for &id in order {
            execute_market_plan(b, id);
        }
    } else if b.rules.capabilities.common_store {
        for &id in order {
            crate::store::execute_store_plan(b, id);
        }
    } else if b.rules.capabilities.administered_prices {
        for &id in order {
            crate::state_store::execute_administered_plan(b, id);
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
fn execute_market_plan(b: &mut TickBuilder, id: CitizenId) {
    let Some(c) = b.world.citizens.get(&id) else {
        return;
    };
    if c.dormant {
        return;
    }
    let plan = c.plan.clone();
    let params = b.world.params.clone();
    let tick = b.tick;
    let first_tick_of_cycle = tick.is_multiple_of(params.time.ticks_per_cycle);

    // Refresh standing orders: cancel this plan's resting orders, then re-place.
    for so in &plan.standing_orders {
        let due = match so.refresh {
            Refresh::EachTick => true,
            Refresh::EachCycle => first_tick_of_cycle,
        };
        if !due {
            continue;
        }
        let resting: Vec<crate::ids::OrderId> = b
            .world
            .books
            .get(&so.instrument)
            .map(|bk| {
                bk.orders
                    .values()
                    .filter(|o| {
                        o.owner == crate::ledger::Party::Citizen(id)
                            && o.side == so.side
                            && o.source == OrderSource::Standing
                    })
                    .map(|o| o.id)
                    .collect()
            })
            .unwrap_or_default();
        for oid in resting {
            run_plan_command(b, id, Command::CancelOrder { order: oid });
        }
        let expires = match so.refresh {
            Refresh::EachTick => Some(tick + 1),
            Refresh::EachCycle => {
                Some((tick / params.time.ticks_per_cycle + 1) * params.time.ticks_per_cycle - 1)
            }
        };
        run_plan_command(
            b,
            id,
            Command::PlaceOrder {
                instrument: so.instrument,
                side: so.side,
                qty: so.qty,
                limit_price: so.limit_price,
                expires_tick: expires,
            },
        );
    }

    // Re-price resting plan bids that a crossable ask now sits above (Q44): a
    // bid placed at the last price would otherwise rest below a cost-plus ask
    // one cent higher until it expired.
    for good in [Good::Food, Good::Wares] {
        // Standing orders are the citizen's own prices; only the rules' bids re-price.
        if plan
            .standing_orders
            .iter()
            .any(|o| o.instrument == Instrument::Good(good) && o.side == Side::Bid)
        {
            continue;
        }
        let limit = match good {
            Good::Food => plan.max_food_price,
            _ => plan.buy_wares_when.and_then(|r| r.max_price),
        };
        let Some(last) = last_price(&b.world, Instrument::Good(good)) else {
            continue;
        };
        let limit = limit.unwrap_or_else(|| default_max_price(last));
        let Some((best_ask, _)) = crate::market::depth(&b.world, Instrument::Good(good), Side::Ask)
            .first()
            .copied()
        else {
            continue;
        };
        if best_ask > limit {
            continue;
        }
        let stale: Vec<crate::ids::OrderId> = b
            .world
            .books
            .get(&Instrument::Good(good))
            .map(|bk| {
                bk.orders
                    .values()
                    .filter(|o| {
                        o.owner == crate::ledger::Party::Citizen(id)
                            && o.side == Side::Bid
                            && o.source == OrderSource::Standing
                            && o.limit_price < best_ask
                    })
                    .map(|o| o.id)
                    .collect()
            })
            .unwrap_or_default();
        for oid in stale {
            run_plan_command(b, id, Command::CancelOrder { order: oid });
        }
    }

    // Consumption rules: Food shortfall, then Wares.
    let Some(c) = b.world.citizens.get(&id) else {
        return;
    };
    let balance = c.household.balance;
    let spendable = (balance - plan.keep_balance_at_least).max_zero();
    let party = crate::ledger::Party::Citizen(id);
    let held = |good: Good, w: &World| {
        w.citizens[&id]
            .household
            .pantry
            .get(&good)
            .copied()
            .unwrap_or(0)
    };
    let room = |good: Good, w: &World| {
        params.pantry.get(&good).map_or(u32::MAX, |cap| {
            cap.saturating_sub(held(good, w) + open_bid_qty(w, party, good))
        })
    };

    let food_have = held(Good::Food, &b.world) + open_bid_qty(&b.world, party, Good::Food);
    if plan.keep_food_at_least > food_have
        && let Some(last) = last_price(&b.world, Instrument::Good(Good::Food))
    {
        // Hunger before savings (Q97): with an empty pantry the balance reserve
        // does not apply to the Food bid. A saver keeps their reserve until
        // there is nothing left to eat, then dips into it.
        let spendable = if held(Good::Food, &b.world) == 0 {
            balance
        } else {
            spendable
        };
        let limit = plan
            .max_food_price
            .unwrap_or_else(|| default_max_price(last));
        let limit = bid_price(&b.world, Instrument::Good(Good::Food), last, limit);
        if limit > Money::ZERO {
            let affordable = (spendable.0 / limit.0) as u32;
            let qty = (plan.keep_food_at_least - food_have)
                .min(affordable)
                .min(room(Good::Food, &b.world));
            if qty > 0 {
                run_plan_command(
                    b,
                    id,
                    Command::PlaceOrder {
                        instrument: Instrument::Good(Good::Food),
                        side: Side::Bid,
                        qty,
                        limit_price: limit,
                        expires_tick: None,
                    },
                );
            }
        }
    }

    if let Some(rule) = plan.buy_wares_when {
        let c = &b.world.citizens[&id];
        let comfort_points = c.needs.comfort / TENTHS;
        let spendable = (c.household.balance - plan.keep_balance_at_least).max_zero();
        if comfort_points < u16::from(rule.comfort_below)
            && c.household.balance > rule.balance_above
            && open_bid_qty(&b.world, party, Good::Wares) == 0
            && let Some(last) = last_price(&b.world, Instrument::Good(Good::Wares))
        {
            let limit = rule.max_price.unwrap_or_else(|| default_max_price(last));
            let limit = bid_price(&b.world, Instrument::Good(Good::Wares), last, limit);
            let per = u16::from(params.needs.comfort_per_wares);
            let wanted = u32::from((100 - comfort_points).div_ceil(per)).max(1);
            let affordable = (spendable.0 / limit.0.max(1)) as u32;
            let qty = wanted.min(affordable).min(room(Good::Wares, &b.world));
            if qty > 0 {
                run_plan_command(
                    b,
                    id,
                    Command::PlaceOrder {
                        instrument: Instrument::Good(Good::Wares),
                        side: Side::Bid,
                        qty,
                        limit_price: limit,
                        expires_tick: None,
                    },
                );
            }
        }
    }
}

/// Step 8k: a human absent for `dormancy_absent_cycles` full cycles goes dormant;
/// their open orders are cancelled first so the freeze is complete. Householders
/// never go dormant (they emigrate instead).
pub fn cycle_end_8k_dormancy(b: &mut TickBuilder) {
    let tpc = b.world.params.time.ticks_per_cycle;
    let limit = b.world.params.population.dormancy_absent_cycles * tpc;
    let tick = b.tick;
    let candidates: Vec<CitizenId> = b
        .world
        .citizens
        .values()
        .filter(|c| {
            c.kind == CitizenKind::Human && !c.dormant && tick + 1 >= c.last_seen_tick + limit
        })
        .map(|c| c.id)
        .collect();
    for id in candidates {
        let orders: Vec<(crate::ids::OrderId, crate::ledger::Asset)> = b
            .world
            .books
            .values()
            .flat_map(|bk| bk.orders.values())
            .filter(|o| o.owner == crate::ledger::Party::Citizen(id))
            .map(|o| (o.id, crate::market::escrow_of(o)))
            .collect();
        for (order, released) in orders {
            b.emit(Event::OrderCancelled { order, released });
        }
        // A collective dwelling is released while its occupant is away (GDD 9.3, Q25).
        if let Some(dwelling) = crate::housing::society_dwelling_of(&b.world, id) {
            b.emit(Event::DwellingOccupied {
                dwelling,
                citizen: None,
            });
        }
        b.emit(Event::CitizenDormant { citizen: id });
    }
}

/// The indexes of `events` that touch `citizen`: the "while you were away" digest.
#[must_use]
pub fn away_digest(citizen: CitizenId, events: &[Event]) -> Vec<usize> {
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| touches(e, citizen))
        .map(|(i, _)| i)
        .collect()
}

/// Whether an event is about this citizen (for digests and the API stream).
#[must_use]
#[allow(clippy::match_same_arms)] // grouped by payload shape for readability
pub fn touches(event: &Event, id: CitizenId) -> bool {
    use crate::ledger::Party::Citizen as P;
    match event {
        Event::CitizenJoined { citizen, .. }
        | Event::CitizenSeen { citizen, .. }
        | Event::CitizenDormant { citizen }
        | Event::CitizenReturned { citizen }
        | Event::HouseholderJoined { citizen, .. }
        | Event::HouseholderEmigrated { citizen, .. }
        | Event::PlanChanged { citizen, .. }
        | Event::LaborSet { citizen, .. }
        | Event::Assigned { citizen, .. }
        | Event::Unassigned { citizen, .. }
        | Event::DividendPaid { citizen, .. }
        | Event::Paid { citizen, .. }
        | Event::PaymentMissed { citizen, .. }
        | Event::Drew { citizen, .. }
        | Event::StoreDrawRequested { citizen, .. }
        | Event::StoreReturned { citizen, .. }
        | Event::Pledged { citizen, .. }
        | Event::StateStoreRequested { citizen, .. }
        | Event::StateStoreSold { citizen, .. }
        | Event::RationIssued { citizen, .. }
        | Event::TransferRequested { citizen, .. }
        | Event::TransferDecided { citizen, .. }
        | Event::TaxAssessed { citizen, .. }
        | Event::NeedFloorPaid { citizen, .. }
        | Event::ShareOutPaid { citizen, .. }
        | Event::DuesPaid { citizen, .. }
        | Event::StrikePaid { citizen, .. }
        | Event::HardshipBegan { citizen, .. }
        | Event::HardshipEnded { citizen, .. }
        | Event::DestitutionBegan { citizen, .. }
        | Event::DestitutionEnded { citizen, .. }
        | Event::EmploymentAccepted { citizen, .. } => *citizen == id,
        Event::Transferred { from, to, .. } => *from == P(id) || *to == P(id),
        Event::Trade { buyer, seller, .. } => *buyer == P(id) || *seller == P(id),
        Event::OrderPlaced { order, .. } => order.owner == P(id),
        Event::SaleOffered { by, .. } | Event::WantedPosted { by, .. } => *by == P(id),
        Event::SaleAccepted { buyer, seller, .. } => *buyer == P(id) || *seller == P(id),
        Event::OrgFounded { founder, .. } => *founder == Some(id),
        Event::ManagerAppointed { citizen, .. } => *citizen == Some(id),
        Event::Produced { per_worker, .. } => per_worker.iter().any(|w| w.citizen == id),
        Event::EmploymentTerminated { by, .. } => *by == P(id),
        Event::CreditAccepted {
            lender, borrower, ..
        } => *lender == P(id) || *borrower == P(id),
        Event::LeaseAccepted { owner, tenant, .. } => *owner == P(id) || *tenant == P(id),
        Event::DwellingOccupied { citizen, .. } => *citizen == Some(id),
        Event::CreditOffered { by, .. } => *by == P(id),
        Event::MembershipRequested { citizen, .. }
        | Event::MemberAdmitted { citizen, .. }
        | Event::MemberLeft { citizen, .. } => *citizen == id,
        _ => false,
    }
}
