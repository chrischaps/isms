//! The proposal machine (GDD §8.1, §6.2 Governance; TDD S2.1): one for every
//! society. A citizen opens a typed proposal; citizens cast one ballot each,
//! replaceable until the close; at the cycle end (8j) the missing ballots are
//! filled from each citizen's standing `vote_default`, the quorum is checked
//! against the active humans, and a proposal that carries passes by yes > no.
//! A passed `PolicyChange` becomes a `PolicyChanged`; a `Resolution` is only
//! its record (Q119). Admission votes (S0.17c) ride the same `Proposal` and
//! close here too, by the cooperative's own rule in `bank.rs`.
//!
//! The constitution gates the machine: `proposal_kinds` is the set of kinds
//! the society may open (checked in `Capabilities::allows`) and `proposers`
//! says who may open one. Householders never propose and never vote.

use crate::command::{Command, Envelope, Reject, RejectCode};
use crate::constitution::{OfficeKind, Proposers, RecallRule};
use crate::event::{Actor, Event};
use crate::ids::{CitizenId, ProposalId};
use crate::kinds::CitizenKind;
use crate::tick::TickBuilder;
use crate::world::{Ballot, Citizen, Proposal, ProposalKind, Tally, VoteDefault, World};
use std::collections::BTreeMap;

/// Whether `citizen` holds a seat of `kind` right now.
#[must_use]
pub fn holds(world: &World, citizen: CitizenId, kind: OfficeKind) -> bool {
    world.offices.holds(citizen, kind)
}

/// Whether `citizen` holds any office.
#[must_use]
pub fn holds_any_office(world: &World, citizen: CitizenId) -> bool {
    world
        .offices
        .holders
        .values()
        .any(|seats| seats.iter().any(|h| h.citizen == citizen))
}

/// The proposals `citizen` has open.
#[must_use]
pub fn open_by(world: &World, citizen: CitizenId) -> usize {
    world.proposals.values().filter(|p| p.by == citizen).count()
}

/// Ballots needed for a vote to carry: `ceil(quorum_fraction x eligible)`
/// (GDD §8.1 "20% of active humans"; Q117). One human needs one ballot.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn quorum_needed(quorum_fraction: f64, eligible: u32) -> u32 {
    (quorum_fraction * f64::from(eligible)).ceil().max(0.0) as u32
}

/// The citizens whose ballots the assembly counts: non-dormant humans (Q117).
/// Householders never vote.
#[must_use]
pub fn electorate(world: &World) -> Vec<CitizenId> {
    world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Human && !c.dormant)
        .map(|c| c.id)
        .collect()
}

/// The count of an assembly vote over `ballots`, against `eligible` voters.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn assembly_tally(
    ballots: &BTreeMap<CitizenId, Ballot>,
    eligible: u32,
    quorum_fraction: f64,
) -> Tally {
    let count = |b: Ballot| ballots.values().filter(|x| **x == b).count() as u32;
    Tally {
        yes: count(Ballot::Yes),
        no: count(Ballot::No),
        abstain: count(Ballot::Abstain),
        cast: ballots.len() as u32,
        quorum: quorum_needed(quorum_fraction, eligible),
        eligible,
    }
}

/// Whether an assembly tally carries: the quorum reached over every ballot
/// cast, abstentions included, and more yes than no among the rest (Q117).
#[must_use]
pub const fn carries(t: &Tally) -> bool {
    t.cast >= t.quorum && t.yes > t.no
}

/// Whether a recall carries under `rule` (GDD 8.2 "Removal"): the quorum as
/// for any vote, then a simple majority of the yes and no ballots, or two
/// thirds of them (S2.2).
#[must_use]
pub const fn recall_carries(t: &Tally, rule: RecallRule) -> bool {
    match rule {
        RecallRule::Majority => carries(t),
        RecallRule::TwoThirds => t.cast >= t.quorum && t.yes > 0 && t.yes * 3 >= (t.yes + t.no) * 2,
    }
}

/// A citizen who may take part in the assembly at all.
fn assembly_citizen<'w>(
    world: &'w World,
    envelope: &Envelope<Command>,
) -> Result<&'w Citizen, Reject> {
    let actor = crate::command::acting_citizen(world, envelope)?;
    if actor.kind == CitizenKind::Householder {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "householders never vote and never propose",
        ));
    }
    Ok(actor)
}

/// `Propose`: open a proposal that closes at the end of the current cycle.
pub fn propose(
    world: &World,
    envelope: &Envelope<Command>,
    title: &str,
    text: &str,
    kind: ProposalKind,
) -> Result<Vec<Event>, Reject> {
    let actor = assembly_citizen(world, envelope)?;
    if world.constitution.proposers == Proposers::OfficeHolders
        && !holds_any_office(world, actor.id)
    {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            "only office-holders propose here",
        ));
    }
    if title.trim().is_empty() {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "a proposal needs a title",
        ));
    }
    let cap = world.params.governance.open_proposals_per_citizen;
    if open_by(world, actor.id) >= cap as usize {
        return Err(Reject::new(
            RejectCode::TooManyProposals,
            format!("{cap} open proposals at once is the limit"),
        ));
    }
    match kind {
        ProposalKind::Admission { org, citizen } => {
            return crate::bank::propose_admission(world, envelope, org, citizen);
        }
        ProposalKind::PolicyChange { patch } => {
            if patch.is_empty() {
                return Err(Reject::new(
                    RejectCode::InvalidQuantity,
                    "an empty patch moves nothing",
                ));
            }
            // GDD 6.2 lists "propose the rationing rule" among the coordinator's
            // powers; read as exclusive (Q116).
            if patch.rationing.is_some() && !holds(world, actor.id, OfficeKind::Coordinator) {
                return Err(Reject::new(
                    RejectCode::NotAuthorized,
                    "the rationing rule is a coordinator's to propose",
                ));
            }
            patch
                .apply_to(&world.policy)
                .validate_against(&world.constitution)
                .map_err(|reason| Reject::new(RejectCode::NotInThisSociety, reason))?;
        }
        ProposalKind::Resolution => {
            if text.trim().is_empty() {
                return Err(Reject::new(
                    RejectCode::InvalidQuantity,
                    "a resolution is its text",
                ));
            }
        }
        ProposalKind::Election { .. } => {
            // Elections open by the calendar (epoch start, a vacancy, a term's
            // last cycle), never by motion (S2.2, Q125).
            return Err(Reject::new(
                RejectCode::NotAuthorized,
                "elections open by the calendar; stand for the office instead",
            ));
        }
        ProposalKind::Recall { office, citizen } => {
            if !holds(world, citizen, office) {
                return Err(Reject::new(
                    RejectCode::NotAnOfficeHolder,
                    format!("{citizen} does not hold that office"),
                ));
            }
            if world
                .proposals
                .values()
                .any(|p| p.kind == ProposalKind::Recall { office, citizen })
            {
                return Err(Reject::new(
                    RejectCode::AlreadyExists,
                    format!("a recall of {citizen} is already open"),
                ));
            }
        }
        ProposalKind::Honor { citizen } => check_honor(world, citizen)?,
        ProposalKind::Disbursement { .. } => {
            return Err(Reject::new(
                RejectCode::NotImplemented,
                "disbursement votes arrive with S2.4",
            ));
        }
    }
    Ok(vec![Event::Proposed {
        proposal: world.next.proposal,
        by: actor.id,
        title: title.trim().to_owned(),
        text: text.trim().to_owned(),
        kind,
        closes_cycle: world.cycle_of(world.meta.tick),
    }])
}

/// An honor's subject (S2.3, Q119): one line on the record, at most one
/// motion per subject per cycle, never revoked. The assembly may honor any
/// citizen it likes, the mover included: that too is data.
fn check_honor(world: &World, citizen: CitizenId) -> Result<(), Reject> {
    if !world.citizens.contains_key(&citizen) {
        return Err(Reject::new(
            RejectCode::UnknownCitizen,
            format!("no citizen {citizen}"),
        ));
    }
    if world
        .proposals
        .values()
        .any(|p| p.kind == ProposalKind::Honor { citizen })
    {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("a motion to honor {citizen} is already before the assembly"),
        ));
    }
    Ok(())
}

/// `Vote`: cast or replace a ballot on an open proposal.
pub fn vote(
    world: &World,
    envelope: &Envelope<Command>,
    proposal: ProposalId,
    ballot: Ballot,
) -> Result<Vec<Event>, Reject> {
    let actor = assembly_citizen(world, envelope)?;
    let p = world.proposals.get(&proposal).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownProposal,
            format!("no open proposal {proposal}"),
        )
    })?;
    if matches!(p.kind, ProposalKind::Admission { .. }) {
        return crate::bank::vote_admission(world, envelope, proposal, ballot);
    }
    Ok(vec![Event::Voted {
        proposal,
        citizen: actor.id,
        ballot,
        by_default: false,
    }])
}

/// The ballot `vote_default` casts for a citizen who cast none: `Abstain`
/// as itself, `Follow` as the followed citizen's own ballot (one hop: a
/// followed citizen who cast none abstains the follower, Q121), `None` as
/// nothing.
#[must_use]
pub fn default_ballot(citizen: &Citizen, own: &BTreeMap<CitizenId, Ballot>) -> Option<Ballot> {
    match citizen.plan.vote_default {
        VoteDefault::Abstain => Some(Ballot::Abstain),
        VoteDefault::Follow(who) => Some(own.get(&who).copied().unwrap_or(Ballot::Abstain)),
        VoteDefault::None => None,
    }
}

/// Step 8j: close every proposal whose cycle has ended, in proposal-id order
/// (so two changes to one field apply later-over-earlier, Q121).
pub fn cycle_end_8j_close_proposals(b: &mut TickBuilder) {
    let cycle = b.cycle;
    let due: Vec<Proposal> = b
        .world
        .proposals
        .values()
        .filter(|p| p.closes_cycle <= cycle)
        .cloned()
        .collect();
    for p in due {
        match p.kind {
            ProposalKind::Admission { org, .. } => {
                // The members did not reach a majority in the cycle: it lapses.
                let tally = crate::bank::admission_tally(&p.ballots, &b.world.orgs[&org]);
                b.emit(Event::ProposalClosed {
                    proposal: p.id,
                    passed: false,
                    tally,
                });
            }
            _ => close_assembly_proposal(b, &p),
        }
    }
}

fn close_assembly_proposal(b: &mut TickBuilder, p: &Proposal) {
    let eligible = electorate(&b.world);
    // Ballots the citizens cast themselves, before any default is filled in:
    // `Follow` copies from these and only these (one hop).
    let own = p.ballots.clone();
    let mut ballots = own.clone();
    for id in &eligible {
        if own.contains_key(id) {
            continue;
        }
        let Some(ballot) = default_ballot(&b.world.citizens[id], &own) else {
            continue;
        };
        ballots.insert(*id, ballot);
        b.emit(Event::Voted {
            proposal: p.id,
            citizen: *id,
            ballot,
            by_default: true,
        });
    }
    let tally = assembly_tally(
        &ballots,
        u32::try_from(eligible.len()).unwrap_or(u32::MAX),
        b.world.params.population.quorum_fraction,
    );
    let passed = match p.kind {
        ProposalKind::Recall { office, .. } => {
            let rule = b
                .world
                .constitution
                .offices
                .iter()
                .find(|o| o.kind == office)
                .map_or(RecallRule::Majority, |o| o.recall);
            recall_carries(&tally, rule)
        }
        _ => carries(&tally),
    };
    b.emit(Event::ProposalClosed {
        proposal: p.id,
        passed,
        tally,
    });
    if !passed {
        return;
    }
    match p.kind {
        ProposalKind::PolicyChange { patch } => {
            // Against the policy as it stands now, so an earlier close this 8j
            // is already in it and the later proposal overwrites (Q121).
            let policy = patch.apply_to(&b.world.policy);
            if policy.validate_against(&b.world.constitution).is_ok() {
                b.emit(Event::PolicyChanged {
                    policy: Box::new(policy),
                    by: Actor::Citizen(p.by),
                    proposal: Some(p.id),
                });
            }
        }
        ProposalKind::Recall { office, citizen } => {
            // The seat may have emptied since (absence, or the term's end
            // last cycle); a recall of an empty seat changes nothing.
            if holds(&b.world, citizen, office) {
                b.emit(Event::OfficeVacated {
                    office,
                    citizen,
                    reason: crate::world::VacancyReason::Recalled,
                });
            }
        }
        ProposalKind::Honor { citizen } => {
            // The record is the citizen's; one gone since (an emigrated
            // householder) has no record to write on.
            if b.world.citizens.contains_key(&citizen) {
                b.emit(Event::Honored {
                    citizen,
                    proposal: p.id,
                    cycle: b.cycle,
                });
            }
        }
        _ => {}
    }
}
