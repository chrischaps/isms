//! S2.2 done gate: elections closed at 8j, terms, no consecutive terms,
//! recall by the office's rule, auto-vacancy on absence, the no-candidate
//! re-run, the unfilled headline, and the tie-break's determinism.

#![allow(clippy::many_single_char_names)]

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::constitution::{OfficeKind, RecallRule};
use isms_core::event::Event;
use isms_core::governance::recall_carries;
use isms_core::ids::CitizenId;
use isms_core::kinds::CitizenKind;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder, assert_deterministic, check_golden};
use isms_core::world::{Ballot, ProposalKind, Tally, VacancyReason};
use proptest::prelude::*;
use std::collections::BTreeSet;

const COORD: OfficeKind = OfficeKind::Coordinator;

fn commune(humans: u32) -> Harness {
    WorldBuilder::new("commune").seed(11).humans(humans).build()
}

fn tick(h: &Harness) -> u32 {
    h.world.meta.tick
}

fn stand(h: &mut Harness, who: CitizenId) -> Result<(), RejectCode> {
    let t = tick(h);
    h.cmd(Envelope::citizen(who, Command::Stand { office: COORD }, t))
        .map(|_| ())
        .map_err(|e| e.code)
}

fn approve(h: &mut Harness, who: CitizenId, of: &[CitizenId]) -> Result<(), RejectCode> {
    let t = tick(h);
    h.cmd(Envelope::citizen(
        who,
        Command::Approve {
            office: COORD,
            candidates: of.iter().copied().collect(),
        },
        t,
    ))
    .map(|_| ())
    .map_err(|e| e.code)
}

fn seen(h: &mut Harness, who: CitizenId) {
    let t = tick(h);
    h.cmd(Envelope::citizen(who, Command::Seen, t))
        .expect("seen");
}

/// A session for each of `present`, then the cycle: a holder who is never
/// seen is vacated for absence after three cycles.
fn run_cycle_present(h: &mut Harness, present: &[CitizenId]) -> Vec<Event> {
    for &who in present {
        seen(h, who);
    }
    h.run_cycle()
}

fn holders(h: &Harness) -> Vec<CitizenId> {
    h.world
        .offices
        .holders
        .get(&COORD)
        .map(|s| s.iter().map(|x| x.citizen).collect())
        .unwrap_or_default()
}

fn taken(events: &[Event]) -> Vec<(CitizenId, u32, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::OfficeTaken {
                citizen,
                term_ends_cycle,
                approvals,
                ..
            } => Some((*citizen, *term_ends_cycle, *approvals)),
            _ => None,
        })
        .collect()
}

fn vacated(events: &[Event]) -> Vec<(CitizenId, VacancyReason)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::OfficeVacated {
                citizen, reason, ..
            } => Some((*citizen, *reason)),
            _ => None,
        })
        .collect()
}

fn count(events: &[Event], kind: &str) -> usize {
    events.iter().filter(|e| e.kind() == kind).count()
}

/// A fresh Commune whose first election is open: the builder does not seed
/// the epoch, so cycle 0's end opens it (closing at the end of cycle 1).
fn with_election(humans: u32) -> Harness {
    let mut h = commune(humans);
    let events = h.run_cycle();
    assert_eq!(count(&events, "ElectionOpened"), 1);
    assert!(h.world.offices.elections.contains_key(&COORD));
    h
}

#[test]
fn the_first_election_opens_at_epoch_start_and_seats_from_cycle_one() {
    let mut h = WorldBuilder::new("commune")
        .seed(3)
        .humans(4)
        .seed_epoch()
        .build();
    h.check_every_step = false;
    let e = &h.world.offices.elections[&COORD];
    assert_eq!((e.seats, e.opened_cycle, e.closes_cycle), (3, 0, 0));
    let (a, b, c, d) = (nth(&h, 0), nth(&h, 1), nth(&h, 2), nth(&h, 3));
    for who in [a, b, c, d] {
        stand(&mut h, who).unwrap();
    }
    approve(&mut h, a, &[a, b]).unwrap();
    approve(&mut h, b, &[b, c]).unwrap();
    approve(&mut h, c, &[c, a]).unwrap();
    approve(&mut h, d, &[]).unwrap(); // a ballot for nobody is still a ballot
    let events = h.run_cycle();
    h.check();
    // d has no approvals; a, b, c two each -> three seats, five-cycle terms.
    let seated = taken(&events);
    assert_eq!(seated.len(), 3);
    assert!(
        seated
            .iter()
            .all(|(_, ends, approvals)| *ends == 5 && *approvals == 2)
    );
    assert_eq!(holders(&h), vec![a, b, c]);
    assert!(!h.world.offices.elections.contains_key(&COORD));
    assert_eq!(h.world.cycle_of(h.world.meta.tick), 1);
    // The Q116 gate is now real: a coordinator may propose the rationing rule.
    assert!(
        h.cmd_dry(Envelope::citizen(
            a,
            Command::Propose {
                title: "ration".into(),
                text: "x".into(),
                kind: ProposalKind::PolicyChange {
                    patch: isms_core::policy::PolicyPatch {
                        rationing: Some(isms_core::policy::Rationing::Lottery),
                        ..Default::default()
                    },
                },
            },
            tick(&h),
        ))
        .is_ok()
    );
    // A losing candidate never took a seat and holds nothing.
    assert!(!h.world.offices.holds(d, COORD));
}

#[test]
fn stand_withdraw_and_approve_are_checked() {
    let mut h = commune(3);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    // No election yet: cycle 0 has not ended.
    assert_eq!(stand(&mut h, a), Err(RejectCode::NoElection));
    h.run_cycle();
    stand(&mut h, a).unwrap();
    assert_eq!(stand(&mut h, a), Err(RejectCode::AlreadyExists));
    assert_eq!(approve(&mut h, b, &[b]), Err(RejectCode::NotACandidate));
    approve(&mut h, b, &[a]).unwrap();
    // Withdrawing drops the candidacy and every approval of it.
    let t = tick(&h);
    h.cmd(Envelope::citizen(a, Command::Withdraw { office: COORD }, t))
        .unwrap();
    let e = &h.world.offices.elections[&COORD];
    assert!(e.candidates.is_empty());
    assert!(e.approvals[&b].is_empty());
    let err = h
        .cmd(Envelope::citizen(a, Command::Withdraw { office: COORD }, t))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotACandidate);
    // Householders never stand and never approve; Freeport has no offices.
    let mut c = WorldBuilder::new("commune")
        .seed(2)
        .humans(1)
        .householders(1)
        .build();
    let householder = nth(&c, 1);
    assert_eq!(c.citizen(householder).kind, CitizenKind::Householder);
    assert_eq!(stand(&mut c, householder), Err(RejectCode::NotAuthorized));
    let mut f = WorldBuilder::new("freeport").humans(1).build();
    let f0 = nth(&f, 0);
    assert_eq!(stand(&mut f, f0), Err(RejectCode::NotInThisSociety));
}

#[test]
fn no_consecutive_terms_refuses_the_just_ended_holder() {
    let mut h = with_election(3);
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    stand(&mut h, a).unwrap();
    h.run_cycle(); // cycle 1 ends: a seated through cycle 6
    assert_eq!(holders(&h), vec![a]);
    assert_eq!(h.world.offices.holders[&COORD][0].term_ends_cycle, 6);
    // The seats still empty get their own election at once.
    assert_eq!(h.world.offices.elections[&COORD].seats, 2);
    // A sitting holder cannot stand for the other seats.
    assert_eq!(stand(&mut h, a), Err(RejectCode::AlreadyExists));
    for _ in 0..3 {
        run_cycle_present(&mut h, &[a, b, c]); // nobody stands: re-runs through cycle 4
    }
    assert_eq!(h.world.cycle_of(h.world.meta.tick), 5);
    // At the end of cycle 5 the running election closes (re-run again) and
    // the seat ending at cycle 6 joins it: the holder's own seat is on the ballot.
    let events = run_cycle_present(&mut h, &[a, b, c]);
    assert_eq!(count(&events, "ElectionRerun"), 1);
    let e = &h.world.offices.elections[&COORD];
    assert_eq!((e.seats, e.closes_cycle), (3, 6));
    // a may not stand for a second term in a row; b and c may.
    assert_eq!(stand(&mut h, a), Err(RejectCode::ConsecutiveTerm));
    stand(&mut h, b).unwrap();
    let events = run_cycle_present(&mut h, &[a, b, c]); // cycle 6 ends: the term expires, b is seated
    assert_eq!(vacated(&events), vec![(a, VacancyReason::TermEnded)]);
    assert_eq!(holders(&h), vec![b]);
    assert_eq!(h.world.offices.record(a, COORD).terms, 1);
    assert_eq!(
        h.world.offices.record(a, COORD).last_full_term_ended,
        Some(6)
    );
    // The election that follows a's term still bars a (it opened this 8j,
    // after the term ended) ...
    assert_eq!(stand(&mut h, a), Err(RejectCode::ConsecutiveTerm));
    stand(&mut h, c).unwrap();
    run_cycle_present(&mut h, &[a, b, c]);
    assert_eq!(holders(&h), vec![b, c]);
    // ... and the next one, opened later, does not.
    stand(&mut h, a).unwrap();
    h.check();
}

#[test]
fn a_holder_absent_three_cycles_is_vacated_at_exactly_three() {
    let mut h = with_election(2);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    stand(&mut h, a).unwrap();
    h.run_cycle(); // a seated at the end of cycle 1
    assert_eq!(holders(&h), vec![a]);
    // Seen at the first tick of cycle 2, then never again.
    seen(&mut h, a);
    let seen_at = tick(&h);
    let tpc = h.world.params.time.ticks_per_cycle;
    assert_eq!(seen_at, 2 * tpc);
    assert_eq!(h.world.params.population.office_vacancy_absent_cycles, 3);
    let e2 = h.run_cycle();
    let e3 = h.run_cycle();
    assert!(vacated(&e2).is_empty() && vacated(&e3).is_empty());
    assert_eq!(holders(&h), vec![a]);
    let e4 = h.run_cycle(); // the third cycle without a session ends
    assert_eq!(vacated(&e4), vec![(a, VacancyReason::Absence)]);
    assert!(holders(&h).is_empty());
    // A partial term counts as served but never as a full one.
    let r = h.world.offices.record(a, COORD);
    assert_eq!((r.terms, r.last_full_term_ended), (1, None));
    // The seat goes straight back to the election already running for the
    // other two, which now covers all three; a may stand again at once.
    assert_eq!(h.world.offices.elections[&COORD].seats, 3);
    stand(&mut h, a).unwrap();
    stand(&mut h, b).unwrap();
    h.check();
}

#[test]
fn recall_needs_the_offices_rule_majority_or_two_thirds() {
    let t = |yes, no| Tally {
        yes,
        no,
        abstain: 0,
        cast: yes + no,
        quorum: 1,
        eligible: 5,
    };
    assert!(recall_carries(&t(2, 1), RecallRule::Majority));
    assert!(recall_carries(&t(2, 1), RecallRule::TwoThirds));
    assert!(recall_carries(&t(3, 2), RecallRule::Majority));
    assert!(!recall_carries(&t(3, 2), RecallRule::TwoThirds));
    assert!(!recall_carries(&t(0, 0), RecallRule::TwoThirds));
    assert!(!recall_carries(&t(1, 1), RecallRule::Majority));

    // End to end, both rules on the same three-to-two vote.
    for (rule, passes) in [(RecallRule::Majority, true), (RecallRule::TwoThirds, false)] {
        let mut h = WorldBuilder::new("commune")
            .seed(5)
            .humans(5)
            .with_preset(|p| p.constitution.offices[0].recall = rule)
            .build();
        h.run_cycle();
        let ids: Vec<CitizenId> = (0..5).map(|i| nth(&h, i)).collect();
        stand(&mut h, ids[0]).unwrap();
        h.run_cycle();
        assert_eq!(holders(&h), vec![ids[0]]);
        seen(&mut h, ids[0]);
        // Only a holder can be recalled.
        let t0 = tick(&h);
        let err = h
            .cmd(Envelope::citizen(
                ids[1],
                Command::Propose {
                    title: "out".into(),
                    text: "x".into(),
                    kind: ProposalKind::Recall {
                        office: COORD,
                        citizen: ids[1],
                    },
                },
                t0,
            ))
            .unwrap_err();
        assert_eq!(err.code, RejectCode::NotAnOfficeHolder);
        let events = h
            .cmd(Envelope::citizen(
                ids[1],
                Command::Propose {
                    title: "out".into(),
                    text: "x".into(),
                    kind: ProposalKind::Recall {
                        office: COORD,
                        citizen: ids[0],
                    },
                },
                t0,
            ))
            .unwrap();
        let Event::Proposed { proposal, .. } = events[0] else {
            panic!("{events:?}")
        };
        for (i, ballot) in [
            Ballot::Yes,
            Ballot::Yes,
            Ballot::Yes,
            Ballot::No,
            Ballot::No,
        ]
        .into_iter()
        .enumerate()
        {
            h.cmd(Envelope::citizen(
                ids[i],
                Command::Vote { proposal, ballot },
                t0,
            ))
            .unwrap();
        }
        let events = h.run_cycle();
        let closed = events.iter().find_map(|e| match e {
            Event::ProposalClosed { passed, tally, .. } => Some((*passed, *tally)),
            _ => None,
        });
        let (passed, tally) = closed.expect("closed");
        assert_eq!((tally.yes, tally.no), (3, 2));
        assert_eq!(passed, passes, "{rule:?}");
        if passes {
            assert_eq!(vacated(&events), vec![(ids[0], VacancyReason::Recalled)]);
            assert!(holders(&h).is_empty());
            // An election for the emptied seat opens the same cycle end.
            assert_eq!(h.world.offices.elections[&COORD].seats, 3);
        } else {
            assert_eq!(holders(&h), vec![ids[0]]);
        }
        h.check();
    }
}

#[test]
fn an_election_nobody_stands_in_reruns_each_cycle_and_the_office_is_headlined() {
    let mut h = with_election(2);
    assert_eq!(h.world.params.governance.unfilled_office_headline_cycles, 5);
    let opened = h.world.offices.elections[&COORD].opened_cycle;
    assert_eq!(opened, 0);
    // Cycles 1..4 end: re-runs, and the headline from the fifth short cycle on.
    for cycle in 1..=4u32 {
        let events = h.run_cycle();
        assert_eq!(count(&events, "ElectionRerun"), 1, "cycle {cycle}");
        assert_eq!(count(&events, "ElectionOpened"), 0, "cycle {cycle}");
        let headlined: Vec<u32> = events
            .iter()
            .filter_map(|e| match e {
                Event::OfficeUnfilled { cycles, .. } => Some(*cycles),
                _ => None,
            })
            .collect();
        assert_eq!(headlined, if cycle >= 4 { vec![cycle + 1] } else { vec![] });
        assert_eq!(h.world.offices.elections[&COORD].opened_cycle, 0);
        assert_eq!(h.world.offices.elections[&COORD].closes_cycle, cycle + 1);
    }
    // Once someone stands, the seat fills and the headline stops.
    let a = nth(&h, 0);
    stand(&mut h, a).unwrap();
    let events = h.run_cycle();
    assert_eq!(taken(&events).len(), 1);
    assert_eq!(count(&events, "OfficeUnfilled"), 1); // two seats still short
    assert!(h.world.offices.short_since.contains_key(&COORD));
    h.check();
}

fn tie_break_run(seed: u64) -> Vec<Event> {
    let mut h = WorldBuilder::new("commune").seed(seed).humans(5).build();
    h.check_every_step = false;
    let ids: Vec<CitizenId> = (0..5).map(|i| nth(&h, i)).collect();
    // ids[3] served a full term before: seated and expired by hand through
    // the ordinary events, so the record is the fold's own.
    h.apply(Event::OfficeTaken {
        office: COORD,
        citizen: ids[3],
        term_ends_cycle: 0,
        approvals: 1,
    });
    h.apply(Event::OfficeVacated {
        office: COORD,
        citizen: ids[3],
        reason: VacancyReason::TermEnded,
    });
    h.run_cycle(); // the election opens (closing at the end of cycle 1)
    // ids[3] is barred this round (Q118); the other four stand.
    assert_eq!(stand(&mut h, ids[3]), Err(RejectCode::ConsecutiveTerm));
    for &c in &ids[..3] {
        stand(&mut h, c).unwrap();
    }
    stand(&mut h, ids[4]).unwrap();
    // Everyone approves everyone: a four-way tie for three seats.
    for &v in &ids {
        approve(&mut h, v, &[ids[0], ids[1], ids[2], ids[4]]).unwrap();
    }
    h.run_cycle();
    h.check();
    h.log
}

#[test]
fn ties_break_by_fewer_past_terms_then_lower_id_deterministically() {
    let log = tie_break_run(9);
    // The seats the election filled: after its close, not the hand-seated term.
    let close_at = log
        .iter()
        .position(|e| matches!(e, Event::ElectionClosed { .. }))
        .expect("closed");
    let seated: Vec<CitizenId> = taken(&log[close_at..])
        .into_iter()
        .map(|(c, _, _)| c)
        .collect();
    let h = Harness::from_log(log.clone());
    let ids: Vec<CitizenId> = (0..5).map(|i| nth(&h, i)).collect();
    assert_eq!(seated, vec![ids[0], ids[1], ids[2]]);
    let closed = log.iter().find_map(|e| match e {
        Event::ElectionClosed { approvals, .. } => Some(approvals.clone()),
        _ => None,
    });
    assert_eq!(
        closed.unwrap(),
        vec![(ids[0], 5), (ids[1], 5), (ids[2], 5), (ids[4], 5)]
    );
    assert_deterministic(9, tie_break_run);
    check_golden("s2_2_commune_tied_election", &log);

    // Past terms come before the id: a returning holder loses the tie to a
    // newcomer with a higher id.
    let mut h = WorldBuilder::new("commune").seed(4).humans(3).build();
    h.check_every_step = false;
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    h.apply(Event::OfficeTaken {
        office: COORD,
        citizen: a,
        term_ends_cycle: 0,
        approvals: 1,
    });
    h.apply(Event::OfficeVacated {
        office: COORD,
        citizen: a,
        reason: VacancyReason::Recalled,
    });
    h.run_cycle();
    stand(&mut h, a).unwrap();
    stand(&mut h, b).unwrap();
    stand(&mut h, c).unwrap();
    for v in [a, b, c] {
        approve(&mut h, v, &[a, b, c]).unwrap();
    }
    let events = h.run_cycle();
    // Three seats, three candidates: everyone sits, but the rank puts a last.
    let closed = events.iter().find_map(|e| match e {
        Event::ElectionClosed { approvals, .. } => Some(approvals.clone()),
        _ => None,
    });
    assert_eq!(closed.unwrap(), vec![(b, 3), (c, 3), (a, 3)]);
    h.check();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// Seats never exceed `OfficeSpec.seats`, and no holder outlives
    /// `term_ends_cycle`, whatever the citizens do: stand, withdraw, approve,
    /// show up, or vanish.
    #[test]
    fn seats_are_bounded_and_terms_end(
        humans in 1u32..7,
        cycles in 1u32..12,
        steps in proptest::collection::vec((0usize..7, 0u8..4, 0usize..7), 0..40),
    ) {
        let mut h = WorldBuilder::new("commune").seed(u64::from(humans)).humans(humans).build();
        h.check_every_step = false;
        let ids = h.citizen_ids();
        let seats = h.world.constitution.offices[0].seats;
        let mut steps = steps.into_iter();
        for _ in 0..cycles {
            for (who, what, other) in steps.by_ref().take(4) {
                let who = ids[who % ids.len()];
                let other = ids[other % ids.len()];
                let t = h.world.meta.tick;
                let cmd = match what {
                    0 => Command::Stand { office: COORD },
                    1 => Command::Withdraw { office: COORD },
                    2 => Command::Approve { office: COORD, candidates: BTreeSet::from([other]) },
                    _ => Command::Seen,
                };
                // Rejections are the rules at work; only a panic is a bug.
                let _ = h.cmd(Envelope::citizen(who, cmd, t));
            }
            h.run_cycle();
            let cycle = h.world.cycle_of(h.world.meta.tick);
            let holders = h.world.offices.holders.get(&COORD).cloned().unwrap_or_default();
            prop_assert!(holders.len() <= seats as usize, "{} holders for {seats} seats", holders.len());
            for holder in &holders {
                prop_assert!(holder.term_ends_cycle >= cycle, "{holder:?} past its term at cycle {cycle}");
            }
            let distinct: BTreeSet<CitizenId> = holders.iter().map(|x| x.citizen).collect();
            prop_assert_eq!(distinct.len(), holders.len());
            // An election is open exactly when a seat is short or ends next cycle.
            let short = holders.len() < seats as usize
                || holders.iter().any(|x| x.term_ends_cycle == cycle);
            prop_assert_eq!(h.world.offices.elections.contains_key(&COORD), short);
        }
        h.check();
    }
}
