//! Unions, collective agreements and strikes (GDD §6.4, §7.2 collective
//! agreement row; S0.17d). The workers of one firm may form a union org that
//! negotiates a single agreement for all its members: a wage floor above the
//! society's, and an hours cap (Q102). Dues fund a strike pay (Q101); a strike
//! is a coordinated hours-zero at the firm for a number of cycles. Built and
//! tested here; the householder never forms one (Q103).

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, Cycle, OfferId, OrgId, WorkplaceId};
use crate::kinds::OrgKind;
use crate::ledger::Party;
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::world::{ContractBody, ContractStatus, OfferBody, Org, Ownership, UnionState, World};

fn union_of(world: &World, org: OrgId) -> Result<(&Org, &UnionState), Reject> {
    let o = world
        .orgs
        .get(&org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
    let u = o.union.as_ref().ok_or_else(|| {
        Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} is not a union", o.name),
        )
    })?;
    Ok((o, u))
}

/// Whether the citizen holds an active employment contract with `firm`.
#[must_use]
pub fn employed_by(world: &World, citizen: CitizenId, firm: OrgId) -> bool {
    world.contracts.values().any(|k| {
        k.status != ContractStatus::Ended
            && k.parties.0 == Party::Org(firm)
            && k.parties.1 == Party::Citizen(citizen)
            && matches!(k.body, ContractBody::Employment { .. })
    })
}

/// The union whose firm employs the citizen and which counts them a member.
#[must_use]
pub fn union_for(world: &World, firm: OrgId, citizen: CitizenId) -> Option<&Org> {
    world
        .orgs
        .values()
        .find(|o| o.union.as_ref().is_some_and(|u| u.firm == firm) && o.members.contains(&citizen))
}

/// The active collective agreement covering a firm, if any: (floor, hours cap).
#[must_use]
pub fn agreement_for(world: &World, firm: OrgId) -> Option<(Money, u8)> {
    world.contracts.values().find_map(|k| match k.body {
        ContractBody::CollectiveAgreement { wage_floor, hours }
            if k.status == ContractStatus::Active && k.parties.1 == Party::Org(firm) =>
        {
            Some((wage_floor, hours))
        }
        _ => None,
    })
}

/// `FormUnion`: an employee of `firm` founds its union and becomes steward.
pub fn form_union(
    world: &World,
    envelope: &Envelope<Command>,
    firm: OrgId,
    name: &str,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let f = world
        .orgs
        .get(&firm)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {firm}")))?;
    if f.kind != OrgKind::Firm {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} is not a firm", f.name),
        ));
    }
    if !employed_by(world, citizen.id, firm) {
        return Err(Reject::new(
            RejectCode::NotParty,
            format!("{} does not work for {}", citizen.id, f.name),
        ));
    }
    if world
        .orgs
        .values()
        .any(|o| o.union.as_ref().is_some_and(|u| u.firm == firm))
    {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{} already has a union", f.name),
        ));
    }
    if name.trim().is_empty() {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "a union needs a name",
        ));
    }
    let org = world.next.org;
    Ok(vec![
        Event::OrgFounded {
            org,
            kind: OrgKind::Union,
            name: name.to_owned(),
            founder: Some(citizen.id),
            ownership: Ownership::Members,
            manager: Some(citizen.id),
            fee_burned: Money::ZERO,
        },
        Event::UnionFormed { org, firm },
    ])
}

/// A union's `admit_member`: the steward admits a fellow employee of the firm.
pub fn admit(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    citizen: CitizenId,
) -> Option<Result<Vec<Event>, Reject>> {
    let o = world.orgs.get(&org)?;
    let u = o.union.as_ref()?;
    Some((|| {
        let actor = acting_citizen(world, envelope)?;
        if o.manager != Some(actor.id) {
            return Err(Reject::new(
                RejectCode::NotManager,
                "only the steward admits",
            ));
        }
        if o.members.contains(&citizen) {
            return Err(Reject::new(RejectCode::AlreadyExists, "already a member"));
        }
        if !employed_by(world, citizen, u.firm) {
            return Err(Reject::new(
                RejectCode::NotParty,
                format!("{citizen} does not work for the union's firm"),
            ));
        }
        Ok(vec![Event::MemberAdmitted { org, citizen }])
    })())
}

/// `OfferCollectiveAgreement`: the steward proposes terms to the firm.
pub fn offer_agreement(
    world: &World,
    envelope: &Envelope<Command>,
    union: OrgId,
    wage_floor: Money,
    hours: u8,
    term_cycles: u32,
) -> Result<Vec<Event>, Reject> {
    let actor = acting_citizen(world, envelope)?;
    let (o, u) = union_of(world, union)?;
    if o.manager != Some(actor.id) {
        return Err(Reject::new(
            RejectCode::NotManager,
            "only the steward offers",
        ));
    }
    if wage_floor <= Money::ZERO || hours == 0 || hours > world.params.labor.base_budget_hours {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "a floor above zero and hours within the budget",
        ));
    }
    if term_cycles == 0 {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            "a term of at least one cycle",
        ));
    }
    if agreement_for(world, u.firm).is_some() {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            "an agreement is already in force",
        ));
    }
    Ok(vec![Event::CollectiveAgreementOffered {
        offer: world.next.offer,
        body: OfferBody::CollectiveAgreement {
            union,
            firm: u.firm,
            wage_floor,
            hours,
            term_cycles,
        },
    }])
}

/// `AcceptCollectiveAgreement`: the firm's manager signs.
pub fn accept_agreement(
    world: &World,
    envelope: &Envelope<Command>,
    offer: OfferId,
) -> Result<Vec<Event>, Reject> {
    let f = world
        .offers
        .get(&offer)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOffer, format!("no offer {offer}")))?;
    let OfferBody::CollectiveAgreement {
        union,
        firm,
        wage_floor,
        hours,
        term_cycles,
    } = f.body
    else {
        return Err(Reject::new(RejectCode::UnknownOffer, "not an agreement"));
    };
    crate::orgs::managed_org(world, envelope, firm)?;
    Ok(vec![Event::CollectiveAgreementAccepted {
        contract: world.next.contract,
        offer,
        union,
        firm,
        wage_floor,
        hours,
        term_cycles,
    }])
}

/// `CallStrike`: the steward calls the members out for `cycles` cycles.
pub fn call_strike(
    world: &World,
    envelope: &Envelope<Command>,
    union: OrgId,
    cycles: u32,
) -> Result<Vec<Event>, Reject> {
    let actor = acting_citizen(world, envelope)?;
    let (o, u) = union_of(world, union)?;
    if o.manager != Some(actor.id) {
        return Err(Reject::new(
            RejectCode::NotManager,
            "only the steward calls a strike",
        ));
    }
    if cycles == 0 {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            "a strike lasts at least one cycle",
        ));
    }
    if u.strike_until.is_some() {
        return Err(Reject::new(RejectCode::AlreadyExists, "already on strike"));
    }
    let now = world.cycle_of(envelope.received_at_tick);
    Ok(vec![Event::StrikeCalled {
        union,
        firm: u.firm,
        until_cycle: now + cycles,
    }])
}

/// Whether the citizen's work at `workplace` is withdrawn by a strike this cycle.
#[must_use]
pub fn on_strike(world: &World, citizen: CitizenId, workplace: WorkplaceId, cycle: Cycle) -> bool {
    let Some(firm) = world.workplaces.get(&workplace).map(|w| w.org) else {
        return false;
    };
    union_for(world, firm, citizen)
        .and_then(|u| u.union.as_ref()?.strike_until)
        .is_some_and(|until| cycle < until)
}

/// Step 8a, after payroll: dues from every member who can pay, strike pay to
/// members on strike, strikes that have run their course end, and agreements
/// whose term has run end.
pub fn cycle_end_8a_unions(b: &mut TickBuilder) {
    let unions: Vec<(OrgId, OrgId, Option<Cycle>)> = b
        .world
        .orgs
        .values()
        .filter_map(|o| o.union.as_ref().map(|u| (o.id, u.firm, u.strike_until)))
        .collect();
    let dues = b.world.params.union.dues_per_cycle;
    let strike_pay = b.world.params.union.strike_pay_per_cycle;
    let cycle = b.cycle;
    for (union, _firm, strike_until) in unions {
        let members: Vec<CitizenId> = b.world.orgs[&union].members.iter().copied().collect();
        if dues > Money::ZERO {
            for m in &members {
                if b.world.citizens[m].household.balance >= dues {
                    b.emit(Event::DuesPaid {
                        union,
                        citizen: *m,
                        amount: dues,
                    });
                }
            }
        }
        if let Some(until) = strike_until {
            if cycle < until {
                let n = i64::try_from(members.len().max(1)).unwrap_or(1);
                // Each striker's share is set from the pool as it stands
                // before the first payment, so the pool is split evenly.
                let pool = b.world.orgs[&union].treasury;
                let each = strike_pay.min(Money(pool.0 / n));
                for m in &members {
                    let amount = each.min(b.world.orgs[&union].treasury);
                    if amount <= Money::ZERO {
                        continue;
                    }
                    let explain = Explain::new(
                        RuleId::StrikePay,
                        "min(strike_pay_per_cycle, union treasury / members)",
                        amount,
                    )
                    .input("strike_pay_per_cycle", strike_pay)
                    .input("union_treasury", pool)
                    .input("members", n);
                    b.emit(Event::StrikePaid {
                        union,
                        citizen: *m,
                        amount,
                        explain,
                    });
                }
            }
            if cycle + 1 >= until {
                b.emit(Event::StrikeEnded { union });
            }
        }
    }
    // Agreements expire at term end (the S0.10 term rule).
    let tpc = b.world.params.time.ticks_per_cycle;
    let expired: Vec<crate::ids::ContractId> = b
        .world
        .contracts
        .values()
        .filter(|k| {
            k.status == ContractStatus::Active
                && matches!(k.body, ContractBody::CollectiveAgreement { .. })
                && k.term_cycles
                    .is_some_and(|t| cycle + 1 >= k.created_tick / tpc + t)
        })
        .map(|k| k.id)
        .collect();
    for contract in expired {
        b.emit(Event::CollectiveAgreementEnded { contract });
    }
}
