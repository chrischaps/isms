//! S2.4 done gate: a member-owned org's treasury and pantry move by its
//! members' vote (`ProposalKind::Disbursement`, Q122), a majority of the
//! membership carries at once, a split membership lapses at the cycle end, the
//! manager's direct treasury `Transfer` is refused, and a carry the org can no
//! longer cover closes failed (Q132).

#![allow(clippy::many_single_char_names)]

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::explain::RuleId;
use isms_core::ids::{CitizenId, OrgId, ProposalId};
use isms_core::kinds::{Good, OrgKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Ballot, ProposalKind};

const ORG: OrgId = OrgId(0);

/// A Freeport with `humans` humans, the first `members` of them members of a
/// funded association whose manager is the first.
fn society(humans: u32, members: usize) -> Harness {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000;
        })
        .humans(humans)
        .org(OrgKind::Association, "Mutual Aid")
        .org_inventory(0, Good::Food, 20)
        .build();
    for i in 0..members {
        h.apply(Event::MemberAdmitted {
            org: ORG,
            citizen: nth(&h, i),
        });
    }
    h.apply(Event::ManagerAppointed {
        org: ORG,
        citizen: Some(nth(&h, 0)),
    });
    h.apply(Event::Seeded {
        holder: Holder::Org(ORG),
        asset: Asset::Money(Money::credits(500)),
    });
    h
}

fn disbursement(to: CitizenId, asset: Asset) -> Command {
    Command::Propose {
        title: String::new(),
        text: String::new(),
        kind: ProposalKind::Disbursement {
            org: ORG,
            to: Party::Citizen(to),
            asset,
        },
    }
}

fn propose(
    h: &mut Harness,
    who: CitizenId,
    to: CitizenId,
    asset: Asset,
) -> Result<Vec<Event>, RejectCode> {
    let t = h.world.meta.tick;
    h.cmd(Envelope::citizen(who, disbursement(to, asset), t))
        .map_err(|e| e.code)
}

fn proposal_of(events: &[Event]) -> ProposalId {
    let Event::Proposed { proposal, .. } = events[0] else {
        panic!("{:?}", events[0].kind())
    };
    proposal
}

fn vote(
    h: &mut Harness,
    who: CitizenId,
    proposal: ProposalId,
    ballot: Ballot,
) -> Result<Vec<Event>, RejectCode> {
    let t = h.world.meta.tick;
    h.cmd(Envelope::citizen(
        who,
        Command::Vote { proposal, ballot },
        t,
    ))
    .map_err(|e| e.code)
}

fn kinds(events: &[Event]) -> Vec<&'static str> {
    events.iter().map(Event::kind).collect()
}

#[test]
fn a_majority_of_the_members_moves_the_treasury() {
    let mut h = society(4, 3);
    let (a, b, d) = (nth(&h, 0), nth(&h, 1), nth(&h, 3));
    let before = h.citizen(d).household.balance;
    let ev = propose(&mut h, a, d, Asset::Money(Money::credits(100))).unwrap();
    assert_eq!(kinds(&ev), ["Proposed"]);
    let p = proposal_of(&ev);
    assert!(h.world.proposals.contains_key(&p));
    // One yes of three members is not a majority.
    let ev = vote(&mut h, a, p, Ballot::Yes).unwrap();
    assert_eq!(kinds(&ev), ["Voted"]);
    // The second yes is: the vote closes and the money moves, with its Explain.
    let ev = vote(&mut h, b, p, Ballot::Yes).unwrap();
    assert_eq!(kinds(&ev), ["Voted", "ProposalClosed", "Disbursed"]);
    let Event::ProposalClosed { passed, tally, .. } = &ev[1] else {
        panic!()
    };
    assert!(passed);
    assert_eq!((tally.yes, tally.eligible, tally.quorum), (2, 3, 2));
    let Event::Disbursed { explain, .. } = &ev[2] else {
        panic!()
    };
    assert_eq!(explain.rule, RuleId::Disbursement);
    assert_eq!(h.world.orgs[&ORG].treasury, Money::credits(400));
    assert_eq!(h.citizen(d).household.balance, before + Money::credits(100));
    assert!(!h.world.proposals.contains_key(&p));
    h.check();
}

#[test]
fn the_manager_cannot_move_a_member_owned_treasury() {
    let h = society(3, 3);
    let (a, c) = (nth(&h, 0), nth(&h, 2));
    for asset in [Asset::Money(Money::credits(10)), Asset::Good(Good::Food, 1)] {
        let r = h.cmd_dry(
            Envelope::citizen(
                a,
                Command::Transfer {
                    to: Party::Citizen(c),
                    asset,
                    memo: "alms".into(),
                },
                0,
            )
            .on_behalf_of(ORG),
        );
        assert_eq!(r.unwrap_err().code, RejectCode::NotAuthorized);
    }
    assert_eq!(h.world.orgs[&ORG].treasury, Money::credits(500));
}

#[test]
fn only_members_propose_and_vote() {
    let mut h = society(3, 2);
    let (a, c) = (nth(&h, 0), nth(&h, 2));
    assert_eq!(
        propose(&mut h, c, a, Asset::Money(Money::credits(1))).unwrap_err(),
        RejectCode::NotParty
    );
    let p = proposal_of(&propose(&mut h, a, c, Asset::Money(Money::credits(1))).unwrap());
    assert_eq!(
        vote(&mut h, c, p, Ballot::Yes).unwrap_err(),
        RejectCode::NotParty
    );
    // The org cannot pay itself, and cannot promise what it lacks.
    let t = h.world.meta.tick;
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::Propose {
            title: String::new(),
            text: String::new(),
            kind: ProposalKind::Disbursement {
                org: ORG,
                to: Party::Org(ORG),
                asset: Asset::Money(Money::credits(1)),
            },
        },
        t,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::SelfDeal);
    assert_eq!(
        propose(&mut h, a, c, Asset::Money(Money::credits(10_000))).unwrap_err(),
        RejectCode::InsufficientFunds
    );
    assert_eq!(
        propose(&mut h, a, c, Asset::Good(Good::Wares, 1)).unwrap_err(),
        RejectCode::InsufficientGoods
    );
}

#[test]
fn a_lone_member_carries_at_once() {
    let mut h = society(2, 1);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    let ev = propose(&mut h, a, b, Asset::Good(Good::Food, 5)).unwrap();
    assert_eq!(kinds(&ev), ["Proposed", "ProposalClosed", "Disbursed"]);
    assert_eq!(h.world.orgs[&ORG].inventory[&Good::Food], 15);
    assert_eq!(h.citizen(b).household.pantry[&Good::Food], 5);
    assert!(h.world.proposals.is_empty());
    h.check();
}

#[test]
fn a_split_membership_lapses_at_the_cycle_end_and_a_majority_no_closes_it() {
    let mut h = society(3, 3);
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    let p = proposal_of(&propose(&mut h, a, c, Asset::Money(Money::credits(100))).unwrap());
    vote(&mut h, a, p, Ballot::Yes).unwrap();
    let ev = h.run_cycle();
    let closed: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            Event::ProposalClosed {
                proposal,
                passed,
                tally,
            } if *proposal == p => Some((*passed, *tally)),
            _ => None,
        })
        .collect();
    assert_eq!(closed.len(), 1);
    assert!(!closed[0].0, "no majority in the cycle: it lapses");
    assert_eq!(
        (closed[0].1.yes, closed[0].1.cast, closed[0].1.eligible),
        (1, 1, 3)
    );
    assert!(
        !ev.iter().any(|e| matches!(
            e,
            Event::Voted {
                by_default: true,
                ..
            }
        )),
        "the assembly's vote_default does not fill a members' vote"
    );
    assert_eq!(h.world.orgs[&ORG].treasury, Money::credits(500));
    // Two noes of three close it failed at once.
    let p = proposal_of(&propose(&mut h, a, c, Asset::Money(Money::credits(100))).unwrap());
    vote(&mut h, b, p, Ballot::No).unwrap();
    let ev = vote(&mut h, c, p, Ballot::No).unwrap();
    assert_eq!(kinds(&ev), ["Voted", "ProposalClosed"]);
    assert!(matches!(ev[1], Event::ProposalClosed { passed: false, .. }));
    h.check();
}

/// Q132: two motions on the same 500 credits; the one that carries second
/// finds the treasury empty and closes failed.
#[test]
fn a_carry_the_treasury_cannot_cover_closes_failed() {
    let mut h = society(3, 2);
    let (a, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    let first = proposal_of(&propose(&mut h, a, c, Asset::Money(Money::credits(500))).unwrap());
    let second = proposal_of(&propose(&mut h, b, c, Asset::Money(Money::credits(500))).unwrap());
    vote(&mut h, a, second, Ballot::Yes).unwrap();
    let ev = vote(&mut h, b, second, Ballot::Yes).unwrap();
    assert_eq!(kinds(&ev), ["Voted", "ProposalClosed", "Disbursed"]);
    vote(&mut h, a, first, Ballot::Yes).unwrap();
    let ev = vote(&mut h, b, first, Ballot::Yes).unwrap();
    assert_eq!(kinds(&ev), ["Voted", "ProposalClosed"]);
    assert!(matches!(ev[1], Event::ProposalClosed { passed: false, .. }));
    assert_eq!(h.world.orgs[&ORG].treasury, Money::ZERO);
    h.check();
}

/// The Commune has associations but no money: goods disburse, credits do not
/// exist. The assembly's `proposal_kinds` does not gate a members' vote.
#[test]
fn a_moneyless_association_disburses_goods_only() {
    let mut h = WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(2)
        .org(OrgKind::Association, "The Guild")
        .org_inventory(0, Good::Wares, 8)
        .build();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    h.apply(Event::MemberAdmitted {
        org: ORG,
        citizen: a,
    });
    h.apply(Event::MemberAdmitted {
        org: ORG,
        citizen: b,
    });
    assert!(
        !h.capabilities()
            .proposal_kinds
            .contains(&isms_core::constitution::ProposalKindTag::Disbursement)
    );
    assert_eq!(
        propose(&mut h, a, b, Asset::Money(Money::credits(1))).unwrap_err(),
        RejectCode::NotInThisSociety
    );
    let p = proposal_of(&propose(&mut h, a, b, Asset::Good(Good::Wares, 3)).unwrap());
    vote(&mut h, a, p, Ballot::Yes).unwrap();
    let ev = vote(&mut h, b, p, Ballot::Yes).unwrap();
    assert_eq!(kinds(&ev), ["Voted", "ProposalClosed", "Disbursed"]);
    assert_eq!(h.world.orgs[&ORG].inventory[&Good::Wares], 5);
    assert_eq!(h.citizen(b).household.pantry[&Good::Wares], 3);
    h.check();
}
