//! Cooperatives (GDD §6.5, §5 A1 `cooperative`, A3 `share`; TDD §5.5 step
//! 8a, S0.17b). A coop is owned equally by its current members, who hold
//! positions at its workplaces rather than employment contracts (Q86); it
//! pays them nothing during the cycle and shares its surplus out at cycle end
//! (Q87), equally or by hours as the coop decides. Leaving forfeits any share
//! (Q89). Admission is by the manager until the member vote lands (S0.17c);
//! the manager election is stubbed as `AppointManager` by the System (Q90).

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::constitution::Compensation;
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, OrgId, WorkplaceId};
use crate::kinds::OrgKind;
use crate::ledger::Party;
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::world::{ContractBody, ContractStatus, Org, ShareRule, World};

/// Whether the org is a cooperative.
#[must_use]
pub fn is_coop(world: &World, org: OrgId) -> bool {
    world
        .orgs
        .get(&org)
        .is_some_and(|o| o.kind == OrgKind::Cooperative)
}

/// The coop's workplace with the fewest workers that still has room, ties to
/// the lowest id: where a new member is placed.
#[must_use]
pub fn least_crowded(world: &World, org: &Org) -> Option<WorkplaceId> {
    org.workplaces
        .iter()
        .filter_map(|w| world.workplaces.get(w))
        .filter(|w| crate::orgs::check_room(world, w.id).is_ok())
        .min_by_key(|w| (w.workers.len(), w.id))
        .map(|w| w.id)
}

/// Whether the coop can take another member: a workplace with room.
#[must_use]
pub fn would_admit(world: &World, org: &Org) -> bool {
    org.kind == OrgKind::Cooperative && least_crowded(world, org).is_some()
}

/// The legacy hiring cap (Q2) as a coop sees it: a workplace is glutted when
/// its output stock exceeds `legacy_hire_inventory_cycles_cap` cycles of full
/// production; a Builders' coop when that many of its dwellings stand empty
/// (a firm's Builders keep building on the owner's account). A glutted
/// workplace admits nobody, and a member choosing where to go must know it
/// (Q99): a mover into a glutted economy would otherwise land nowhere.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn glutted(world: &World, org: &Org, wp: &crate::world::Workplace) -> bool {
    let p = &world.params;
    let recipe = &p.recipes[&wp.kind];
    let full_cycle_output = recipe.base_rate
        * f64::from(p.labor.max_workers_per_workplace)
        * f64::from(p.labor.base_budget_hours);
    let stock = recipe.produces.as_good().map_or_else(
        || {
            if org.kind == OrgKind::Cooperative {
                u32::try_from(
                    world
                        .dwellings
                        .values()
                        .filter(|d| {
                            d.owner == crate::world::Owner::Org(org.id) && d.occupant.is_none()
                        })
                        .count(),
                )
                .unwrap_or(u32::MAX)
            } else {
                0
            }
        },
        |g| org.inventory.get(&g).copied().unwrap_or(0),
    );
    f64::from(stock) > full_cycle_output * f64::from(p.householder.legacy_hire_inventory_cycles_cap)
}

/// Whether the coop's steward would admit anyone this hour: a workplace with
/// room that is not glutted.
#[must_use]
pub fn admitting(world: &World, org: &Org) -> bool {
    org.kind == OrgKind::Cooperative
        && org.workplaces.iter().any(|w| {
            world.workplaces.get(w).is_some_and(|wp| {
                u32::try_from(wp.workers.len()).unwrap_or(u32::MAX)
                    < world.params.labor.max_workers_per_workplace
                    && !glutted(world, org, wp)
            })
        })
}

/// Places a coop could still fill, net of the requests already waiting on it:
/// what a citizen choosing where to ask should count.
#[must_use]
pub fn open_places(world: &World, org: &Org) -> u32 {
    if org.kind != OrgKind::Cooperative {
        return 0;
    }
    let max = world.params.labor.max_workers_per_workplace;
    let room: u32 = org
        .workplaces
        .iter()
        .filter_map(|w| world.workplaces.get(w))
        .map(|w| max.saturating_sub(u32::try_from(w.workers.len()).unwrap_or(u32::MAX)))
        .sum();
    let pending = u32::try_from(
        world
            .offers
            .values()
            .filter(|f| matches!(f.body, crate::world::OfferBody::Membership { org: x, .. } if x == org.id))
            .count(),
    )
    .unwrap_or(u32::MAX);
    room.saturating_sub(pending)
}

/// `SetShareRule`: the coop decides how its surplus is split (GDD §6.5).
pub fn set_share_rule(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    rule: ShareRule,
) -> Result<Vec<Event>, Reject> {
    let o = crate::orgs::managed_org(world, envelope, org)?;
    if o.kind != OrgKind::Cooperative {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} is not a cooperative", o.name),
        ));
    }
    Ok(vec![Event::ShareRuleSet { org, rule }])
}

/// Money the coop must hold back from this cycle's surplus: the credit
/// installments due at 8c (and, from S0.17c, the capital levy at 8b).
#[must_use]
pub fn obligations_due(world: &World, org: OrgId) -> Money {
    world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active && k.parties.1 == Party::Org(org))
        .filter_map(|k| match k.body {
            ContractBody::Credit { installment, .. } => Some(installment),
            _ => None,
        })
        .sum::<Money>()
        + crate::bank::levy_due(world, org)
}

/// This cycle's surplus (Q88): what the treasury gained since the last
/// share-out, beyond capital that came in (seed money, loan principal) and the
/// obligations falling due, never negative and never more than is held.
#[must_use]
pub fn surplus(world: &World, org: &Org) -> Money {
    (org.treasury - org.surplus_base - obligations_due(world, org.id))
        .max_zero()
        .min(org.treasury)
}

/// The share-out: (member, amount, explain), summing exactly to `surplus`.
/// Equal: floor(surplus / members) each, remainder cents to members in id
/// order. Hours-weighted: by the member's tick-hours at the coop's workplaces
/// this cycle, remainder cents in id order; with no hours worked, nothing.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn share_out(world: &World, org: &Org, surplus: Money) -> Vec<(CitizenId, Money, Explain)> {
    let members: Vec<CitizenId> = org.members.iter().copied().collect();
    if members.is_empty() || surplus <= Money::ZERO {
        return Vec::new();
    }
    let rule = org.share_rule.unwrap_or(ShareRule::Equal);
    let hours: Vec<(CitizenId, u64)> = members
        .iter()
        .map(|m| {
            let h: u64 = org
                .workplaces
                .iter()
                .filter_map(|w| world.workplaces.get(w))
                .filter_map(|w| w.workers.get(m))
                .map(|a| u64::from(a.cycle_tick_hours))
                .sum();
            (*m, h)
        })
        .collect();
    let total_hours: u64 = hours.iter().map(|(_, h)| *h).sum();
    let n = i64::try_from(members.len()).unwrap_or(i64::MAX);
    let mut out: Vec<(CitizenId, Money, Explain)> = Vec::new();
    let mut given = 0i64;
    for (m, h) in &hours {
        let (amount, explain) = match rule {
            ShareRule::Equal => (
                Money(surplus.0 / n),
                Explain::new(RuleId::PayShare, "surplus / members", Money(surplus.0 / n))
                    .input("surplus", surplus)
                    .input("members", n),
            ),
            ShareRule::HoursWeighted => {
                if total_hours == 0 {
                    continue;
                }
                let amount = Money(
                    ((i128::from(surplus.0) * i128::from(*h)) / i128::from(total_hours)) as i64,
                );
                (
                    amount,
                    Explain::new(RuleId::PayShare, "surplus x hours / total_hours", amount)
                        .input("surplus", surplus)
                        .input("hours", i64::try_from(*h).unwrap_or(i64::MAX))
                        .input(
                            "total_hours",
                            i64::try_from(total_hours).unwrap_or(i64::MAX),
                        ),
                )
            }
        };
        given += amount.0;
        out.push((*m, amount, explain));
    }
    // The indivisible remainder goes one cent each, in id order, to those with a share.
    let mut remainder = surplus.0 - given;
    for entry in &mut out {
        if remainder == 0 {
            break;
        }
        if rule == ShareRule::Equal || entry.1 > Money::ZERO {
            entry.1 += Money(1);
            entry.2.result = entry.1.into();
            remainder -= 1;
        }
    }
    out.retain(|e| e.1 > Money::ZERO);
    out
}

/// Step 8a for share systems: every coop declares its surplus (zero included,
/// so last cycle's share per member is never stale) and shares what there is.
pub fn cycle_end_8a_share_out(b: &mut TickBuilder) {
    if b.rules.capabilities.pay != Compensation::Share {
        return;
    }
    let coops: Vec<OrgId> = b
        .world
        .orgs
        .values()
        .filter(|o| o.kind == OrgKind::Cooperative)
        .map(|o| o.id)
        .collect();
    for org in coops {
        let o = &b.world.orgs[&org];
        let s = surplus(&b.world, o);
        let shares = share_out(&b.world, o, s);
        b.emit(Event::SurplusDeclared {
            org,
            cycle: b.cycle,
            surplus: s,
            rule: o.share_rule.unwrap_or(ShareRule::Equal),
            members: u32::try_from(o.members.len()).unwrap_or(u32::MAX),
        });
        for (citizen, amount, explain) in shares {
            b.emit(Event::ShareOutPaid {
                org,
                citizen,
                amount,
                explain,
            });
        }
    }
}

/// Last cycle's surplus per member, the Commonwealth's scoreboard figure.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn surplus_per_member(org: &Org) -> f64 {
    if org.last_share_out_members == 0 {
        0.0
    } else {
        org.last_surplus.as_credits_f64() / f64::from(org.last_share_out_members)
    }
}

/// Cycles a citizen has been a member, from `member_since`.
#[must_use]
pub fn tenure_cycles(world: &World, org: &Org, citizen: CitizenId) -> u32 {
    org.member_since.get(&citizen).map_or(0, |since| {
        world
            .meta
            .tick
            .saturating_sub(*since)
            .checked_div(world.params.time.ticks_per_cycle)
            .unwrap_or(0)
    })
}

/// A coop's `admit_member`: the manager admits an unplaced citizen into the
/// least-crowded workplace with room (Q86). Returns `None` when the org is
/// not a coop, so the association path applies.
pub fn admit(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    citizen: CitizenId,
) -> Option<Result<Vec<Event>, Reject>> {
    let o = world.orgs.get(&org)?;
    if o.kind != OrgKind::Cooperative {
        return None;
    }
    Some((|| {
        let actor = acting_citizen(world, envelope)?;
        if o.manager != Some(actor.id) {
            return Err(Reject::new(
                RejectCode::NotManager,
                "only the manager admits (the member vote arrives in S0.17c)",
            ));
        }
        if !world.citizens.contains_key(&citizen) {
            return Err(Reject::new(
                RejectCode::UnknownCitizen,
                format!("no citizen {citizen}"),
            ));
        }
        if o.members.contains(&citizen) {
            return Err(Reject::new(RejectCode::AlreadyExists, "already a member"));
        }
        if crate::labor::has_position(world, citizen) {
            return Err(Reject::new(
                RejectCode::AlreadyExists,
                format!("{citizen} already holds a position elsewhere"),
            ));
        }
        let workplace = least_crowded(world, o).ok_or_else(|| {
            Reject::new(RejectCode::WorkplaceFull, format!("{} has no room", o.name))
        })?;
        Ok(vec![
            Event::MemberAdmitted { org, citizen },
            Event::Assigned {
                workplace,
                citizen,
                contract: None,
            },
        ])
    })())
}

/// The events that take a citizen out of a coop: `MemberLeft` plus an
/// `Unassigned` for every position at its workplaces.
#[must_use]
pub fn leave_events(world: &World, org: &Org, citizen: CitizenId) -> Vec<Event> {
    let mut events = vec![Event::MemberLeft {
        org: org.id,
        citizen,
    }];
    for w in org
        .workplaces
        .iter()
        .filter_map(|w| world.workplaces.get(w))
    {
        if w.workers.contains_key(&citizen) {
            events.push(Event::Unassigned {
                workplace: w.id,
                citizen,
            });
        }
    }
    events
}
