//! Order books (GDD §4.5, §7.2; TDD D5, §5.5 phase 6, ADR-0001). One continuous
//! double auction per instrument: orders match on submission at the resting
//! order's price with price–time priority; ticks expire orders and record the
//! tick's VWAP and the basket price index.

use crate::command::{Command, Envelope, Reject, RejectCode};
use crate::event::Event;
use crate::ids::{OrderId, Tick};
use crate::kinds::Good;
use crate::ledger::{Asset, Party};
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::transfers::{acting_party, check_has, check_pantry_room};
use crate::world::{Instrument, Order, OrderBook, OrderSource, Side, World};
use std::collections::BTreeMap;

/// Money a bid must escrow: `remaining × limit`.
#[must_use]
pub fn bid_escrow(remaining: u32, limit: Money) -> Money {
    Money(limit.0 * i64::from(remaining))
}

/// What an order holds in escrow right now (money for bids, goods for asks of
/// goods). Share asks hold shares in `World::share_escrow` instead.
#[must_use]
pub fn escrow_of(order: &Order) -> Asset {
    match (order.side, order.instrument) {
        (Side::Bid, _) => Asset::Money(bid_escrow(order.remaining, order.limit_price)),
        (Side::Ask, Instrument::Good(g)) => Asset::Good(g, order.remaining),
        (Side::Ask, Instrument::Share(_)) => Asset::Money(Money::ZERO),
    }
}

/// The default expiry: the end of the cycle `order_expiry_cycles` after this one.
#[must_use]
pub fn default_expiry(world: &World) -> Tick {
    let tpc = world.params.time.ticks_per_cycle;
    let cycle = world.cycle_of(world.meta.tick);
    (cycle + 1 + world.params.market.order_expiry_cycles) * tpc - 1
}

/// Last traded price, else the start price, else the book value (S0.10), else none (Q8).
#[must_use]
pub fn last_price(world: &World, instrument: Instrument) -> Option<Money> {
    if let Some(p) = world.books.get(&instrument).and_then(|b| b.last_price) {
        return Some(p);
    }
    match instrument {
        Instrument::Good(g) => world.params.money.start_prices.get(&g).copied(),
        Instrument::Share(_) => None,
    }
}

/// Resting orders on one side, best price first, then by id (time).
fn resting(book: &OrderBook, side: Side) -> Vec<&Order> {
    let mut v: Vec<&Order> = book
        .orders
        .values()
        .filter(|o| o.side == side && o.remaining > 0)
        .collect();
    match side {
        Side::Ask => v.sort_by(|a, b| a.limit_price.cmp(&b.limit_price).then(a.id.cmp(&b.id))),
        Side::Bid => v.sort_by(|a, b| b.limit_price.cmp(&a.limit_price).then(a.id.cmp(&b.id))),
    }
    v
}

/// Open bid quantity a party has resting for a good (for the pantry cap, Q12).
pub fn open_bid_qty(world: &World, party: Party, good: Good) -> u32 {
    world.books.get(&Instrument::Good(good)).map_or(0, |b| {
        b.orders
            .values()
            .filter(|o| o.owner == party && o.side == Side::Bid)
            .map(|o| o.remaining)
            .sum()
    })
}

/// `PlaceOrder`: escrow, then match against the opposite side.
pub fn place_order(
    world: &World,
    envelope: &Envelope<Command>,
    instrument: Instrument,
    side: Side,
    qty: u32,
    limit_price: Money,
    expires_tick: Option<Tick>,
) -> Result<Vec<Event>, Reject> {
    let owner = acting_party(world, envelope)?;
    if qty == 0 {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "quantity must be positive",
        ));
    }
    if limit_price <= Money::ZERO {
        return Err(Reject::new(
            RejectCode::InvalidPrice,
            "limit price must be positive",
        ));
    }
    if let Instrument::Share(org) = instrument {
        let o = world
            .orgs
            .get(&org)
            .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
        if !matches!(o.ownership, crate::world::Ownership::Shares { .. }) {
            return Err(Reject::new(
                RejectCode::NotInThisSociety,
                "no share registry",
            ));
        }
        if !world
            .constitution
            .contracts
            .contains(&crate::kinds::ContractKind::Share)
        {
            return Err(Reject::new(
                RejectCode::NotInThisSociety,
                "shares do not trade here",
            ));
        }
    }
    let tick = world.meta.tick;
    let expires_tick = expires_tick.unwrap_or_else(|| default_expiry(world));
    if expires_tick < tick {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            "expiry is in the past",
        ));
    }
    let order = Order {
        id: world.next.order,
        owner,
        instrument,
        side,
        qty,
        remaining: qty,
        limit_price,
        placed_tick: tick,
        expires_tick,
        source: if envelope.client_kind == crate::kinds::ClientKind::Plan {
            OrderSource::Standing
        } else {
            OrderSource::Manual
        },
    };
    let escrow = escrow_of(&order);
    match (side, instrument) {
        (Side::Ask, Instrument::Share(org)) => {
            let held = crate::shares::shares_held(world, owner, org);
            if held < u64::from(qty) {
                return Err(Reject::new(
                    RejectCode::InsufficientGoods,
                    format!("{owner:?} holds {held} shares of {org}"),
                ));
            }
        }
        (Side::Bid, Instrument::Good(good)) => {
            check_has(world, owner, escrow)?;
            check_pantry_room(world, owner, good, open_bid_qty(world, owner, good) + qty)?;
        }
        _ => check_has(world, owner, escrow)?,
    }
    let mut events = vec![Event::OrderPlaced { order, escrow }];
    let fills = matches_for(world, &order);
    if let Instrument::Share(org) = instrument {
        // Control passes with the shares (Q29): the buyer of the last fill that
        // crosses 50% becomes manager.
        let mut bought: std::collections::BTreeMap<Party, u64> = std::collections::BTreeMap::new();
        for f in &fills {
            if let Event::Trade { buyer, qty, .. } = f {
                *bought.entry(*buyer).or_insert(0) += u64::from(*qty);
            }
        }
        let mut control = Vec::new();
        for (buyer, qty) in bought {
            if let Some(h) = crate::shares::holder_of(buyer, org)
                && let Some(e) = crate::shares::control_change(world, org, h, qty)
            {
                control.push(e);
            }
        }
        events.extend(fills);
        events.extend(control);
    } else {
        events.extend(fills);
    }
    Ok(events)
}

/// The fills an incoming order takes from the resting opposite side, at the
/// resting orders' prices, best price first, then oldest first.
fn matches_for(world: &World, incoming: &Order) -> Vec<Event> {
    let Some(book) = world.books.get(&incoming.instrument) else {
        return Vec::new();
    };
    let opposite = match incoming.side {
        Side::Bid => Side::Ask,
        Side::Ask => Side::Bid,
    };
    let mut remaining = incoming.remaining;
    let mut trades = Vec::new();
    for rest in resting(book, opposite) {
        if remaining == 0 {
            break;
        }
        let crosses = match incoming.side {
            Side::Bid => rest.limit_price <= incoming.limit_price,
            Side::Ask => rest.limit_price >= incoming.limit_price,
        };
        if !crosses {
            break;
        }
        let qty = remaining.min(rest.remaining);
        remaining -= qty;
        let (buyer, seller, buy_order, sell_order) = match incoming.side {
            Side::Bid => (incoming.owner, rest.owner, incoming.id, rest.id),
            Side::Ask => (rest.owner, incoming.owner, rest.id, incoming.id),
        };
        trades.push(Event::Trade {
            instrument: incoming.instrument,
            buyer,
            seller,
            buy_order,
            sell_order,
            qty,
            price: rest.limit_price,
            tick: world.meta.tick,
        });
    }
    trades
}

/// `CancelOrder`: the owner releases the remaining escrow.
pub fn cancel_order(
    world: &World,
    envelope: &Envelope<Command>,
    id: OrderId,
) -> Result<Vec<Event>, Reject> {
    let party = acting_party(world, envelope)?;
    let order = world
        .books
        .values()
        .find_map(|b| b.orders.get(&id))
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrder, format!("no order {id}")))?;
    if order.owner != party {
        return Err(Reject::new(RejectCode::NotParty, "only the owner cancels"));
    }
    Ok(vec![Event::OrderCancelled {
        order: id,
        released: escrow_of(order),
    }])
}

/// Phase 6 (market part): expire orders, then record the tick's VWAPs and the
/// basket price index. Trades since the last tick are accumulated by `apply`.
pub fn phase_6_markets(b: &mut TickBuilder) -> (BTreeMap<Instrument, Money>, Option<f64>) {
    let tick = b.tick;
    let expired: Vec<(OrderId, Asset)> = b
        .world
        .books
        .values()
        .flat_map(|bk| bk.orders.values())
        .filter(|o| o.expires_tick <= tick)
        .map(|o| (o.id, escrow_of(o)))
        .collect();
    for (order, released) in expired {
        b.emit(Event::OrderExpired { order, released });
    }
    let vwap: BTreeMap<Instrument, Money> = b
        .world
        .books
        .iter()
        .filter(|(_, bk)| bk.tick_volume > 0)
        .map(|(i, bk)| (*i, Money(bk.tick_value.0 / i64::from(bk.tick_volume))))
        .collect();
    (vwap, price_index(&b.world))
}

/// Basket index: `sum(w_g * price_g) / sum(w_g * start_g)` over goods with both (Q23).
#[must_use]
pub fn price_index(world: &World) -> Option<f64> {
    let mut num = 0.0;
    let mut den = 0.0;
    for (product, weight) in &world.params.basket {
        let Some(good) = product.as_good() else {
            continue;
        };
        let Some(start) = world.params.money.start_prices.get(&good) else {
            continue;
        };
        let Some(now) = last_price(world, Instrument::Good(good)) else {
            continue;
        };
        #[allow(clippy::cast_precision_loss)]
        {
            num += weight * now.0 as f64;
            den += weight * start.0 as f64;
        }
    }
    (den > 0.0).then(|| num / den)
}

/// Depth view for tests and the API: (price, total qty) per level, best first.
#[must_use]
pub fn depth(world: &World, instrument: Instrument, side: Side) -> Vec<(Money, u32)> {
    let Some(book) = world.books.get(&instrument) else {
        return Vec::new();
    };
    let mut levels: BTreeMap<Money, u32> = BTreeMap::new();
    for o in resting(book, side) {
        *levels.entry(o.limit_price).or_insert(0) += o.remaining;
    }
    let mut v: Vec<(Money, u32)> = levels.into_iter().collect();
    if side == Side::Bid {
        v.reverse();
    }
    v
}
