#![allow(clippy::many_single_char_names)]
//! S0.15b done gate: equal-shortfall and lottery rationing; `SetPolicy` by the
//! System switches the rule and rejects fields the constitution forbids; the
//! Materials split caps each sink; surplus is shared equally at cycle end; a
//! pledge is recorded and closed by the ledger; the Ledger of Contribution
//! records hours exactly and output as attributed.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::{Actor, Event};
use isms_core::explain::RuleId;
use isms_core::ids::{CitizenId, OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder};
use isms_core::needs::{FULL, TENTHS};
use isms_core::policy::Rationing;
use isms_core::test_support::{Harness, WorldBuilder, assert_deterministic};
use isms_core::world::{ContractBody, ContractStatus};
use std::collections::BTreeMap;

fn commune(n: u32) -> Harness {
    WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(n)
        .build()
}

/// Humans never emigrate, so a fixture that runs a whole cycle keeps them.
fn commune_humans(n: u32) -> Harness {
    WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n)
        .build()
}

fn food_meter(h: &mut Harness, id: CitizenId, tenths: u16) {
    let mut needs = h.citizen(id).needs.clone();
    needs.food = tenths;
    h.set_needs(id, needs);
}

fn set_rule(h: &mut Harness, rule: Rationing) {
    let mut policy = h.world.policy.clone();
    policy.rationing = Some(rule);
    let tick = h.world.meta.tick;
    let events = h
        .cmd(Envelope::system(
            Command::SetPolicy {
                policy: Box::new(policy),
            },
            tick,
        ))
        .unwrap();
    assert!(matches!(
        events[0],
        Event::PolicyChanged {
            by: Actor::System,
            ..
        }
    ));
}

fn drew(events: &[Event], rule: RuleId) -> BTreeMap<CitizenId, u32> {
    let mut out = BTreeMap::new();
    for e in events {
        if let Event::Drew {
            citizen,
            goods,
            explain,
        } = e
            && explain.rule == rule
        {
            *out.entry(*citizen).or_insert(0) += goods.get(&Good::Food).copied().unwrap_or(0);
        }
    }
    out
}

/// Shortfalls of 6, 3 and 3 units against a stock of 8.
fn shortage(h: &mut Harness) -> (CitizenId, CitizenId, CitizenId) {
    let (a, b, c) = (CitizenId(0), CitizenId(1), CitizenId(2));
    food_meter(h, a, FULL - 36 * TENTHS);
    food_meter(h, b, FULL - 18 * TENTHS);
    food_meter(h, c, FULL - 18 * TENTHS);
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Food, 8),
    });
    (a, b, c)
}

#[test]
fn equal_shortfall_cuts_everyone_proportionally() {
    let mut h = commune(3);
    set_rule(&mut h, Rationing::EqualShortfall);
    let (a, b, c) = shortage(&mut h);
    let got = drew(&h.tick(), RuleId::StoreDrawEqualShortfall);
    // 8 of 12 requested: floors 4, 2, 2; nothing left over.
    assert_eq!(got.get(&a), Some(&4));
    assert_eq!(got.get(&b), Some(&2));
    assert_eq!(got.get(&c), Some(&2));
}

#[test]
fn equal_shortfall_hands_the_remainder_out_one_each() {
    let mut h = commune(3);
    set_rule(&mut h, Rationing::EqualShortfall);
    let (a, b, c) = shortage(&mut h);
    // Stock 10 of 12: floors 5, 2, 2 (sum 9); one unit left for someone.
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Food, 2),
    });
    let got = drew(&h.tick(), RuleId::StoreDrawEqualShortfall);
    let total: u32 = got.values().sum();
    assert_eq!(total, 10);
    assert!(got[&a] >= 5 && got[&b] >= 2 && got[&c] >= 2);
    assert_eq!(got.values().filter(|q| **q > 0).count(), 3);
}

#[test]
fn lottery_fills_in_drawn_order_and_is_deterministic() {
    let run = |seed: u64| {
        let mut h = WorldBuilder::new("commune")
            .with_preset(|p| p.params.population.collapse_enabled = false)
            .seed(seed)
            .householders(3)
            .build();
        set_rule(&mut h, Rationing::Lottery);
        shortage(&mut h);
        h.tick()
    };
    assert_deterministic(11, run);
    let got = drew(&run(11), RuleId::StoreDrawLottery);
    let mut served: Vec<u32> = got.values().copied().collect();
    served.sort_unstable();
    // Whole requests in drawn order: either 6 then 2 of a 3, or 3, 3 then 2 of the 6.
    assert!(
        served == vec![2, 6] || served == vec![2, 3, 3],
        "{served:?}"
    );
}

#[test]
fn set_policy_switches_the_rationing_rule_next_tick() {
    let mut h = commune(3);
    let (a, ..) = shortage(&mut h);
    assert_eq!(h.world.policy.rationing, Some(Rationing::NeedFirst));
    set_rule(&mut h, Rationing::Lottery);
    assert_eq!(h.world.policy.rationing, Some(Rationing::Lottery));
    let events = h.tick();
    assert!(drew(&events, RuleId::StoreDrawNeedFirst).is_empty());
    assert!(!drew(&events, RuleId::StoreDrawLottery).is_empty());
    let _ = a;
}

#[test]
fn set_policy_rejects_fields_the_constitution_forbids_and_non_system_actors() {
    let mut h = commune(1);
    let mut policy = h.world.policy.clone();
    policy.tax_rate = Some(0.2);
    let err = h
        .cmd(Envelope::system(
            Command::SetPolicy {
                policy: Box::new(policy),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    let mut policy = h.world.policy.clone();
    policy.materials_split = Some(isms_core::policy::MaterialsSplit {
        wares: 0.5,
        machines: 0.5,
        dwellings: 0.5,
    });
    let err = h
        .cmd(Envelope::system(
            Command::SetPolicy {
                policy: Box::new(policy),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    let err = h
        .cmd(Envelope::citizen(
            CitizenId(0),
            Command::SetPolicy {
                policy: Box::new(h.world.policy.clone()),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
}

#[test]
fn materials_split_caps_each_sink() {
    // A Workshop (1 Materials per Wares) and a Machine Shop (2 per Machine)
    // share a store holding 20 Materials under a 0.2 / 0.2 / 0.6 split: the
    // Workshop may use 4 and the Machine Shop 4 this tick, whatever the labor
    // could make; the Builders' 12 stay untouched.
    let mut h = WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(12)
        .org(OrgKind::Collective, "Works")
        .workplace(WorkplaceKind::Workshop, 0, 0)
        .workplace(WorkplaceKind::MachineShop, 0, 0)
        .build();
    let mut policy = h.world.policy.clone();
    policy.materials_split = Some(isms_core::policy::MaterialsSplit {
        wares: 0.2,
        machines: 0.2,
        dwellings: 0.6,
    });
    h.cmd(Envelope::system(
        Command::SetPolicy {
            policy: Box::new(policy),
        },
        0,
    ))
    .unwrap();
    for i in 0..6 {
        h.apply(Event::Assigned {
            workplace: WorkplaceId(0),
            citizen: CitizenId(i),
            contract: None,
        });
        h.apply(Event::Assigned {
            workplace: WorkplaceId(1),
            citizen: CitizenId(i + 6),
            contract: None,
        });
    }
    for i in 0..12 {
        h.cmd(Envelope::citizen(
            CitizenId(i),
            Command::SetLabor {
                allocations: vec![isms_core::world::Allocation {
                    workplace: WorkplaceId(u32::from(i >= 6)),
                    hours: 8,
                    effort: Effort::High,
                }],
            },
            0,
        ))
        .unwrap();
    }
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Materials, 20),
    });
    let events = h.tick();
    let consumed: BTreeMap<WorkplaceId, u32> = events
        .iter()
        .filter_map(|e| match e {
            Event::Produced {
                workplace,
                inputs_consumed,
                ..
            } => Some((
                *workplace,
                inputs_consumed.get(&Good::Materials).copied().unwrap_or(0),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(consumed.get(&WorkplaceId(0)), Some(&4), "Workshop share");
    assert_eq!(
        consumed.get(&WorkplaceId(1)),
        Some(&4),
        "Machine Shop share"
    );
    let left = h.world.store.as_ref().unwrap().stock[&Good::Materials];
    assert_eq!(left, 12, "the Builders' share is untouched");
    // Without a split the same labor takes what it can.
    let mut policy = h.world.policy.clone();
    policy.materials_split = None;
    let tick = h.world.meta.tick;
    h.cmd(Envelope::system(
        Command::SetPolicy {
            policy: Box::new(policy),
        },
        tick,
    ))
    .unwrap();
    let events = h.tick();
    let total: u32 = events
        .iter()
        .filter_map(|e| match e {
            Event::Produced {
                inputs_consumed, ..
            } => inputs_consumed.get(&Good::Materials).copied(),
            _ => None,
        })
        .sum();
    assert!(total > 8, "uncapped, the two shops used {total} of the 12");
}

#[test]
fn surplus_is_shared_equally_and_the_remainder_stays() {
    let mut h = commune_humans(4);
    let cap = h.world.params.pantry[&Good::Food];
    // One pantry sits at the cap, so it cannot take a share.
    h.apply(Event::Seeded {
        holder: Holder::Citizen(CitizenId(3)),
        asset: Asset::Good(Good::Food, cap),
    });
    // Plenty in the store: the cycle's draws by need leave a surplus at 8b.
    h.apply(Event::Seeded {
        holder: Holder::Store,
        asset: Asset::Good(Good::Food, 400),
    });
    let events = h.run_cycle();
    let shares: Vec<(CitizenId, u32, u32, u32)> = events
        .iter()
        .filter_map(|e| match e {
            Event::Drew {
                citizen,
                goods,
                explain,
            } if explain.rule == RuleId::StoreSurplusShare => {
                let input = |name: &str| {
                    explain
                        .inputs
                        .iter()
                        .find_map(|(n, v)| (n == name).then_some(*v))
                };
                let (
                    Some(isms_core::explain::Num::Int(stock)),
                    Some(isms_core::explain::Num::Int(share)),
                ) = (input("stock"), input("share"))
                else {
                    panic!("surplus Explain carries stock and share");
                };
                Some((
                    *citizen,
                    goods[&Good::Food],
                    u32::try_from(stock).unwrap(),
                    u32::try_from(share).unwrap(),
                ))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        shares.len(),
        4,
        "everyone with room gets a share: {shares:?}"
    );
    let (_, _, stock, share) = shares[0];
    assert_eq!(share, stock / 4, "floor(stock / citizens)");
    assert!(shares.iter().all(|s| s.2 == stock && s.3 == share));
    // Each takes min(share, pantry room): the capped pantry takes less.
    assert!(shares.iter().all(|s| s.1 <= share));
    let full = shares.iter().find(|s| s.0 == CitizenId(3)).unwrap();
    assert!(
        full.1 < share,
        "the capped pantry took only its room: {full:?}"
    );
    let taken: u32 = shares.iter().map(|s| s.1).sum();
    let left = h
        .world
        .store
        .as_ref()
        .unwrap()
        .stock
        .get(&Good::Food)
        .copied()
        .unwrap_or(0);
    assert_eq!(left, stock - taken, "the unshared units stay in the store");
}

#[test]
fn pledge_is_recorded_and_closed_by_the_ledger() {
    let mut h = WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(2)
        .org(OrgKind::Collective, "Farm")
        .workplace(WorkplaceKind::Farm, 0, 0)
        .assign(0, 0, 8, Effort::Normal)
        .build();
    let a = CitizenId(0);
    // A pledge of 6 hours per cycle to the Farm's collective for one cycle.
    let events = h
        .cmd(Envelope::citizen(
            a,
            Command::Pledge {
                hours: Some(6),
                goods: None,
                term_cycles: 1,
                to: Some(OrgId(0)),
            },
            0,
        ))
        .unwrap();
    let Event::Pledged { contract, .. } = events[0] else {
        panic!("expected Pledged");
    };
    assert!(matches!(
        h.world.contracts[&contract].body,
        ContractBody::Pledge {
            hours: Some(6),
            goods: None
        }
    ));
    // An empty pledge and a pledge to a missing org are rejected; a citizen who
    // is not in a pledge society gets NotInThisSociety.
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::Pledge {
                hours: None,
                goods: None,
                term_cycles: 1,
                to: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::InvalidQuantity);
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(1)
        .build();
    let err = f
        .cmd(Envelope::citizen(
            a,
            Command::Pledge {
                hours: Some(1),
                goods: None,
                term_cycles: 1,
                to: None,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    // Worker 0 works 8 hours a tick all cycle: the pledge is met at the close.
    let events = h.run_cycle();
    let closed = events.iter().find_map(|e| match e {
        Event::PledgeClosed { contract: k, met } if *k == contract => Some(*met),
        _ => None,
    });
    assert_eq!(closed, Some(Some(true)));
    assert_eq!(h.world.contracts[&contract].status, ContractStatus::Ended);
}

#[test]
fn norms_ledger_records_hours_exactly_and_output_as_attributed() {
    let mut h = WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(3)
        .org(OrgKind::Collective, "Farm")
        .workplace(WorkplaceKind::Farm, 0, 0)
        .assign(0, 0, 8, Effort::Normal)
        .assign(1, 0, 4, Effort::Normal)
        .build();
    let tpc = h.world.params.time.ticks_per_cycle;
    let events = h.run_cycle();
    let ledger = events.iter().find_map(|e| match e {
        Event::NormsLedgerClosed { entries, cycle } => Some((*cycle, entries.clone())),
        _ => None,
    });
    let (cycle, entries) = ledger.expect("the ledger closed");
    assert_eq!(cycle, 0);
    assert_eq!(entries.len(), 3);
    let by: BTreeMap<CitizenId, _> = entries.iter().map(|e| (e.citizen, e)).collect();
    assert_eq!(by[&CitizenId(0)].tick_hours, 8 * tpc);
    assert_eq!(by[&CitizenId(1)].tick_hours, 4 * tpc);
    assert_eq!(by[&CitizenId(2)].tick_hours, 0);
    assert!(by[&CitizenId(0)].met_norm, "8 h/day meets the 6 h norm");
    assert!(!by[&CitizenId(1)].met_norm);
    assert!(!by[&CitizenId(2)].met_norm);
    // Output as attributed: the sum of the phase-4 attributed figures.
    let attributed: f64 = events
        .iter()
        .filter_map(|e| match e {
            Event::Produced { per_worker, .. } => Some(
                per_worker
                    .iter()
                    .filter(|w| w.citizen == CitizenId(0))
                    .map(|w| w.attributed_output)
                    .sum::<f64>(),
            ),
            _ => None,
        })
        .sum();
    assert!((by[&CitizenId(0)].attributed - attributed).abs() < 1e-9);
    assert!(by[&CitizenId(0)].attributed > 0.0);
    // The record folds into the citizen and the accumulators reset after 8i.
    let r = &h.citizen(CitizenId(0)).contribution;
    assert_eq!(r.cycles, 1);
    assert_eq!(r.norm_met_cycles, 1);
    assert_eq!(r.last_cycle_tick_hours, 8 * tpc);
    assert_eq!(
        h.world.workplaces[&WorkplaceId(0)].workers[&CitizenId(0)].cycle_tick_hours,
        0
    );
    // A market society keeps no ledger.
    let mut f = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .householders(1)
        .build();
    assert!(
        !f.run_cycle()
            .iter()
            .any(|e| matches!(e, Event::NormsLedgerClosed { .. }))
    );
}
