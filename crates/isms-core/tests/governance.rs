//! S2.1 done gate: typed proposals, one-citizen-one-vote ballots, quorum from
//! params, the 8j close, and `vote_default` executed at close.

#![allow(clippy::many_single_char_names)]

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::constitution::{OfficeKind, ProposalKindTag, Proposers};
use isms_core::event::{Actor, Event};
use isms_core::governance::{carries, quorum_needed};
use isms_core::ids::{CitizenId, ProposalId};
use isms_core::kinds::CitizenKind;
use isms_core::ledger::conservation_check;
use isms_core::policy::{MaterialsSplit, MonitoringPolicy, PolicyPatch, Rationing};
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder, assert_deterministic, check_golden};
use isms_core::world::{Ballot, ProposalKind, Tally, VoteDefault};
use proptest::prelude::*;

fn commune(humans: u32) -> Harness {
    WorldBuilder::new("commune").seed(7).humans(humans).build()
}

fn norm_patch(hours: u8) -> ProposalKind {
    ProposalKind::PolicyChange {
        patch: PolicyPatch {
            work_norm_hours: Some(hours),
            ..PolicyPatch::default()
        },
    }
}

fn propose(h: &mut Harness, who: CitizenId, title: &str, kind: ProposalKind) -> ProposalId {
    let events = h
        .cmd(Envelope::citizen(
            who,
            Command::Propose {
                title: title.into(),
                text: "because".into(),
                kind,
            },
            0,
        ))
        .unwrap_or_else(|e| panic!("propose {title}: {e}"));
    let Event::Proposed { proposal, .. } = events[0] else {
        panic!("{events:?}")
    };
    proposal
}

fn vote(h: &mut Harness, who: CitizenId, proposal: ProposalId, ballot: Ballot) {
    h.cmd(Envelope::citizen(
        who,
        Command::Vote { proposal, ballot },
        0,
    ))
    .unwrap_or_else(|e| panic!("vote: {e}"));
}

fn set_default(h: &mut Harness, who: CitizenId, vote_default: VoteDefault) {
    let mut plan = h.citizen(who).plan.clone();
    plan.vote_default = vote_default;
    h.apply(Event::PlanChanged {
        citizen: who,
        plan: Box::new(plan),
    });
}

/// Seat a coordinator through the event an election emits (S2.2), so the
/// fold check stays on; the election itself is `tests/offices.rs`.
fn seat_coordinator(h: &mut Harness, who: CitizenId) {
    h.apply(Event::OfficeTaken {
        office: OfficeKind::Coordinator,
        citizen: who,
        term_ends_cycle: 5,
        approvals: 1,
    });
}

fn closed(events: &[Event], proposal: ProposalId) -> (bool, Tally) {
    events
        .iter()
        .find_map(|e| match e {
            Event::ProposalClosed {
                proposal: p,
                passed,
                tally,
            } if *p == proposal => Some((*passed, *tally)),
            _ => None,
        })
        .expect("the proposal closed")
}

#[test]
fn the_constitution_gates_the_kind_and_the_proposer() {
    let mut f = WorldBuilder::new("freeport").humans(2).build();
    let err = f
        .cmd(Envelope::citizen(
            nth(&f, 0),
            Command::Propose {
                title: "hours".into(),
                text: String::new(),
                kind: ProposalKind::Resolution,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    assert!(f.capabilities().proposal_kinds.is_empty());

    let c = commune(3);
    let caps = c.capabilities();
    assert!(caps.proposal_kinds.contains(&ProposalKindTag::PolicyChange));
    assert!(!caps.proposal_kinds.contains(&ProposalKindTag::Disbursement));
    assert_eq!(caps.proposers, Proposers::Anyone);
    // An honor is a real motion since S2.3 (`tests/coordinator.rs`); the one
    // declared-only kind left, a disbursement, is refused as not yet
    // implemented rather than as absent where its tag is enabled.
    assert!(
        c.cmd_dry(Envelope::citizen(
            nth(&c, 0),
            Command::Propose {
                title: "honor".into(),
                text: String::new(),
                kind: ProposalKind::Honor {
                    citizen: nth(&c, 1),
                },
            },
            0,
        ))
        .is_ok()
    );
    let mut d = WorldBuilder::new("commune")
        .with_preset(|p| {
            p.constitution
                .proposal_kinds
                .insert(ProposalKindTag::Disbursement);
        })
        .humans(2)
        .build();
    let err = d
        .cmd(Envelope::citizen(
            nth(&d, 0),
            Command::Propose {
                title: "pay out".into(),
                text: String::new(),
                kind: ProposalKind::Disbursement {
                    org: isms_core::ids::OrgId(0),
                },
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotImplemented);

    // Office-holders only: nobody holds an office yet, so nobody proposes.
    let mut o = WorldBuilder::new("commune")
        .with_preset(|p| p.constitution.proposers = Proposers::OfficeHolders)
        .humans(2)
        .build();
    let err = o
        .cmd(Envelope::citizen(
            nth(&o, 0),
            Command::Propose {
                title: "hours".into(),
                text: "x".into(),
                kind: norm_patch(5),
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    let holder = nth(&o, 0);
    seat_coordinator(&mut o, holder);
    assert!(
        o.cmd_dry(Envelope::citizen(
            holder,
            Command::Propose {
                title: "hours".into(),
                text: "x".into(),
                kind: norm_patch(5),
            },
            0,
        ))
        .is_ok()
    );
}

#[test]
fn householders_dormant_citizens_and_the_cap_are_refused() {
    let mut h = WorldBuilder::new("commune")
        .seed(3)
        .humans(2)
        .householders(1)
        .build();
    let (a, hh) = (nth(&h, 0), nth(&h, 2));
    assert_eq!(h.citizen(hh).kind, CitizenKind::Householder);
    let err = h
        .cmd(Envelope::citizen(
            hh,
            Command::Propose {
                title: "t".into(),
                text: "x".into(),
                kind: ProposalKind::Resolution,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    let p = propose(&mut h, a, "minutes", ProposalKind::Resolution);
    let err = h
        .cmd(Envelope::citizen(
            hh,
            Command::Vote {
                proposal: p,
                ballot: Ballot::Yes,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
    // The cap: three open at once (presets/_base.toml), the fourth is refused.
    let cap = h.world.params.governance.open_proposals_per_citizen;
    assert_eq!(cap, 3);
    propose(&mut h, a, "second", ProposalKind::Resolution);
    propose(&mut h, a, "third", ProposalKind::Resolution);
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::Propose {
                title: "fourth".into(),
                text: "x".into(),
                kind: ProposalKind::Resolution,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::TooManyProposals);
    // Dormant: a citizen asleep neither proposes nor votes.
    h.apply(Event::CitizenDormant { citizen: a });
    let err = h
        .cmd(Envelope::citizen(
            a,
            Command::Vote {
                proposal: p,
                ballot: Ballot::Yes,
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::Dormant);
    // A title is required; a resolution needs its text; an empty patch moves nothing.
    let b = nth(&h, 1);
    for (title, text, kind) in [
        (" ", "x", ProposalKind::Resolution),
        ("t", " ", ProposalKind::Resolution),
        (
            "t",
            "x",
            ProposalKind::PolicyChange {
                patch: PolicyPatch::default(),
            },
        ),
    ] {
        let err = h
            .cmd(Envelope::citizen(
                b,
                Command::Propose {
                    title: title.into(),
                    text: text.into(),
                    kind,
                },
                0,
            ))
            .unwrap_err();
        assert_eq!(err.code, RejectCode::InvalidQuantity, "{title:?} {text:?}");
    }
}

#[test]
fn a_quorum_miss_fails_regardless_of_tally() {
    // Ten humans, quorum ceil(0.2 x 10) = 2. One yes, no defaults: it fails.
    let mut h = commune(10);
    let ids = h.citizen_ids();
    for id in &ids {
        set_default(&mut h, *id, VoteDefault::None);
    }
    let p = propose(&mut h, ids[0], "hours", norm_patch(5));
    vote(&mut h, ids[0], p, Ballot::Yes);
    let events = h.run_cycle();
    let (passed, tally) = closed(&events, p);
    assert!(!passed);
    assert_eq!(
        tally,
        Tally {
            yes: 1,
            no: 0,
            abstain: 0,
            cast: 1,
            quorum: 2,
            eligible: 10
        }
    );
    assert_eq!(h.world.policy.work_norm_hours, Some(6));
    assert!(h.world.proposals.is_empty());
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PolicyChanged { .. }))
    );

    // The same yes with one abstention beside it reaches the quorum and passes.
    let p = propose(&mut h, ids[0], "hours", norm_patch(5));
    vote(&mut h, ids[0], p, Ballot::Yes);
    vote(&mut h, ids[1], p, Ballot::Abstain);
    let events = h.run_cycle();
    let (passed, tally) = closed(&events, p);
    assert!(passed);
    assert_eq!((tally.cast, tally.abstain), (2, 1));
    assert_eq!(h.world.policy.work_norm_hours, Some(5));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::PolicyChanged { proposal: Some(x), by: Actor::Citizen(c), .. } if *x == p && *c == ids[0]
    )));
    h.check();
}

#[test]
fn follow_copies_exactly_one_hop() {
    let mut h = commune(4);
    let ids = h.citizen_ids();
    let (a, b, c, d) = (ids[0], ids[1], ids[2], ids[3]);
    // a votes; b follows a; c follows b; d follows nobody who voted.
    set_default(&mut h, b, VoteDefault::Follow(a));
    set_default(&mut h, c, VoteDefault::Follow(b));
    set_default(&mut h, d, VoteDefault::None);
    let p = propose(&mut h, a, "hours", norm_patch(4));
    vote(&mut h, a, p, Ballot::No);
    let events = h.run_cycle();
    let defaults: Vec<(CitizenId, Ballot)> = events
        .iter()
        .filter_map(|e| match e {
            Event::Voted {
                citizen,
                ballot,
                by_default: true,
                ..
            } => Some((*citizen, *ballot)),
            _ => None,
        })
        .collect();
    assert_eq!(defaults, vec![(b, Ballot::No), (c, Ballot::Abstain)]);
    let (passed, tally) = closed(&events, p);
    assert!(!passed);
    assert_eq!(
        (tally.yes, tally.no, tally.abstain, tally.cast),
        (0, 2, 1, 3)
    );

    // A ballot is replaceable until the close: b's own yes beats the default.
    let p = propose(&mut h, a, "hours", norm_patch(4));
    vote(&mut h, a, p, Ballot::No);
    vote(&mut h, b, p, Ballot::No);
    vote(&mut h, b, p, Ballot::Yes);
    assert_eq!(h.world.proposals[&p].ballots[&b], Ballot::Yes);
    let events = h.run_cycle();
    let (_, tally) = closed(&events, p);
    // c follows b, and this time b cast a ballot of their own: c copies the yes.
    assert_eq!((tally.yes, tally.no), (2, 1));
    h.check();
}

#[test]
fn an_out_of_constitution_policy_change_is_refused_at_propose() {
    // The Republic's legislature may change policy, but rationing is not a
    // Republic policy: the patch is refused before it is ever put to a vote.
    let mut r = WorldBuilder::new("republic")
        .with_preset(|p| {
            p.constitution.proposal_kinds = [ProposalKindTag::PolicyChange].into();
        })
        .humans(2)
        .build();
    let me = nth(&r, 0);
    seat_coordinator(&mut r, me);
    let err = r
        .cmd(Envelope::citizen(
            me,
            Command::Propose {
                title: "ration".into(),
                text: "x".into(),
                kind: ProposalKind::PolicyChange {
                    patch: PolicyPatch {
                        rationing: Some(Rationing::Lottery),
                        ..PolicyPatch::default()
                    },
                },
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    // A malformed split is refused the same way.
    let mut c = commune(2);
    let err = c
        .cmd(Envelope::citizen(
            nth(&c, 0),
            Command::Propose {
                title: "split".into(),
                text: "x".into(),
                kind: ProposalKind::PolicyChange {
                    patch: PolicyPatch {
                        materials_split: Some(MaterialsSplit {
                            wares: 0.9,
                            machines: 0.9,
                            dwellings: 0.0,
                        }),
                        ..PolicyPatch::default()
                    },
                },
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotInThisSociety);
    // The rationing rule is a coordinator's to propose (Q116).
    let err = c
        .cmd(Envelope::citizen(
            nth(&c, 0),
            Command::Propose {
                title: "ration".into(),
                text: "x".into(),
                kind: ProposalKind::PolicyChange {
                    patch: PolicyPatch {
                        rationing: Some(Rationing::Lottery),
                        ..PolicyPatch::default()
                    },
                },
            },
            0,
        ))
        .unwrap_err();
    assert_eq!(err.code, RejectCode::NotAuthorized);
}

#[test]
fn conservation_holds_across_a_policy_change_that_flips_rationing() {
    let mut h = WorldBuilder::new("commune")
        .seed(11)
        .humans(4)
        .seed_epoch()
        .build();
    let ids = h.citizen_ids();
    let humans: Vec<CitizenId> = ids
        .iter()
        .copied()
        .filter(|id| h.citizen(*id).kind == CitizenKind::Human)
        .collect();
    let me = humans[0];
    seat_coordinator(&mut h, me);
    assert_eq!(h.world.policy.rationing, Some(Rationing::NeedFirst));
    let p = propose(
        &mut h,
        me,
        "lottery",
        ProposalKind::PolicyChange {
            patch: PolicyPatch {
                rationing: Some(Rationing::Lottery),
                monitoring: Some(MonitoringPolicy::High),
                ..PolicyPatch::default()
            },
        },
    );
    for id in &humans {
        vote(&mut h, *id, p, Ballot::Yes);
    }
    let events = h.run_cycle();
    let (passed, _) = closed(&events, p);
    assert!(passed);
    assert_eq!(h.world.policy.rationing, Some(Rationing::Lottery));
    assert_eq!(h.world.policy.monitoring, MonitoringPolicy::High);
    conservation_check(&h.world).unwrap();
    // The new rule runs the next cycle's draws; the books still balance.
    h.run_cycle();
    conservation_check(&h.world).unwrap();
}

#[test]
fn two_changes_to_one_field_apply_in_id_order() {
    let mut h = commune(3);
    let ids = h.citizen_ids();
    let first = propose(&mut h, ids[0], "five", norm_patch(5));
    let second = propose(&mut h, ids[1], "seven", norm_patch(7));
    for id in &ids {
        vote(&mut h, *id, first, Ballot::Yes);
        vote(&mut h, *id, second, Ballot::Yes);
    }
    let events = h.run_cycle();
    let changes: Vec<(Option<ProposalId>, Option<u8>)> = events
        .iter()
        .filter_map(|e| match e {
            Event::PolicyChanged {
                proposal, policy, ..
            } => Some((*proposal, policy.work_norm_hours)),
            _ => None,
        })
        .collect();
    assert_eq!(
        changes,
        vec![(Some(first), Some(5)), (Some(second), Some(7))]
    );
    assert_eq!(h.world.policy.work_norm_hours, Some(7));
}

#[test]
fn a_resolution_is_the_record_only() {
    let mut h = commune(2);
    let ids = h.citizen_ids();
    let before = h.world.policy.clone();
    let p = propose(
        &mut h,
        ids[0],
        "we thank the millers",
        ProposalKind::Resolution,
    );
    vote(&mut h, ids[0], p, Ballot::Yes);
    let events = h.run_cycle();
    let (passed, _) = closed(&events, p);
    assert!(passed);
    assert_eq!(h.world.policy, before);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PolicyChanged { .. }))
    );
}

fn three_proposal_cycle(seed: u64) -> Vec<Event> {
    let mut h = WorldBuilder::new("commune")
        .seed(seed)
        .humans(5)
        .seed_epoch()
        .build();
    h.check_every_step = false;
    let humans: Vec<CitizenId> = h
        .citizen_ids()
        .into_iter()
        .filter(|id| h.citizen(*id).kind == CitizenKind::Human)
        .collect();
    set_default(&mut h, humans[3], VoteDefault::Follow(humans[0]));
    set_default(&mut h, humans[4], VoteDefault::None);
    let hours = propose(&mut h, humans[0], "five hours", norm_patch(5));
    let split = propose(
        &mut h,
        humans[1],
        "more machines",
        ProposalKind::PolicyChange {
            patch: PolicyPatch {
                materials_split: Some(MaterialsSplit {
                    wares: 0.4,
                    machines: 0.4,
                    dwellings: 0.2,
                }),
                ..PolicyPatch::default()
            },
        },
    );
    let minutes = propose(&mut h, humans[2], "minutes", ProposalKind::Resolution);
    vote(&mut h, humans[0], hours, Ballot::Yes);
    vote(&mut h, humans[1], hours, Ballot::No);
    vote(&mut h, humans[2], hours, Ballot::Yes);
    vote(&mut h, humans[0], split, Ballot::No);
    vote(&mut h, humans[1], split, Ballot::Yes);
    vote(&mut h, humans[2], minutes, Ballot::Yes);
    h.run_cycle();
    h.check();
    h.log
}

#[test]
fn golden_a_cycle_with_three_proposals() {
    let log = three_proposal_cycle(7);
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::ProposalClosed { .. }))
            .count(),
        3
    );
    assert_deterministic(7, three_proposal_cycle);
    check_golden("s2_1_commune_three_proposals", &log);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Ballots never exceed the eligible voters: only humans vote, each once,
    /// and the defaults fill only the humans who cast none.
    #[test]
    fn ballots_never_exceed_eligible(
        humans in 1u32..8,
        householders in 0u32..3,
        votes in proptest::collection::vec((0usize..8, 0u8..3), 0..16),
        defaults in proptest::collection::vec(0u8..3, 8),
    ) {
        let mut h = WorldBuilder::new("commune").seed(1).humans(humans).householders(householders).build();
        h.check_every_step = false;
        let ids = h.citizen_ids();
        let all: Vec<CitizenId> = ids.iter().copied().filter(|id| h.citizen(*id).kind == CitizenKind::Human).collect();
        for (i, id) in all.iter().enumerate() {
            let d = match defaults[i] {
                0 => VoteDefault::Abstain,
                1 => VoteDefault::Follow(all[(i + 1) % all.len()]),
                _ => VoteDefault::None,
            };
            set_default(&mut h, *id, d);
        }
        let p = propose(&mut h, all[0], "hours", norm_patch(5));
        for (who, b) in votes {
            let ballot = [Ballot::Yes, Ballot::No, Ballot::Abstain][usize::from(b)];
            let who = ids[who % ids.len()];
            let r = h.cmd(Envelope::citizen(who, Command::Vote { proposal: p, ballot }, 0));
            if h.citizen(who).kind == CitizenKind::Householder {
                prop_assert_eq!(r.unwrap_err().code, RejectCode::NotAuthorized);
            } else {
                prop_assert!(r.is_ok());
            }
        }
        let events = h.run_cycle();
        let (passed, t) = closed(&events, p);
        prop_assert_eq!(t.eligible, humans);
        prop_assert!(t.cast <= t.eligible);
        prop_assert_eq!(t.yes + t.no + t.abstain, t.cast);
        prop_assert_eq!(passed, carries(&t));
        h.check();
    }

    /// Quorum is monotone in `quorum_fraction`: a higher fraction never needs
    /// fewer ballots, and a vote that carries under it carries under any lower one.
    #[test]
    fn quorum_is_monotone_in_the_fraction(
        eligible in 0u32..500,
        lo in 0.0f64..=1.0,
        hi in 0.0f64..=1.0,
        yes in 0u32..6,
        no in 0u32..6,
        abstain in 0u32..6,
    ) {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        prop_assert!(quorum_needed(lo, eligible) <= quorum_needed(hi, eligible));
        prop_assert!(quorum_needed(hi, eligible) <= eligible);
        let t = |f: f64| Tally { yes, no, abstain, cast: yes + no + abstain, quorum: quorum_needed(f, eligible), eligible };
        if carries(&t(hi)) {
            prop_assert!(carries(&t(lo)));
        }
    }
}
