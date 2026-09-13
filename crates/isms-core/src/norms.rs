//! Work norms, pledges and the Ledger of Contribution (GDD §6.2, §7.2 pledge
//! row; TDD §5.5 step 8i, S0.15b). Norms are advisory: nothing here changes
//! what a citizen may do. The ledger records, exactly, the hours each citizen
//! worked this cycle and the output attributed to them under the society's
//! monitoring (the phase-4 figure, already noised; Q55), and whether the
//! published norm was met. A pledge is a public promise with a term; when the
//! term ends the ledger closes it and records whether the hours half was kept.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::constitution::LaborMode;
use crate::event::{ContributionEntry, Event};
use crate::ids::{CitizenId, OrgId};
use crate::kinds::Good;
use crate::ledger::Party;
use crate::tick::TickBuilder;
use crate::world::{ContractBody, ContractStatus, World};
use std::collections::BTreeMap;

/// `Pledge`: commit hours per cycle and/or goods to an org, or to the assembly
/// when `to` is `None` (Q57). Recorded, never enforced.
pub fn pledge(
    world: &World,
    envelope: &Envelope<Command>,
    hours: Option<u8>,
    goods: Option<(Good, u32)>,
    term_cycles: u32,
    to: Option<OrgId>,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    if hours.is_none() && goods.is_none() {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "a pledge commits hours or goods",
        ));
    }
    if hours.is_some_and(|h| h == 0 || h > world.params.labor.base_budget_hours)
        || goods.is_some_and(|(_, q)| q == 0)
    {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "pledged hours must be within the budget and goods positive",
        ));
    }
    if term_cycles == 0 {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            "a pledge needs a term",
        ));
    }
    if let Some(org) = to
        && !world.orgs.contains_key(&org)
    {
        return Err(Reject::new(RejectCode::UnknownOrg, format!("no org {org}")));
    }
    Ok(vec![Event::Pledged {
        contract: world.next.contract,
        citizen: citizen.id,
        to,
        hours,
        goods,
        term_cycles,
    }])
}

/// `JoinWorkplace` (norm systems, Q62): take a position at a workplace with
/// room. Positions carry no contract; the Ledger of Contribution is the record.
pub fn join_workplace(
    world: &World,
    envelope: &Envelope<Command>,
    workplace: crate::ids::WorkplaceId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let wp = crate::orgs::check_room(world, workplace)?;
    if wp.workers.contains_key(&citizen.id) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{} already works at {workplace}", citizen.id),
        ));
    }
    let held = world
        .workplaces
        .values()
        .filter(|w| w.workers.contains_key(&citizen.id))
        .count();
    if held >= usize::from(world.params.labor.max_workplaces) {
        return Err(Reject::new(
            RejectCode::TooManyWorkplaces,
            format!("at most {} workplaces", world.params.labor.max_workplaces),
        ));
    }
    Ok(vec![Event::Assigned {
        workplace,
        citizen: citizen.id,
        contract: None,
    }])
}

/// `LeaveWorkplace`: give up a contract-less position.
pub fn leave_workplace(
    world: &World,
    envelope: &Envelope<Command>,
    workplace: crate::ids::WorkplaceId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let wp = world.workplaces.get(&workplace).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownWorkplace,
            format!("no workplace {workplace}"),
        )
    })?;
    match wp.workers.get(&citizen.id) {
        Some(a) if a.contract.is_none() => Ok(vec![Event::Unassigned {
            workplace,
            citizen: citizen.id,
        }]),
        _ => Err(Reject::new(
            RejectCode::NotAssigned,
            format!("{} holds no position at {workplace}", citizen.id),
        )),
    }
}

/// The parties of a pledge contract: the citizen and the org, or the citizen
/// twice for a pledge to the assembly.
#[must_use]
pub fn pledge_parties(citizen: CitizenId, to: Option<OrgId>) -> (Party, Party) {
    (
        Party::Citizen(citizen),
        to.map_or(Party::Citizen(citizen), Party::Org),
    )
}

/// This cycle's tick-hours and attributed output per active citizen, summed
/// over every workplace where they hold a position.
#[must_use]
pub fn cycle_contribution(world: &World) -> BTreeMap<CitizenId, (u32, f64)> {
    let mut out: BTreeMap<CitizenId, (u32, f64)> = world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| (c.id, (0, 0.0)))
        .collect();
    for w in world.workplaces.values() {
        for (id, a) in &w.workers {
            if let Some(e) = out.get_mut(id) {
                e.0 += a.cycle_tick_hours;
                e.1 += a.cycle_attributed;
            }
        }
    }
    out
}

/// Step 8i (norm systems): close the Ledger of Contribution for the cycle and
/// any pledge whose term has run.
pub fn cycle_end_8i_norms_ledger(b: &mut TickBuilder) {
    if b.rules.capabilities.labor != LaborMode::Norm {
        return;
    }
    let tpc = b.world.params.time.ticks_per_cycle;
    let norm_tick_hours = u32::from(b.world.policy.work_norm_hours.unwrap_or(0)) * tpc;
    let contribution = cycle_contribution(&b.world);
    let entries: Vec<ContributionEntry> = contribution
        .iter()
        .map(|(citizen, (tick_hours, attributed))| ContributionEntry {
            citizen: *citizen,
            tick_hours: *tick_hours,
            attributed: *attributed,
            met_norm: *tick_hours >= norm_tick_hours,
        })
        .collect();
    b.emit(Event::NormsLedgerClosed {
        cycle: b.cycle,
        entries,
    });
    // Pledges whose term ends with this cycle (the S0.10 term rule, Q28).
    let cycle = b.cycle;
    let due: Vec<(crate::ids::ContractId, Option<bool>)> = b
        .world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active)
        .filter_map(|k| match k.body {
            ContractBody::Pledge { hours, .. } => {
                let created = b.world.cycle_of(k.created_tick);
                let term = k.term_cycles.unwrap_or(1);
                (cycle + 1 >= created + term).then(|| {
                    let met = hours.map(|h| {
                        let Party::Citizen(c) = k.parties.0 else {
                            return false;
                        };
                        contribution
                            .get(&c)
                            .is_some_and(|(t, _)| *t >= u32::from(h) * tpc)
                    });
                    (k.id, met)
                })
            }
            _ => None,
        })
        .collect();
    for (contract, met) in due {
        b.emit(Event::PledgeClosed { contract, met });
    }
}
