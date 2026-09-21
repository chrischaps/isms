//! S2.9's gate: the Commune's copy renders every governance event from
//! fixtures, Freeport's admission vote reads in its own voice, and a preset
//! without copy is refused by name before any row exists.

use crate::chronicle::{Projector, Templates, check_copy};
use isms_core::WORKSPACE_PRESETS_DIR;
use isms_core::config::load_preset;
use isms_core::constitution::OfficeKind;
use isms_core::event::{Actor, CycleAggregates, Event};
use isms_core::ids::{CitizenId, OrgId, ProposalId, WorkplaceId};
use isms_core::kinds::{CitizenKind, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Party};
use isms_core::money::Money;
use isms_core::policy::{MaterialsSplit, MonitoringPolicy, Rationing};
use isms_core::world::{Ownership, ProposalKind, Tally, VacancyReason};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn presets() -> &'static Path {
    Path::new(WORKSPACE_PRESETS_DIR)
}

fn commune() -> Projector {
    Projector::new(Templates::load(presets(), "commune").unwrap())
}

/// A projector that has seen the society's birth and two people: ids read as names.
fn peopled(preset: &str) -> Projector {
    let p = load_preset(presets(), preset).unwrap();
    let mut proj = Projector::new(Templates::load(presets(), preset).unwrap());
    let created = Event::SocietyCreated {
        society_id: 1,
        seed: 1,
        preset: Box::new(p),
    };
    assert!(proj.observe(&created, 0, 0, 0).is_empty());
    for (id, handle) in [(41, "marlow"), (42, "ines")] {
        let joined = Event::CitizenJoined {
            citizen: CitizenId(id),
            handle: handle.into(),
            kind: CitizenKind::Human,
            endowment: Money(0),
            dwelling: None,
            explain: None,
        };
        proj.observe(&joined, 1, 0, 0);
    }
    let founded = Event::OrgFounded {
        org: OrgId(7),
        kind: OrgKind::Association,
        name: "The Readers".into(),
        founder: Some(CitizenId(41)),
        ownership: Ownership::Members,
        manager: Some(CitizenId(41)),
        fee_burned: Money(0),
    };
    proj.observe(&founded, 2, 0, 0);
    proj
}

fn explain() -> isms_core::explain::Explain {
    isms_core::explain::Explain::new(
        isms_core::explain::RuleId::LaborOutput,
        "members voted",
        5_u32,
    )
}

fn lines(p: &mut Projector, e: &Event) -> Vec<String> {
    p.observe(e, 10, 5, 1)
        .into_iter()
        .map(|h| h.headline)
        .collect()
}

fn tally(yes: u32, no: u32, cast: u32, quorum: u32) -> Tally {
    Tally {
        yes,
        no,
        abstain: cast - yes - no,
        cast,
        quorum,
        eligible: 6,
    }
}

/// The card's gate: every governance event of S2.1–S2.4 makes a headline in
/// the Commune's words, and the words are the ones the copy carries.
#[test]
#[allow(clippy::too_many_lines)]
fn commune_copy_renders_every_governance_event() {
    let mut p = peopled("commune");
    let proposed = Event::Proposed {
        proposal: ProposalId(1),
        by: CitizenId(41),
        title: "Honor Ines".into(),
        text: "for the hours".into(),
        kind: ProposalKind::Honor {
            citizen: CitizenId(42),
        },
        closes_cycle: 3,
    };
    assert_eq!(
        lines(&mut p, &proposed),
        ["marlow moves before the assembly: Honor Ines. The vote closes at the end of day 3."]
    );
    // A ballot is not news.
    let voted = Event::Voted {
        proposal: ProposalId(1),
        citizen: CitizenId(42),
        ballot: isms_core::world::Ballot::Yes,
        by_default: false,
    };
    assert!(lines(&mut p, &voted).is_empty());
    // The three outcomes, keyed on the tally.
    let passed = Event::ProposalClosed {
        proposal: ProposalId(1),
        passed: true,
        tally: tally(4, 1, 5, 2),
    };
    assert_eq!(
        lines(&mut p, &passed),
        ["The assembly carries Honor Ines, 4 to 1."]
    );
    let failed = Event::ProposalClosed {
        proposal: ProposalId(1),
        passed: false,
        tally: tally(1, 4, 5, 2),
    };
    assert_eq!(
        lines(&mut p, &failed),
        ["The assembly declines Honor Ines, 4 against 1."]
    );
    let lapsed = Event::ProposalClosed {
        proposal: ProposalId(1),
        passed: false,
        tally: tally(1, 0, 1, 2),
    };
    assert_eq!(
        lines(&mut p, &lapsed),
        ["Honor Ines lapses: 1 voted and 2 were needed."]
    );
    let honored = Event::Honored {
        citizen: CitizenId(42),
        proposal: ProposalId(1),
        cycle: 1,
    };
    assert_eq!(
        lines(&mut p, &honored),
        ["The assembly honors ines. The line is on their record for good."]
    );

    // Offices (S2.2).
    let office = OfficeKind::Coordinator;
    let opened = Event::ElectionOpened {
        office,
        seats: 3,
        closes_cycle: 2,
    };
    assert_eq!(
        lines(&mut p, &opened),
        [
            "An election opens for the office of coordinator: 3 seats, closing at the end of day 2. Any citizen may stand."
        ]
    );
    let stood = Event::CandidacyDeclared {
        office,
        citizen: CitizenId(41),
    };
    assert_eq!(lines(&mut p, &stood), ["marlow stands for coordinator."]);
    let withdrew = Event::CandidacyWithdrawn {
        office,
        citizen: CitizenId(42),
    };
    assert_eq!(
        lines(&mut p, &withdrew),
        ["ines withdraws from the coordinator election."]
    );
    let approved = Event::Approved {
        office,
        citizen: CitizenId(42),
        candidates: BTreeSet::from([CitizenId(41)]),
    };
    assert!(lines(&mut p, &approved).is_empty());
    let counted = Event::ElectionClosed {
        office,
        approvals: vec![(CitizenId(41), 2)],
    };
    assert_eq!(
        lines(&mut p, &counted),
        ["The coordinator election is counted."]
    );
    let seated = Event::OfficeTaken {
        office,
        citizen: CitizenId(41),
        term_ends_cycle: 7,
        approvals: 2,
    };
    assert_eq!(
        lines(&mut p, &seated),
        ["marlow is seated as coordinator through day 7, with 2 approvals."]
    );
    for (reason, expect) in [
        (
            VacancyReason::TermEnded,
            "marlow's term as coordinator has run. The seat passes on; nobody sits two terms running.",
        ),
        (
            VacancyReason::Absence,
            "marlow has not been seen for days. Their coordinator seat is declared vacant.",
        ),
        (
            VacancyReason::Recalled,
            "The assembly recalls marlow from coordinator.",
        ),
    ] {
        let vacated = Event::OfficeVacated {
            office,
            citizen: CitizenId(41),
            reason,
        };
        assert_eq!(lines(&mut p, &vacated), [expect]);
    }
    let rerun = Event::ElectionRerun {
        office,
        seats: 2,
        closes_cycle: 4,
    };
    assert_eq!(
        lines(&mut p, &rerun),
        ["Nobody stood for coordinator. The election runs one more day for 2 seats."]
    );
    let unfilled = Event::OfficeUnfilled { office, cycles: 5 };
    assert_eq!(
        lines(&mut p, &unfilled),
        ["The Commune has no coordinators, or too few. A seat has stood empty for 5 days."]
    );

    // The coordinator's powers (S2.3).
    let plan = Event::PlanPublished {
        cycle: 2,
        targets: BTreeMap::from([(WorkplaceId(3), 40.0)]),
        by: Actor::Citizen(CitizenId(41)),
    };
    assert_eq!(
        lines(&mut p, &plan),
        ["marlow publishes the Plan for day 2. It asks; it does not command."]
    );
    let opened = Event::WorkplaceOpened {
        workplace: WorkplaceId(19),
        org: OrgId(1),
        kind: WorkplaceKind::MachineShop,
        slot: None,
        materials_consumed: 20,
        by: CitizenId(41),
    };
    assert_eq!(
        lines(&mut p, &opened),
        [
            "marlow opens the machine shop #19 for the collective on the land, 20 Materials from the Store."
        ]
    );
    let closed = Event::WorkplaceClosed {
        workplace: WorkplaceId(19),
        org: OrgId(1),
        slot: None,
        machines_returned: 2,
        by: CitizenId(41),
    };
    assert_eq!(
        lines(&mut p, &closed),
        [
            "marlow closes the machine shop #19. Its 2 machines go back to the collective's stock; its slot is free."
        ]
    );

    // An association's disbursement (S2.4).
    let disbursed = Event::Disbursed {
        proposal: ProposalId(2),
        org: OrgId(7),
        to: Party::Citizen(CitizenId(42)),
        asset: Asset::Good(Good::Food, 5),
        explain: explain(),
    };
    assert_eq!(
        lines(&mut p, &disbursed),
        ["The members of The Readers vote 5 food from their pantry to ines."]
    );
    // To an org, the copy has no words: skipped, not garbled.
    let to_org = Event::Disbursed {
        proposal: ProposalId(2),
        org: OrgId(7),
        to: Party::Org(OrgId(7)),
        asset: Asset::Good(Good::Food, 5),
        explain: explain(),
    };
    assert!(lines(&mut p, &to_org).is_empty());
}

/// One line per policy field that moved, read against the policy in force
/// since `SocietyCreated`; the rationing rule and the watch in words.
#[test]
fn policy_change_is_narrated_field_by_field() {
    let mut p = peopled("commune");
    let base = load_preset(presets(), "commune").unwrap().policy;
    let mut next = base.clone();
    next.work_norm_hours = Some(8);
    next.rationing = Some(Rationing::Lottery);
    let changed = Event::PolicyChanged {
        policy: Box::new(next.clone()),
        by: Actor::System,
        proposal: Some(ProposalId(3)),
    };
    assert_eq!(
        lines(&mut p, &changed),
        [
            "The assembly votes the rule for a short shelf: the lot decides whose request is served whole.",
            "The assembly sets the norm at 8 hours a day, from 6.",
        ]
    );
    // The same policy again moves nothing.
    assert!(lines(&mut p, &changed).is_empty());
    let mut again = next;
    again.monitoring = MonitoringPolicy::High;
    again.materials_split = Some(MaterialsSplit {
        wares: 0.2,
        machines: 0.5,
        dwellings: 0.3,
    });
    let changed = Event::PolicyChanged {
        policy: Box::new(again),
        by: Actor::System,
        proposal: Some(ProposalId(4)),
    };
    assert_eq!(
        lines(&mut p, &changed),
        [
            "The assembly moves the Materials split: 20% to Wares, 50% to Machines, 30% to Dwellings.",
            "The assembly votes to meter every hour. The Ledger will now say exactly what each of us made.",
        ]
    );
    // Before any `SocietyCreated`, there is nothing to read the change against.
    let mut fresh = commune();
    assert!(lines(&mut fresh, &changed).is_empty());
}

/// The night lines: a bare shelf, the verdict on need, hunger.
#[test]
fn night_lines_name_the_bare_shelf_and_the_verdict() {
    let mut p = peopled("commune");
    let night = |stock: &[(Good, u32)], rate: f64, hardship: u32| Event::CycleClosed {
        cycle: 1,
        aggregates: CycleAggregates {
            population: 41,
            need_fulfillment_rate: rate,
            hardship_count: hardship,
            store_stock: stock.iter().copied().collect(),
            ..Default::default()
        },
        low_population_cycles: 0,
        reached_floor: false,
    };
    assert_eq!(
        lines(
            &mut p,
            &night(&[(Good::Food, 12), (Good::Wares, 0)], 1.0, 0)
        ),
        [
            "The Store has no wares on the shelf tonight.",
            "Everyone ate and everyone was housed today.",
        ]
    );
    assert_eq!(
        lines(&mut p, &night(&[(Good::Food, 0)], 0.875, 3)),
        [
            "3 went hungry today. The Store fell short.",
            "The Store has no food on the shelf tonight.",
            "The Store reached 88% of need today.",
        ]
    );
    // Freeport has no shelf and no verdict: nothing new at a quiet close.
    let mut f = peopled("freeport");
    assert!(lines(&mut f, &night(&[], 1.0, 0)).is_empty());
}

/// Freeport gains only the admission vote, named after the person and the org.
#[test]
fn freeport_admission_vote_reads_in_its_own_voice() {
    let mut f = peopled("freeport");
    let proposed = Event::AdmissionProposed {
        proposal: ProposalId(1),
        org: OrgId(7),
        citizen: CitizenId(42),
        by: CitizenId(41),
    };
    assert_eq!(
        lines(&mut f, &proposed),
        ["marlow moves that The Readers admit ines. The members will vote."]
    );
    let voted = Event::AdmissionVoted {
        proposal: ProposalId(1),
        citizen: CitizenId(41),
        approve: true,
    };
    assert!(lines(&mut f, &voted).is_empty());
    let closed = Event::ProposalClosed {
        proposal: ProposalId(1),
        passed: true,
        tally: tally(1, 0, 1, 1),
    };
    assert_eq!(
        lines(&mut f, &closed),
        ["The members carry the admission of ines to The Readers, 1 to 0."]
    );
    // No Commune line leaks into Freeport.
    let unfilled = Event::OfficeUnfilled {
        office: OfficeKind::Coordinator,
        cycles: 5,
    };
    assert!(lines(&mut f, &unfilled).is_empty());
}

/// Every preset's copy loads, and a preset without copy is refused by name.
#[test]
fn copy_is_checked_by_name() {
    for preset in ["freeport", "commune"] {
        check_copy(presets(), preset).unwrap_or_else(|e| panic!("{e}"));
    }
    let err = check_copy(presets(), "nowhere").unwrap_err();
    assert!(
        err.contains("nowhere") && err.contains("chronicle.toml"),
        "{err}"
    );
    let tmp = std::env::temp_dir().join(format!("isms-copy-{}", std::process::id()));
    std::fs::create_dir_all(tmp.join("copy/half")).unwrap();
    std::fs::copy(
        presets().join("copy/commune/chronicle.toml"),
        tmp.join("copy/half/chronicle.toml"),
    )
    .unwrap();
    let err = check_copy(&tmp, "half").unwrap_err();
    assert!(err.contains("welcome.md"), "{err}");
    let _ = std::fs::remove_dir_all(tmp);
}

/// The copy's words for an enum value, and the value itself where it has none.
#[test]
fn words_fall_back_to_the_value() {
    let t = Templates::load(presets(), "commune").unwrap();
    assert_eq!(
        t.word("rationing", "need_first"),
        "the largest shortfall is served first"
    );
    assert_eq!(t.word("rationing", "other"), "other");
    let f = Templates::load(presets(), "freeport").unwrap();
    assert_eq!(f.word("rationing", "need_first"), "need_first");
}
