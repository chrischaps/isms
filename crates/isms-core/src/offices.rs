//! Offices (GDD §8.2, §8.3; TDD S2.2): elections, terms, rotation, recall,
//! vacancy. An election opens for the vacant seats of an office (and, one
//! cycle early, for the seats whose terms end next cycle) and closes at the
//! next cycle end (8j). Citizens stand and withdraw; voters approve any subset
//! of the candidates; the top `seats` by approvals win, ties broken by fewer
//! past terms and then the lower id (Q118). A holder serves through
//! `term_ends_cycle`, is auto-vacated after
//! `population.office_vacancy_absent_cycles` cycles without a session, and can
//! be recalled by a proposal that carries the office's `RecallRule`
//! (`governance.rs`). An election nobody stands in re-runs each cycle, and an
//! office short of holders for `governance.unfilled_office_headline_cycles`
//! cycles says so once per cycle. The engine never appoints anyone.
//!
//! `OfficeSpec` on the constitution is the whole rule: seats, term length,
//! whether consecutive terms are allowed, and the recall threshold.

use crate::command::{Command, Envelope, Reject, RejectCode};
use crate::constitution::{OfficeKind, OfficeSpec};
use crate::event::Event;
use crate::ids::CitizenId;
use crate::kinds::CitizenKind;
use crate::tick::TickBuilder;
use crate::world::{Citizen, Election, VacancyReason, World};
use std::collections::BTreeSet;

fn spec(world: &World, office: OfficeKind) -> Result<&OfficeSpec, Reject> {
    world
        .constitution
        .offices
        .iter()
        .find(|o| o.kind == office)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::NotInThisSociety,
                format!("{office:?} is not an office here"),
            )
        })
}

fn open_election(world: &World, office: OfficeKind) -> Result<&Election, Reject> {
    world.offices.elections.get(&office).ok_or_else(|| {
        Reject::new(
            RejectCode::NoElection,
            format!("no election is open for {office:?}"),
        )
    })
}

/// A human, non-dormant citizen: the only kind that stands or approves
/// (GDD 8.1: householders never vote and never hold office).
fn elector<'w>(world: &'w World, envelope: &Envelope<Command>) -> Result<&'w Citizen, Reject> {
    let actor = crate::command::acting_citizen(world, envelope)?;
    if actor.kind == CitizenKind::Householder {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "householders never vote and never hold office",
        ));
    }
    Ok(actor)
}

/// `Stand`: enter the open election for `office`.
pub fn stand(
    world: &World,
    envelope: &Envelope<Command>,
    office: OfficeKind,
) -> Result<Vec<Event>, Reject> {
    let actor = elector(world, envelope)?;
    let spec = spec(world, office)?;
    let election = open_election(world, office)?;
    if election.candidates.contains(&actor.id) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{} already stands for {office:?}", actor.id),
        ));
    }
    if let Some(seat) = world
        .offices
        .holders
        .get(&office)
        .and_then(|h| h.iter().find(|h| h.citizen == actor.id))
    {
        // A sitting holder stands only for the seat they are about to leave,
        // and only where the office allows a second term in a row.
        if seat.term_ends_cycle != election.closes_cycle {
            return Err(Reject::new(
                RejectCode::AlreadyExists,
                format!("{} already holds {office:?}", actor.id),
            ));
        }
        if !spec.consecutive {
            return Err(Reject::new(
                RejectCode::ConsecutiveTerm,
                format!("{office:?} bars a second term in a row"),
            ));
        }
    }
    // A full term that ended since this election opened bars the same
    // citizen from it (a partial term, by recall or absence, does not; Q118).
    if !spec.consecutive
        && world
            .offices
            .record(actor.id, office)
            .last_full_term_ended
            .is_some_and(|ended| ended >= election.opened_cycle)
    {
        return Err(Reject::new(
            RejectCode::ConsecutiveTerm,
            format!("{office:?} bars a second term in a row"),
        ));
    }
    Ok(vec![Event::CandidacyDeclared {
        office,
        citizen: actor.id,
    }])
}

/// `Withdraw`: leave the open election for `office`.
pub fn withdraw(
    world: &World,
    envelope: &Envelope<Command>,
    office: OfficeKind,
) -> Result<Vec<Event>, Reject> {
    let actor = elector(world, envelope)?;
    spec(world, office)?;
    let election = open_election(world, office)?;
    if !election.candidates.contains(&actor.id) {
        return Err(Reject::new(
            RejectCode::NotACandidate,
            format!("{} does not stand for {office:?}", actor.id),
        ));
    }
    Ok(vec![Event::CandidacyWithdrawn {
        office,
        citizen: actor.id,
    }])
}

/// `Approve`: cast or replace an approval ballot: any subset of the
/// candidates, the empty set included.
pub fn approve(
    world: &World,
    envelope: &Envelope<Command>,
    office: OfficeKind,
    candidates: &BTreeSet<CitizenId>,
) -> Result<Vec<Event>, Reject> {
    let actor = elector(world, envelope)?;
    spec(world, office)?;
    let election = open_election(world, office)?;
    if let Some(stranger) = candidates.iter().find(|c| !election.candidates.contains(c)) {
        return Err(Reject::new(
            RejectCode::NotACandidate,
            format!("{stranger} does not stand for {office:?}"),
        ));
    }
    Ok(vec![Event::Approved {
        office,
        citizen: actor.id,
        candidates: candidates.clone(),
    }])
}

/// The candidates of `election` in rank order with their approvals: most
/// approvals first, then fewer past terms in this office, then the lower id
/// (Q118). Candidates who went dormant since standing are left out.
#[must_use]
pub fn ranked(world: &World, election: &Election) -> Vec<(CitizenId, u32)> {
    let mut rows: Vec<(CitizenId, u32, u32)> = election
        .candidates
        .iter()
        .filter(|c| world.citizens.get(c).is_some_and(|c| !c.dormant))
        .map(|&c| {
            let approvals = election
                .approvals
                .values()
                .filter(|set| set.contains(&c))
                .count();
            let terms = world.offices.record(c, election.office).terms;
            (c, u32::try_from(approvals).unwrap_or(u32::MAX), terms)
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));
    rows.into_iter().map(|(c, a, _)| (c, a)).collect()
}

/// The seats an election closing at the end of `cycle + 1` must fill: what
/// is empty now plus what empties at that close.
fn seats_to_fill(world: &World, spec: &OfficeSpec, cycle: crate::ids::Cycle) -> u32 {
    let holders = world.offices.holders.get(&spec.kind);
    let filled = holders.map_or(0, |h| u32::try_from(h.len()).unwrap_or(u32::MAX));
    let ending_next = holders.map_or(0, |h| {
        u32::try_from(h.iter().filter(|h| h.term_ends_cycle == cycle + 1).count())
            .unwrap_or(u32::MAX)
    });
    spec.seats.saturating_sub(filled) + ending_next
}

/// Whether a holder has gone `absent_cycles` cycles without a session at
/// the end of `tick` (the dormancy arithmetic in `plan.rs`).
fn absent(citizen: &Citizen, tick: crate::ids::Tick, absent_cycles: u32, tpc: u32) -> bool {
    citizen.dormant || tick + 1 >= citizen.last_seen_tick + absent_cycles * tpc
}

/// Step 8j, after the proposals close: for every office in the constitution,
/// in its order, (1) terms that end this cycle expire, (2) absent holders
/// vacate, (3) an election due closes (winners seated, or a re-run when nobody
/// stood), (4) an election opens for any vacancy or for the seats whose terms
/// end next cycle, (5) an office short for long enough is headlined.
pub fn cycle_end_8j_offices(b: &mut TickBuilder) {
    let cycle = b.cycle;
    let tick = b.tick;
    let specs = b.world.constitution.offices.clone();
    let p = &b.world.params;
    let absent_cycles = p.population.office_vacancy_absent_cycles;
    let tpc = p.time.ticks_per_cycle;
    let headline_after = p.governance.unfilled_office_headline_cycles;

    for spec in &specs {
        let office = spec.kind;

        // (1) and (2): the seats that empty this cycle end.
        let vacating: Vec<(CitizenId, VacancyReason)> = b
            .world
            .offices
            .holders
            .get(&office)
            .map(|seats| {
                seats
                    .iter()
                    .filter_map(|h| {
                        if h.term_ends_cycle <= cycle {
                            Some((h.citizen, VacancyReason::TermEnded))
                        } else if b
                            .world
                            .citizens
                            .get(&h.citizen)
                            .is_none_or(|c| absent(c, tick, absent_cycles, tpc))
                        {
                            Some((h.citizen, VacancyReason::Absence))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (citizen, reason) in vacating {
            b.emit(Event::OfficeVacated {
                office,
                citizen,
                reason,
            });
        }

        // (3): the election due now.
        if let Some(election) = b.world.offices.elections.get(&office).cloned()
            && election.closes_cycle <= cycle
        {
            let rows = ranked(&b.world, &election);
            if rows.is_empty() {
                b.emit(Event::ElectionRerun {
                    office,
                    seats: seats_to_fill(&b.world, spec, cycle),
                    closes_cycle: cycle + 1,
                });
            } else {
                let vacant = spec.seats.saturating_sub(b.world.offices.filled(office));
                let winners = usize::try_from(election.seats.min(vacant)).unwrap_or(0);
                b.emit(Event::ElectionClosed {
                    office,
                    approvals: rows.clone(),
                });
                for (citizen, approvals) in rows.into_iter().take(winners) {
                    b.emit(Event::OfficeTaken {
                        office,
                        citizen,
                        term_ends_cycle: cycle + spec.term_cycles,
                        approvals,
                    });
                }
            }
        }

        // (4): open for what is empty now and what empties next cycle.
        if !b.world.offices.elections.contains_key(&office) {
            let seats = seats_to_fill(&b.world, spec, cycle);
            if seats > 0 {
                b.emit(Event::ElectionOpened {
                    office,
                    seats,
                    closes_cycle: cycle + 1,
                });
            }
        }

        // (5): the headline.
        if b.world.offices.filled(office) < spec.seats
            && let Some(&since) = b.world.offices.short_since.get(&office)
        {
            let cycles = cycle.saturating_sub(since) + 1;
            if cycles >= headline_after {
                b.emit(Event::OfficeUnfilled { office, cycles });
            }
        }
    }
}
