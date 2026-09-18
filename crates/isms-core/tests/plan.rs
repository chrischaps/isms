//! S0.9 done gate: the plan runs identically with or without presence, respects
//! the saving floor, refreshes standing orders; dormancy at exactly 7 absent
//! cycles with a freeze and a return on the next Seen; the away digest.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::OrgId;
use isms_core::kinds::{ClientKind, Good, OrgKind};
use isms_core::ledger::Party;
use isms_core::money::Money;
use isms_core::plan::{away_digest, default_max_price};
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{
    BuyRule, Instrument, LaborPlan, Refresh, Side, StandingOrder, StandingPlan, VoteDefault,
};

/// A Freeport with a legacy Mill holding Food and a manager who has posted a big ask.
fn shop(humans: u32, ask_cents: i64) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(humans)
        .householders(1)
        .org(OrgKind::Firm, "Legacy Mill")
        .org_inventory(0, Good::Food, 2000)
        .org_inventory(0, Good::Wares, 500);
    for i in 0..humans as usize {
        b = b.pantry(i, Good::Food, 10);
    }
    let mut h = b.build();
    let mgr = nth(&h, humans as usize);
    // Everyone joins with the default plan (keep Food at 24); switch it off so
    // only the citizen under test acts, once the test sets their plan.
    for who in h.citizen_ids() {
        let mut p = h.citizen(who).plan.clone();
        p.keep_food_at_least = 0;
        p.buy_wares_when = None;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(p),
        });
    }
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(mgr),
    });
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::PlaceOrder {
                instrument: Instrument::Good(Good::Food),
                side: Side::Ask,
                qty: 1000,
                limit_price: Money::cents(ask_cents),
                expires_tick: Some(2000),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::PlaceOrder {
                instrument: Instrument::Good(Good::Wares),
                side: Side::Ask,
                qty: 300,
                limit_price: Money::cents(410),
                expires_tick: Some(2000),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    h
}

fn plan(keep_food: u32, keep_balance: i64) -> StandingPlan {
    StandingPlan {
        labor: LaborPlan::Explicit,
        keep_food_at_least: keep_food,
        max_food_price: None,
        buy_wares_when: Some(BuyRule {
            comfort_below: 60,
            balance_above: Money::credits(200),
            max_price: None,
        }),
        keep_balance_at_least: Money::credits(keep_balance),
        standing_orders: Vec::new(),
        vote_default: VoteDefault::Abstain,
    }
}

#[test]
fn default_max_price_is_last_times_1_25_to_the_cent() {
    assert_eq!(default_max_price(Money::cents(132)), Money::cents(165));
    assert_eq!(
        default_max_price(Money::cents(130)),
        Money::cents(163),
        "162.5 rounds up"
    );
    assert_eq!(default_max_price(Money::cents(1)), Money::cents(1));
}

#[test]
fn the_plan_buys_the_food_shortfall_at_or_below_the_limit() {
    let mut h = shop(1, 132);
    let me = nth(&h, 0);
    h.cmd(Envelope::citizen(
        me,
        Command::SetStandingPlan {
            plan: Box::new(plan(24, 100)),
        },
        0,
    ))
    .unwrap();
    let events = h.tick();
    let placed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::OrderPlaced { .. }))
        .collect();
    assert_eq!(placed.len(), 1);
    let Event::OrderPlaced { order, .. } = placed[0] else {
        unreachable!()
    };
    assert_eq!(order.owner, Party::Citizen(me));
    assert_eq!(
        (order.qty, order.limit_price),
        (14, Money::cents(132)),
        "shortfall 24 - 10, placed at the best resting ask (limit 1.63)"
    );
    assert_eq!(order.source, isms_core::world::OrderSource::Standing);
    assert!(
        events.iter().any(
            |e| matches!(e, Event::Trade { qty: 14, price, .. } if *price == Money::cents(132))
        )
    );
    // ate 1 and bought 14: pantry 23; next tick it tops up by 1 again
    assert_eq!(h.citizen(me).household.pantry[&Good::Food], 23);
    let events = h.tick();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Trade { qty: 1, .. }))
    );
    h.check();
}

#[test]
fn the_saving_floor_caps_the_bid_and_the_wares_rule_fires() {
    let mut h = shop(1, 132);
    let me = nth(&h, 0);
    // balance 1000, floor 990: only 10 credits to spend at the 1.32 ask -> 7 Food
    h.cmd(Envelope::citizen(
        me,
        Command::SetStandingPlan {
            plan: Box::new(plan(48, 990)),
        },
        0,
    ))
    .unwrap();
    let events = h.tick();
    let Some(Event::OrderPlaced { order, .. }) = events
        .iter()
        .find(|e| matches!(e, Event::OrderPlaced { .. }))
    else {
        panic!()
    };
    assert_eq!(order.qty, 7);
    assert!(h.citizen(me).household.balance >= Money::credits(990));
    // Wares: comfort must fall below 60 first (unhoused: -2/tick), and balance > 200 must hold
    let mut h = shop(1, 132);
    let me = nth(&h, 0);
    h.cmd(Envelope::citizen(
        me,
        Command::SetStandingPlan {
            plan: Box::new(plan(0, 0)),
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    let mut bought = None;
    for t in 0..40 {
        let events = h.tick();
        if let Some(Event::Trade { qty, .. }) =
            events.iter().find(|e| matches!(e, Event::Trade { .. }))
        {
            bought = Some((t, *qty));
            break;
        }
    }
    // comfort 100 -> 58 after 21 ticks (tick index 20); wanted = ceil((100-58)/6) = 7
    assert_eq!(bought, Some((21, 7)));
    h.check();
}

#[test]
fn the_plan_runs_the_same_whether_or_not_the_citizen_was_present() {
    let run = |present: bool| {
        let mut h = shop(2, 132);
        let me = nth(&h, 0);
        h.cmd(Envelope::citizen(
            me,
            Command::SetStandingPlan {
                plan: Box::new(plan(24, 100)),
            },
            0,
        ))
        .unwrap();
        h.check_every_step = false;
        for t in 0..24u32 {
            if present && t % 3 == 0 {
                h.cmd(Envelope::citizen(me, Command::Seen, t).via(ClientKind::Web))
                    .unwrap();
            }
            h.tick();
        }
        (
            h.citizen(me).household.pantry.clone(),
            h.citizen(me).household.balance,
        )
    };
    assert_eq!(run(true), run(false));
}

#[test]
fn standing_orders_are_refreshed_each_tick_and_each_cycle() {
    let mut h = shop(1, 200); // ask far above the standing bid: it never fills
    let me = nth(&h, 0);
    let mut p = plan(0, 0);
    p.standing_orders = vec![
        StandingOrder {
            instrument: Instrument::Good(Good::Food),
            side: Side::Bid,
            qty: 10,
            limit_price: Money::cents(130),
            refresh: Refresh::EachTick,
        },
        StandingOrder {
            instrument: Instrument::Good(Good::Wares),
            side: Side::Bid,
            qty: 2,
            limit_price: Money::cents(100),
            refresh: Refresh::EachCycle,
        },
    ];
    h.cmd(Envelope::citizen(
        me,
        Command::SetStandingPlan { plan: Box::new(p) },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    for t in 0..30u32 {
        let events = h.tick();
        let placed = events
            .iter()
            .filter(|e| matches!(e, Event::OrderPlaced { .. }))
            .count();
        let expected = if t % 24 == 0 { 2 } else { 1 };
        assert_eq!(placed, expected, "tick {t}");
        let mine = h
            .world
            .books
            .values()
            .flat_map(|b| b.orders.values())
            .filter(|o| o.owner == Party::Citizen(me))
            .count();
        // the each-cycle Wares bid expires on the last tick of its cycle
        let resting = if t % 24 == 23 { 1 } else { 2 };
        assert_eq!(mine, resting, "tick {t}");
    }
    h.check();
}

#[test]
fn set_standing_plan_is_validated_against_the_constitution() {
    let h = WorldBuilder::new("commune").humans(2).build();
    let me = nth(&h, 0);
    let mut p = plan(24, 0);
    p.buy_wares_when = None;
    p.standing_orders = vec![StandingOrder {
        instrument: Instrument::Good(Good::Food),
        side: Side::Bid,
        qty: 1,
        limit_price: Money::cents(1),
        refresh: Refresh::EachTick,
    }];
    assert_eq!(
        h.cmd_dry(Envelope::citizen(
            me,
            Command::SetStandingPlan {
                plan: Box::new(p.clone())
            },
            0
        ))
        .unwrap_err()
        .code,
        RejectCode::NotInThisSociety
    );
    p.standing_orders.clear();
    p.keep_balance_at_least = Money::credits(1);
    assert_eq!(
        h.cmd_dry(Envelope::citizen(
            me,
            Command::SetStandingPlan {
                plan: Box::new(p.clone())
            },
            0
        ))
        .unwrap_err()
        .code,
        RejectCode::NotInThisSociety
    );
    p.keep_balance_at_least = Money::ZERO;
    p.labor = LaborPlan::FollowNorm;
    p.vote_default = VoteDefault::Follow(nth(&h, 1));
    assert!(
        h.cmd_dry(Envelope::citizen(
            me,
            Command::SetStandingPlan {
                plan: Box::new(p.clone())
            },
            0
        ))
        .is_ok()
    );
    let f = WorldBuilder::new("freeport").humans(2).build();
    assert_eq!(
        f.cmd_dry(Envelope::citizen(
            nth(&f, 0),
            Command::SetStandingPlan { plan: Box::new(p) },
            0
        ))
        .unwrap_err()
        .code,
        RejectCode::NotInThisSociety,
        "no norm, no votes in Freeport"
    );
}

#[test]
fn dormancy_after_exactly_seven_absent_cycles_and_return_on_seen() {
    let mut h = shop(2, 132);
    let (me, other) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(
        me,
        Command::SetStandingPlan {
            plan: Box::new(plan(24, 0)),
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    // present at tick 0; cycles 0..6 absent -> dormant at the end of cycle 6 (tick 167)
    let mut went_dormant = None;
    for t in 0..24 * 8 {
        let events = h.tick();
        if events
            .iter()
            .any(|e| matches!(e, Event::CitizenDormant { citizen } if *citizen == me))
        {
            went_dormant = Some(t);
            break;
        }
    }
    assert_eq!(went_dormant, Some(24 * 7 - 1));
    assert!(h.citizen(me).dormant);
    // (the householder emigrates under the fixture floor of 0; that is S0.12a, not dormancy)
    assert!(h.citizen(other).dormant, "an equally absent human does");
    let open_mine = h
        .world
        .books
        .values()
        .flat_map(|b| b.orders.values())
        .filter(|o| o.owner == Party::Citizen(me))
        .count();
    assert_eq!(open_mine, 0, "open orders were cancelled");
    let frozen = (h.citizen(me).needs.clone(), h.citizen(me).household.clone());
    h.run_cycle();
    assert_eq!(
        (h.citizen(me).needs.clone(), h.citizen(me).household.clone()),
        frozen,
        "meters and balance do not move"
    );
    let events = h
        .cmd(Envelope::citizen(me, Command::Seen, h.world.meta.tick).via(ClientKind::Web))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::CitizenReturned { .. }))
    );
    assert!(!h.citizen(me).dormant);
    h.tick();
    assert_ne!(h.citizen(me).needs, frozen.0);
    h.check();
}

#[test]
fn away_digest_lists_the_events_that_touch_the_citizen() {
    let mut h = shop(2, 132);
    let (me, other) = (nth(&h, 0), nth(&h, 1));
    let since = h.log.len();
    h.cmd(Envelope::citizen(
        other,
        Command::Transfer {
            to: Party::Citizen(me),
            asset: isms_core::ledger::Asset::Money(Money::credits(5)),
            memo: "hi".into(),
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        other,
        Command::SetStandingPlan {
            plan: Box::new(plan(24, 0)),
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        me,
        Command::SetStandingPlan {
            plan: Box::new(plan(24, 0)),
        },
        0,
    ))
    .unwrap();
    h.tick();
    let recent = &h.log[since..];
    let mine = away_digest(me, recent);
    let kinds: Vec<&str> = mine.iter().map(|i| recent[*i].kind()).collect();
    assert!(kinds.contains(&"Transferred"));
    assert!(kinds.contains(&"PlanChanged"));
    assert!(kinds.contains(&"OrderPlaced") && kinds.contains(&"Trade"));
    assert!(!kinds.contains(&"TickResolved"));
    assert_eq!(
        kinds.iter().filter(|k| **k == "PlanChanged").count(),
        1,
        "only my own plan change"
    );
}

#[test]
fn a_standing_order_names_an_instrument_that_exists() {
    // Q108 (D1): a share order on an org that does not exist is refused when
    // the plan is set, as PlaceOrder refuses it, not silently every cycle.
    let h = shop(1, 132);
    let me = nth(&h, 0);
    let order = |org: u32| StandingOrder {
        instrument: Instrument::Share(OrgId(org)),
        side: Side::Bid,
        qty: 1,
        limit_price: Money::cents(100),
        refresh: Refresh::EachCycle,
    };
    let mut p = plan(0, 0);
    p.standing_orders = vec![order(999_999)];
    let r = h.cmd_dry(Envelope::citizen(
        me,
        Command::SetStandingPlan {
            plan: Box::new(p.clone()),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::UnknownOrg);
    p.standing_orders = vec![order(0)];
    assert!(
        h.cmd_dry(Envelope::citizen(
            me,
            Command::SetStandingPlan { plan: Box::new(p) },
            0
        ))
        .is_ok(),
        "the Legacy Mill exists and keeps a share registry"
    );
}
