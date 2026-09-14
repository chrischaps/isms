//! Tax and transfer (GDD §6.4, §5 A6 `tax-transfer`; TDD §5.5 step 8b, §5.6,
//! S0.17a). In a tax-transfer society an income tax on each cycle's earnings
//! feeds a treasury, which pays a need floor (a money top-up to the price of
//! the floor's Food, Q81) and houses the unhoused in public
//! dwellings. The legislature (System until Phase 3) sets the rate, the
//! brackets, the floor, the minimum wage and the public dwelling stock as
//! policy. The minimum wage is enforced on employment offers.

use crate::constitution::Redistribution;
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, OrgId};
use crate::kinds::Good;
use crate::ledger::Holder;
use crate::money::Money;
use crate::policy::Policy;
use crate::tick::TickBuilder;
use crate::world::{Instrument, World};

/// Marginal income tax on one cycle's income: everything up to the lowest
/// bracket at `tax_rate`, each slice above a bracket's threshold at that
/// bracket's rate; brackets ascending; an empty list is a flat tax (Q84).
/// Each slice is floored to the cent.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn assess(policy: &Policy, income: Money) -> (Money, Explain) {
    let base = policy.tax_rate.unwrap_or(0.0);
    let mut brackets: Vec<(Money, f64)> = policy
        .tax_brackets
        .as_ref()
        .map(|b| b.iter().map(|x| (x.above, x.rate)).collect())
        .unwrap_or_default();
    brackets.sort_by_key(|b| b.0);
    let mut explain = Explain::new(RuleId::TaxIncome, "sum(slice x rate)", Money::ZERO)
        .input("income", income)
        .input("rate", base);
    let mut tax = 0i64;
    let mut floor = Money::ZERO;
    let mut rate = base;
    for (i, (above, r)) in brackets.iter().enumerate() {
        let top = (*above).min(income);
        if top > floor {
            tax += ((top.0 - floor.0) as f64 * rate).floor() as i64;
        }
        floor = *above;
        rate = *r;
        explain = explain
            .input(bracket_name(i, "above"), *above)
            .input(bracket_name(i, "rate"), *r);
    }
    if income > floor {
        tax += ((income.0 - floor.0) as f64 * rate).floor() as i64;
    }
    explain.result = Money(tax).into();
    (Money(tax), explain)
}

fn bracket_name(i: usize, what: &str) -> &'static str {
    // A closed set of names keeps `Explain` inputs static strings.
    match (i, what) {
        (0, "above") => "bracket_1_above",
        (0, _) => "bracket_1_rate",
        (1, "above") => "bracket_2_above",
        (1, _) => "bracket_2_rate",
        (2, "above") => "bracket_3_above",
        (2, _) => "bracket_3_rate",
        (_, "above") => "bracket_n_above",
        _ => "bracket_n_rate",
    }
}

/// The wage floor an employment offer must meet at `org`: the society's
/// minimum wage (0 or none = no enforcement). Collective agreements (S0.17d)
/// raise it for their firms.
#[must_use]
pub fn wage_floor(world: &World, org: OrgId) -> Money {
    let society = world.policy.minimum_wage.unwrap_or(Money::ZERO);
    crate::union::agreement_for(world, org).map_or(society, |(floor, _)| society.max(floor))
}

/// The need floor in money: the floor's Food units at the last Food price
/// (the trade, else the start price; Q8).
#[must_use]
pub fn floor_money(world: &World, floor_food: u32) -> Money {
    let price = crate::market::last_price(world, Instrument::Good(Good::Food))
        .or_else(|| crate::state_store::list_price(world, Good::Food))
        .unwrap_or(Money::ZERO);
    Money(price.0 * i64::from(floor_food))
}

/// Pay every active citizen whose means (balance plus pantry Food at the
/// Food price) fall short of the floor, largest shortfall first, until
/// `source` runs dry. Shared by the Republic's treasury and the
/// Commonwealth's bank (S0.17c).
pub fn need_floor(b: &mut TickBuilder, source: Holder, floor_food: u32) {
    let floor = floor_money(&b.world, floor_food);
    if floor <= Money::ZERO {
        return;
    }
    let food_price = Money(floor.0 / i64::from(floor_food.max(1)));
    let mut short: Vec<(Money, CitizenId)> = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .filter_map(|c| {
            let pantry_food = c.household.pantry.get(&Good::Food).copied().unwrap_or(0);
            let means = c.household.balance + Money(food_price.0 * i64::from(pantry_food));
            (means < floor).then(|| (floor - means, c.id))
        })
        .collect();
    short.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (shortfall, citizen) in short {
        let available = money_at(&b.world, source);
        let amount = shortfall.min(available);
        if amount <= Money::ZERO {
            break;
        }
        let c = &b.world.citizens[&citizen];
        let explain = Explain::new(
            RuleId::NeedFloorTransfer,
            "min(floor - (balance + pantry Food x price), source)",
            amount,
        )
        .input("floor_food", floor_food)
        .input("food_price", food_price)
        .input("balance", c.household.balance)
        .input(
            "pantry_food",
            c.household.pantry.get(&Good::Food).copied().unwrap_or(0),
        )
        .input("source", available);
        b.emit(Event::NeedFloorPaid {
            citizen,
            amount,
            from: source,
            explain,
        });
    }
}

fn money_at(world: &World, holder: Holder) -> Money {
    match holder {
        Holder::Treasury => world.treasury,
        Holder::Org(o) => world.orgs.get(&o).map_or(Money::ZERO, |o| o.treasury),
        Holder::StateStock => world.state_stock.as_ref().map_or(Money::ZERO, |s| s.till),
        Holder::Citizen(c) => world
            .citizens
            .get(&c)
            .map_or(Money::ZERO, |c| c.household.balance),
        _ => Money::ZERO,
    }
}

/// Step 8b for tax-transfer systems: assess every citizen's income since the
/// last assessment (even at zero tax, so the base resets), then the floor,
/// then public dwellings (the shared assignment step handles those).
pub fn cycle_end_8b_tax(b: &mut TickBuilder) {
    if b.rules.capabilities.redistribution != Redistribution::TaxTransfer {
        return;
    }
    let policy = b.world.policy.clone();
    let earners: Vec<(CitizenId, Money)> = b
        .world
        .citizens
        .values()
        .filter(|c| !c.dormant && c.taxable_income > Money::ZERO)
        .map(|c| (c.id, c.taxable_income))
        .collect();
    for (citizen, income) in earners {
        let (tax, explain) = assess(&policy, income);
        b.emit(Event::TaxAssessed {
            citizen,
            income,
            tax,
            explain,
        });
    }
    if let Some(floor_food) = policy.need_floor_food
        && floor_food > 0
    {
        need_floor(b, Holder::Treasury, floor_food);
    }
}
