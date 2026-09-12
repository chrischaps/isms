#![allow(clippy::many_single_char_names)]

//! S0.8 done gate: price–time priority, partial fills, escrow release on
//! cancel/expiry/fill, no fill at a worse price than the limit, VWAP and index,
//! the scripted 20-order golden tape, and conservation under arbitrary orders.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::OrderId;
use isms_core::kinds::{Good, OrgKind};
use isms_core::ledger::{Asset, Party};
use isms_core::market::{depth, last_price, price_index};
use isms_core::money::Money;
use isms_core::test_support::strategies::{arb_cmd_scenario, nth, run_cmd_scenario};
use isms_core::test_support::{Harness, WorldBuilder, check_golden};
use isms_core::world::{Instrument, Side};
use proptest::prelude::*;

const FOOD: Instrument = Instrument::Good(Good::Food);

fn market(n: u32) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n)
        .org(OrgKind::Firm, "Legacy Mill");
    for i in 0..n as usize {
        b = b.pantry(i, Good::Food, 20);
    }
    b.build()
}

fn order(side: Side, qty: u32, cents: i64) -> Command {
    Command::PlaceOrder {
        instrument: FOOD,
        side,
        qty,
        limit_price: Money::cents(cents),
        expires_tick: None,
    }
}

fn trades(events: &[Event]) -> Vec<(u32, Money)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Trade { qty, price, .. } => Some((*qty, *price)),
            _ => None,
        })
        .collect()
}

#[test]
fn bids_and_asks_match_at_the_resting_price_with_price_time_priority() {
    let mut h = market(3);
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    // resting asks: a @1.34 (first), b @1.32, a @1.32 (later)
    h.cmd(Envelope::citizen(a, order(Side::Ask, 5, 134), 0))
        .unwrap();
    h.cmd(Envelope::citizen(b, order(Side::Ask, 5, 132), 0))
        .unwrap();
    h.cmd(Envelope::citizen(a, order(Side::Ask, 5, 132), 0))
        .unwrap();
    assert_eq!(
        depth(&h.world, FOOD, Side::Ask),
        vec![(Money::cents(132), 10), (Money::cents(134), 5)]
    );
    assert_eq!(
        h.citizen(a).household.pantry[&Good::Food],
        10,
        "asks escrow goods"
    );
    // a bid for 8 @ 1.40 fills 5 from b (older at 1.32), then 3 from a's 1.32; never touches 1.34
    let events = h
        .cmd(Envelope::citizen(c, order(Side::Bid, 8, 140), 0))
        .unwrap();
    assert_eq!(
        trades(&events),
        vec![(5, Money::cents(132)), (3, Money::cents(132))]
    );
    assert_eq!(h.citizen(c).household.pantry[&Good::Food], 28);
    assert_eq!(
        h.citizen(c).household.balance,
        Money::credits(1000) - Money::cents(8 * 132),
        "paid the resting price, refund of the improvement"
    );
    assert_eq!(
        h.citizen(b).household.balance,
        Money::credits(1000) + Money::cents(5 * 132)
    );
    assert_eq!(
        depth(&h.world, FOOD, Side::Ask),
        vec![(Money::cents(132), 2), (Money::cents(134), 5)]
    );
    assert!(
        h.world.escrow.len() == 2,
        "two resting asks remain in escrow"
    );
    assert_eq!(last_price(&h.world, FOOD), Some(Money::cents(132)));
    h.check();
}

#[test]
fn a_resting_bid_is_filled_by_an_incoming_ask_at_the_bid_price() {
    let mut h = market(2);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(a, order(Side::Bid, 10, 130), 0))
        .unwrap();
    assert_eq!(
        h.citizen(a).household.balance,
        Money::credits(1000) - Money::cents(1300)
    );
    let events = h
        .cmd(Envelope::citizen(b, order(Side::Ask, 4, 125), 0))
        .unwrap();
    assert_eq!(trades(&events), vec![(4, Money::cents(130))]);
    assert_eq!(
        h.citizen(b).household.balance,
        Money::credits(1000) + Money::cents(520)
    );
    assert_eq!(
        depth(&h.world, FOOD, Side::Bid),
        vec![(Money::cents(130), 6)]
    );
    assert_eq!(
        h.world.escrow[&isms_core::world::EscrowKey::Order(OrderId(0))],
        Asset::Money(Money::cents(780))
    );
    h.check();
}

#[test]
fn placement_validates_funds_goods_caps_and_capability() {
    let mut h = market(2);
    let a = nth(&h, 0);
    let r = h.cmd_dry(Envelope::citizen(a, order(Side::Ask, 21, 100), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientGoods);
    let r = h.cmd_dry(Envelope::citizen(a, order(Side::Bid, 1000, 200), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientFunds);
    let r = h.cmd_dry(Envelope::citizen(a, order(Side::Bid, 29, 100), 0));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::PantryFull,
        "20 held + 29 bid > 48"
    );
    h.cmd(Envelope::citizen(a, order(Side::Bid, 20, 100), 0))
        .unwrap();
    let r = h.cmd_dry(Envelope::citizen(a, order(Side::Bid, 9, 100), 0));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::PantryFull,
        "open bids count"
    );
    let r = h.cmd_dry(Envelope::citizen(a, order(Side::Bid, 0, 100), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InvalidQuantity);
    let r = h.cmd_dry(Envelope::citizen(a, order(Side::Bid, 1, 0), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InvalidPrice);
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::PlaceOrder {
            instrument: Instrument::Share(isms_core::ids::OrgId(7)),
            side: Side::Bid,
            qty: 1,
            limit_price: Money::cents(1),
            expires_tick: None,
        },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::UnknownOrg,
        "share instruments need a real org (S0.10b)"
    );
    let commune = WorldBuilder::new("commune").humans(1).build();
    let r = commune.cmd_dry(Envelope::citizen(
        nth(&commune, 0),
        order(Side::Bid, 1, 1),
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotInThisSociety);
}

#[test]
fn cancel_and_expiry_release_exactly_the_escrow() {
    let mut h = market(2);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    // The default plan bids for Food each tick (S0.9); switch it off here so
    // the resting ask can only expire.
    for who in [a, b] {
        let mut plan = h.citizen(who).plan.clone();
        plan.keep_food_at_least = 0;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(plan),
        });
    }
    h.cmd(Envelope::citizen(a, order(Side::Bid, 10, 130), 0))
        .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::CancelOrder { order: OrderId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotParty);
    let events = h
        .cmd(Envelope::citizen(
            a,
            Command::CancelOrder { order: OrderId(0) },
            0,
        ))
        .unwrap();
    assert!(
        matches!(events[0], Event::OrderCancelled { released: Asset::Money(m), .. } if m == Money::cents(1300))
    );
    assert_eq!(h.citizen(a).household.balance, Money::credits(1000));
    assert!(h.world.escrow.is_empty());
    // default expiry: end of next cycle (tick 47 from cycle 0); it expires in phase 6 of tick 47
    h.cmd(Envelope::citizen(b, order(Side::Ask, 3, 150), 0))
        .unwrap();
    assert_eq!(h.citizen(b).household.pantry[&Good::Food], 17);
    h.check_every_step = false;
    let mut expired_at = None;
    for _ in 0..48 {
        let t = h.world.meta.tick;
        if h.tick().iter().any(|e| {
            matches!(
                e,
                Event::OrderExpired {
                    order: OrderId(1),
                    ..
                }
            )
        }) {
            expired_at = Some(t);
            break;
        }
    }
    assert_eq!(expired_at, Some(47));
    // b ate 48 Food over 48 ticks but only had 17 + 3 released; check the count
    assert!(h.world.escrow.is_empty());
    h.check();
}

#[test]
fn vwap_and_price_index_are_recorded_at_the_tick() {
    let mut h = market(3);
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    assert_eq!(
        last_price(&h.world, FOOD),
        Some(Money::cents(130)),
        "start price anchors last price"
    );
    assert!(
        (price_index(&h.world).unwrap() - 1.0).abs() < 1e-12,
        "at start prices the index is 1"
    );
    h.cmd(Envelope::citizen(a, order(Side::Ask, 4, 140), 0))
        .unwrap();
    h.cmd(Envelope::citizen(b, order(Side::Ask, 4, 160), 0))
        .unwrap();
    h.cmd(Envelope::citizen(c, order(Side::Bid, 8, 160), 0))
        .unwrap();
    let events = h.tick();
    let Some(Event::TickResolved {
        vwap,
        price_index: idx,
        ..
    }) = events.last()
    else {
        unreachable!()
    };
    assert_eq!(vwap[&FOOD], Money::cents(150));
    // Food at 1.60 vs 1.30, other goods at start: index = (1.0*160 + 1.0*400 + 2.0*900) / (130 + 400 + 1800)
    let expected = (160.0 + 400.0 + 1800.0) / (130.0 + 400.0 + 1800.0);
    assert!((idx.unwrap() - expected).abs() < 1e-9, "{idx:?}");
    assert!((h.world.price_index.unwrap() - expected).abs() < 1e-9);
    assert_eq!(h.world.books[&FOOD].tick_volume, 0, "accumulators reset");
    // a tick without trades carries no vwap for the instrument
    let events = h.tick();
    let Some(Event::TickResolved { vwap, .. }) = events.last() else {
        unreachable!()
    };
    assert!(vwap.is_empty());
    h.check();
}

#[test]
fn golden_twenty_order_tape() {
    let mut h = market(4);
    let ids: Vec<_> = (0..4).map(|i| nth(&h, i)).collect();
    let script: [(usize, Side, u32, i64); 20] = [
        (0, Side::Ask, 5, 134),
        (1, Side::Ask, 5, 132),
        (2, Side::Bid, 3, 131),
        (3, Side::Bid, 6, 133),
        (0, Side::Ask, 2, 130),
        (1, Side::Bid, 4, 135),
        (2, Side::Ask, 7, 129),
        (3, Side::Ask, 1, 140),
        (0, Side::Bid, 2, 128),
        (1, Side::Bid, 5, 132),
        (2, Side::Bid, 1, 134),
        (3, Side::Ask, 3, 127),
        (0, Side::Ask, 4, 136),
        (1, Side::Ask, 2, 126),
        (2, Side::Bid, 6, 137),
        (3, Side::Bid, 2, 125),
        (0, Side::Bid, 3, 130),
        (1, Side::Ask, 3, 133),
        (2, Side::Ask, 2, 131),
        (3, Side::Bid, 4, 139),
    ];
    let mut log = h.log.clone();
    for (who, side, qty, cents) in script {
        if let Ok(events) = h.cmd(Envelope::citizen(ids[who], order(side, qty, cents), 0)) {
            log.extend(events);
        }
    }
    let tape = trades(&log);
    assert!(tape.len() >= 8, "{tape:?}");
    check_golden("s0_8_order_tape", &log);
    h.check();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// After any order sequence: conservation, no negative holdings, no fill at
    /// a worse price than either limit, and escrow equals the resting orders.
    #[test]
    fn orders_conserve_and_respect_limits(steps in arb_cmd_scenario(4, 80)) {
        let mut h = market(4);
        h.check_every_step = false;
        let before = h.log.len();
        run_cmd_scenario(&mut h, &steps);
        h.check();
        // every trade was within both orders' limits
        let mut orders = std::collections::BTreeMap::new();
        for e in &h.log[before..] {
            match e {
                Event::OrderPlaced { order, .. } => { orders.insert(order.id, *order); }
                Event::Trade { buy_order, sell_order, price, .. } => {
                    let b = orders[buy_order];
                    let s = orders[sell_order];
                    prop_assert!(*price <= b.limit_price && *price >= s.limit_price);
                }
                _ => {}
            }
        }
        // escrow matches the resting book exactly
        for book in h.world.books.values() {
            for o in book.orders.values() {
                let key = isms_core::world::EscrowKey::Order(o.id);
                prop_assert_eq!(h.world.escrow.get(&key).copied(), Some(isms_core::market::escrow_of(o)));
            }
        }
        prop_assert_eq!(h.world.escrow.len(), h.world.books.values().map(|b| b.orders.len()).sum::<usize>() + h.world.offers.values().filter(|o| matches!(o.body, isms_core::world::OfferBody::Sale { .. })).count());
        for c in h.world.citizens.values() {
            prop_assert!(!c.household.balance.is_negative());
        }
        // cancelling everything drains the escrow
        let open: Vec<(OrderId, Party)> = h.world.books.values().flat_map(|b| b.orders.values().map(|o| (o.id, o.owner))).collect();
        for (id, owner) in open {
            if let Party::Citizen(c) = owner {
                h.cmd(Envelope::citizen(c, Command::CancelOrder { order: id }, 0)).unwrap();
            }
        }
        let sales: Vec<(isms_core::ids::OfferId, Party)> = h.world.offers.values().map(|o| (o.id, o.by)).collect();
        for (id, by) in sales {
            if let Party::Citizen(c) = by {
                let _ = h.cmd(Envelope::citizen(c, Command::CancelSale { offer: id }, 0));
                let _ = h.cmd(Envelope::citizen(c, Command::RemoveWanted { offer: id }, 0));
            }
        }
        prop_assert!(h.world.escrow.is_empty());
        h.check();
    }
}
