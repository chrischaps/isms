//! Shares, dividends and the Freeport scoreboard (GDD §6.1, §10; TDD §5.3 org
//! rows, step 8e, ADR-0005, T21). Shares are a registry per firm; new shares
//! land in the org's own holdings and are sold on the book or by direct sale.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, OrgId};
use crate::kinds::Good;
use crate::ledger::Party;
use crate::market::last_price;
use crate::money::Money;
use crate::orgs::controlling_owner;
use crate::tick::TickBuilder;
use crate::world::{Instrument, Org, Ownership, ShareHolder, World};

/// The share holder a party acts as: a citizen's own account, or the org itself.
#[must_use]
pub fn holder_of(party: Party, org: OrgId) -> Option<ShareHolder> {
    match party {
        Party::Citizen(c) => Some(ShareHolder::Citizen(c)),
        Party::Org(o) if o == org => Some(ShareHolder::OrgSelf),
        Party::Org(_) => None,
    }
}

/// Shares a party holds in `org`.
#[must_use]
pub fn shares_held(world: &World, party: Party, org: OrgId) -> u64 {
    let Some(o) = world.orgs.get(&org) else {
        return 0;
    };
    let Some(h) = holder_of(party, org) else {
        return 0;
    };
    crate::ledger::shares_of(&o.ownership, h)
}

fn firm(world: &World, org: OrgId) -> Result<&Org, Reject> {
    let o = world
        .orgs
        .get(&org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
    if !matches!(o.ownership, Ownership::Shares { .. }) {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} has no share registry", o.name),
        ));
    }
    Ok(o)
}

fn require_control<'w>(
    world: &'w World,
    envelope: &Envelope<Command>,
    org: OrgId,
) -> Result<&'w Org, Reject> {
    let actor = acting_citizen(world, envelope)?;
    let o = firm(world, org)?;
    if controlling_owner(o) != Some(actor.id) {
        return Err(Reject::new(
            RejectCode::NotControllingOwner,
            format!("{} does not control {}", actor.id, o.name),
        ));
    }
    Ok(o)
}

/// `IssueShares`: the controlling owner mints shares into the org's own holdings.
pub fn issue_shares(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    qty: u64,
) -> Result<Vec<Event>, Reject> {
    require_control(world, envelope, org)?;
    if qty == 0 {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "quantity must be positive",
        ));
    }
    Ok(vec![Event::SharesIssued { org, qty }])
}

/// Shares held by citizens (not the org itself): what a dividend pays on.
#[must_use]
pub fn citizen_held(org: &Org) -> u64 {
    match &org.ownership {
        Ownership::Shares { holdings, .. } => holdings
            .iter()
            .filter(|(h, _)| matches!(h, ShareHolder::Citizen(_)))
            .map(|(_, q)| *q)
            .sum(),
        _ => 0,
    }
}

/// `DeclareDividend`: the controlling owner declares a per-share payout from
/// the treasury, paid at this cycle's end (8e). Org-held shares earn nothing.
pub fn declare_dividend(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    per_share: Money,
) -> Result<Vec<Event>, Reject> {
    let o = require_control(world, envelope, org)?;
    if per_share <= Money::ZERO {
        return Err(Reject::new(
            RejectCode::InvalidPrice,
            "dividend must be positive",
        ));
    }
    if o.declared_dividend.is_some() {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            "a dividend is already declared this cycle",
        ));
    }
    let total = Money(per_share.0 * i64::try_from(citizen_held(o)).unwrap_or(i64::MAX));
    if total > o.treasury {
        return Err(Reject::new(
            RejectCode::InsufficientFunds,
            format!("{total} exceeds the treasury {}", o.treasury),
        ));
    }
    Ok(vec![Event::DividendDeclared {
        org,
        per_share,
        cycle: world.cycle_of(world.meta.tick),
    }])
}

/// Step 8e: pay declared dividends to citizen holders, pro rata if payroll left
/// the treasury short.
pub fn cycle_end_8e_dividends(b: &mut TickBuilder) {
    let orgs: Vec<OrgId> = b
        .world
        .orgs
        .values()
        .filter(|o| o.declared_dividend.is_some())
        .map(|o| o.id)
        .collect();
    for org in orgs {
        let o = &b.world.orgs[&org];
        let Some(per_share) = o.declared_dividend else {
            continue;
        };
        let Ownership::Shares { holdings, .. } = &o.ownership else {
            continue;
        };
        let owed: Vec<(CitizenId, u64, Money)> = holdings
            .iter()
            .filter_map(|(h, q)| match h {
                ShareHolder::Citizen(c) if *q > 0 => {
                    Some((*c, *q, Money(per_share.0 * i64::try_from(*q).unwrap_or(0))))
                }
                _ => None,
            })
            .collect();
        let total: Money = owed.iter().map(|o| o.2).sum();
        let treasury = o.treasury;
        for (citizen, shares, amount) in owed {
            let paid = if total > treasury {
                Money(amount.0 * treasury.0 / total.0.max(1))
            } else {
                amount
            };
            if paid > Money::ZERO {
                let explain = Explain::new(RuleId::DividendPerShare, "shares x per_share", paid)
                    .input("shares", i64::try_from(shares).unwrap_or(i64::MAX))
                    .input("per_share", per_share);
                b.emit(Event::DividendPaid {
                    org,
                    citizen,
                    amount: paid,
                    explain,
                });
            }
        }
    }
}

/// Book value per share: (treasury + inventory and machines at last price) / issued.
#[must_use]
pub fn book_value(world: &World, org: &Org) -> Money {
    let mut value = org.treasury;
    for (g, q) in &org.inventory {
        if let Some(p) = last_price(world, Instrument::Good(*g)) {
            value += Money(p.0 * i64::from(*q));
        }
    }
    let machines: u32 = org
        .workplaces
        .iter()
        .filter_map(|w| world.workplaces.get(w))
        .map(|w| w.machines)
        .sum();
    if let Some(p) = last_price(world, Instrument::Good(Good::Machines)) {
        value += Money(p.0 * i64::from(machines));
    }
    value
}

/// A share's value: last trade if any, else book value per issued share.
#[must_use]
pub fn share_value(world: &World, org: &Org) -> Money {
    if let Some(p) = last_price(world, Instrument::Share(org.id)) {
        return p;
    }
    match org.ownership {
        Ownership::Shares { issued, .. } if issued > 0 => {
            Money(book_value(world, org).0 / i64::try_from(issued).unwrap_or(1))
        }
        _ => Money::ZERO,
    }
}

/// Net worth (GDD §6.1): balance + pantry at last price + shares at value.
#[must_use]
pub fn net_worth(world: &World, citizen: CitizenId) -> Money {
    let Some(c) = world.citizens.get(&citizen) else {
        return Money::ZERO;
    };
    let mut worth = c.household.balance;
    for (g, q) in &c.household.pantry {
        if let Some(p) = last_price(world, Instrument::Good(*g)) {
            worth += Money(p.0 * i64::from(*q));
        }
    }
    for o in world.orgs.values() {
        let held = crate::ledger::shares_of(&o.ownership, ShareHolder::Citizen(citizen));
        if held > 0 {
            worth += Money(share_value(world, o).0 * i64::try_from(held).unwrap_or(0));
        }
    }
    worth
}

/// The "self-made" track: net worth less the endowment.
#[must_use]
pub fn self_made(world: &World, citizen: CitizenId) -> Money {
    net_worth(world, citizen) - world.params.money.endowment
}

/// After shares move, the new controlling owner (if any, and if not already the
/// manager) takes the manager's chair (Q29).
#[must_use]
pub fn control_change(world: &World, org: OrgId, to: ShareHolder, qty: u64) -> Option<Event> {
    let o = world.orgs.get(&org)?;
    let Ownership::Shares { issued, holdings } = &o.ownership else {
        return None;
    };
    let ShareHolder::Citizen(c) = to else {
        return None;
    };
    let after = holdings.get(&to).copied().unwrap_or(0) + qty;
    (after * 2 > *issued && o.manager != Some(c)).then_some(Event::ManagerAppointed {
        org,
        citizen: Some(c),
    })
}
