//! Proptest strategies that generate *well-formed* steps: a `Step` names
//! participants by index and amounts by fraction, and is resolved against the
//! live world when applied, so it can never reference a missing id or overdraw.
//! `apply` trusts its input; the strategy, not `apply`, guarantees well-formedness.

use super::harness::Harness;
use crate::event::{CitizenDelta, Event};
use crate::ids::CitizenId;
use crate::kinds::{Effort, Good};
use crate::ledger::{Asset, Holder, Party};
use crate::money::Money;
use crate::world::{Allocation, LaborPlan, StandingPlan, VoteDefault};
use proptest::prelude::*;
use std::collections::BTreeMap;

/// One generated step, resolved at apply time.
#[derive(Clone, Debug)]
pub enum Step {
    SeedGood {
        citizen: usize,
        good: Good,
        qty: u32,
    },
    SeedMoney {
        citizen: usize,
        credits: u32,
    },
    TransferMoney {
        from: usize,
        to: usize,
        frac: f64,
    },
    TransferGood {
        from: usize,
        to: usize,
        good: Good,
        frac: f64,
    },
    SetPlan {
        citizen: usize,
        keep_food: u32,
        keep_balance_credits: u32,
    },
    SetLabor {
        citizen: usize,
        hours: u8,
        effort: Effort,
    },
    /// A `TickResolved` that eats `eat` Food per citizen (capped by pantry) and
    /// nudges the Food meter.
    Tick {
        eat: u32,
        food_meter: u8,
    },
}

pub fn arb_good() -> impl Strategy<Value = Good> {
    prop::sample::select(Good::ALL.to_vec())
}

pub fn arb_effort() -> impl Strategy<Value = Effort> {
    prop::sample::select(vec![Effort::Low, Effort::Normal, Effort::High])
}

/// A step over a society with `n` citizens (indices are taken modulo `n`).
pub fn arb_step(n: usize) -> impl Strategy<Value = Step> {
    let n = n.max(1);
    prop_oneof![
        (0..n, arb_good(), 0u32..50).prop_map(|(citizen, good, qty)| Step::SeedGood {
            citizen,
            good,
            qty
        }),
        (0..n, 0u32..500).prop_map(|(citizen, credits)| Step::SeedMoney { citizen, credits }),
        (0..n, 0..n, 0.0f64..=1.0).prop_map(|(from, to, frac)| Step::TransferMoney {
            from,
            to,
            frac
        }),
        (0..n, 0..n, arb_good(), 0.0f64..=1.0).prop_map(|(from, to, good, frac)| {
            Step::TransferGood {
                from,
                to,
                good,
                frac,
            }
        }),
        (0..n, 0u32..48, 0u32..300).prop_map(|(citizen, keep_food, keep_balance_credits)| {
            Step::SetPlan {
                citizen,
                keep_food,
                keep_balance_credits,
            }
        }),
        (0..n, 0u8..=8, arb_effort()).prop_map(|(citizen, hours, effort)| Step::SetLabor {
            citizen,
            hours,
            effort
        }),
        (0u32..3, 0u8..=100).prop_map(|(eat, food_meter)| Step::Tick { eat, food_meter }),
    ]
}

/// Up to `max_steps` steps over `n` citizens.
pub fn arb_scenario(n: usize, max_steps: usize) -> impl Strategy<Value = Vec<Step>> {
    prop::collection::vec(arb_step(n), 0..=max_steps)
}

/// Resolve a step against the harness into a well-formed event, if it does
/// anything at all (a transfer of zero is skipped).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn resolve(h: &Harness, step: &Step) -> Option<Event> {
    let ids = h.citizen_ids();
    if ids.is_empty() {
        return None;
    }
    let id = |i: usize| ids[i % ids.len()];
    let has_money = h.world.constitution.has_money();
    match step {
        Step::SeedGood { citizen, good, qty } => (*qty > 0).then(|| Event::Seeded {
            holder: Holder::Citizen(id(*citizen)),
            asset: Asset::Good(*good, *qty),
        }),
        Step::SeedMoney { citizen, credits } => {
            (has_money && *credits > 0).then(|| Event::Seeded {
                holder: Holder::Citizen(id(*citizen)),
                asset: Asset::Money(Money::credits(i64::from(*credits))),
            })
        }
        Step::TransferMoney { from, to, frac } => {
            let (a, b) = (id(*from), id(*to));
            let have = h.citizen(a).household.balance;
            let amount = Money((have.0 as f64 * frac).floor() as i64);
            (has_money && a != b && amount.0 > 0).then(|| Event::Transferred {
                from: Party::Citizen(a),
                to: Party::Citizen(b),
                asset: Asset::Money(amount),
                memo: String::new(),
            })
        }
        Step::TransferGood {
            from,
            to,
            good,
            frac,
        } => {
            let (a, b) = (id(*from), id(*to));
            let have = h
                .citizen(a)
                .household
                .pantry
                .get(good)
                .copied()
                .unwrap_or(0);
            let qty = (f64::from(have) * frac).floor() as u32;
            (a != b && qty > 0).then(|| Event::Transferred {
                from: Party::Citizen(a),
                to: Party::Citizen(b),
                asset: Asset::Good(*good, qty),
                memo: "gift".into(),
            })
        }
        Step::SetPlan {
            citizen,
            keep_food,
            keep_balance_credits,
        } => Some(Event::PlanChanged {
            citizen: id(*citizen),
            plan: Box::new(StandingPlan {
                labor: LaborPlan::Explicit,
                keep_food_at_least: *keep_food,
                max_food_price: None,
                buy_wares_when: None,
                keep_balance_at_least: Money::credits(i64::from(*keep_balance_credits)),
                standing_orders: Vec::new(),
                vote_default: VoteDefault::Abstain,
            }),
        }),
        Step::SetLabor {
            citizen,
            hours,
            effort,
        } => {
            // No workplaces exist yet in this card; an allocation to a missing
            // workplace is still a well-formed event for `apply` (it mirrors nothing).
            let wp = h.world.workplaces.keys().next().copied();
            Some(Event::LaborSet {
                citizen: id(*citizen),
                allocations: wp
                    .map(|workplace| {
                        vec![Allocation {
                            workplace,
                            hours: *hours,
                            effort: *effort,
                        }]
                    })
                    .unwrap_or_default(),
            })
        }
        Step::Tick { eat, food_meter } => {
            let tick = h.world.meta.tick;
            let cycle = h.world.cycle_of(tick);
            let deltas = h
                .world
                .citizens
                .values()
                .map(|c| {
                    let have = c.household.pantry.get(&Good::Food).copied().unwrap_or(0);
                    let mut needs = c.needs.clone();
                    needs.food = *food_meter;
                    CitizenDelta {
                        citizen: c.id,
                        needs,
                        food_eaten: (*eat).min(have),
                        wares_consumed: 0,
                        output_mult: 1.0,
                        budget: c.labor.budget,
                        fatigue_debt: c.labor.fatigue_debt,
                        consecutive_high_effort_cycles: c.labor.consecutive_high_effort_cycles,
                        skill: BTreeMap::new(),
                    }
                })
                .collect();
            Some(Event::TickResolved {
                tick,
                cycle,
                price_index: None,
                citizen_deltas: deltas,
                workplace_deltas: Vec::new(),
            })
        }
    }
}

/// Apply a scenario to a harness, resolving each step against the live state.
pub fn run_scenario(h: &mut Harness, steps: &[Step]) {
    for s in steps {
        if let Some(e) = resolve(h, s) {
            h.apply(e);
        }
    }
}

/// A convenience for tests that want a citizen id by index.
#[must_use]
pub fn nth(h: &Harness, i: usize) -> CitizenId {
    h.citizen_ids()[i]
}
