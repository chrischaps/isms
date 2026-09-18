use super::*;
use crate::config::load_preset;
use crate::event::CitizenDelta;
use crate::ledger::{Imbalance, conservation_check};
use std::path::Path;

fn preset(name: &str) -> crate::config::Preset {
    load_preset(Path::new(crate::WORKSPACE_PRESETS_DIR), name).unwrap()
}

fn fresh(name: &str) -> World {
    let p = preset(name);
    let mut w = World::new(0, 0, &p);
    apply(
        &mut w,
        &Event::SocietyCreated {
            society_id: 7,
            seed: 42,
            preset: Box::new(p),
        },
    );
    apply(&mut w, &Event::EpochStarted { epoch: 0 });
    w
}

fn joined(w: &mut World, n: u32) {
    for i in 0..n {
        let endowment = if w.constitution.has_money() {
            w.params.money.endowment
        } else {
            Money::ZERO
        };
        apply(
            w,
            &Event::CitizenJoined {
                citizen: CitizenId(i),
                handle: format!("c{i}"),
                kind: CitizenKind::Human,
                endowment,
                dwelling: None,
                explain: None,
            },
        );
    }
}

fn seed_food(w: &mut World, citizen: u32, qty: u32) {
    apply(
        w,
        &Event::Seeded {
            holder: Holder::Citizen(CitizenId(citizen)),
            asset: Asset::Good(Good::Food, qty),
        },
    );
}

#[test]
fn society_created_builds_the_world_from_the_preset() {
    let w = fresh("freeport");
    assert_eq!(w.meta.society_id, 7);
    assert_eq!(w.meta.seed, 42);
    assert_eq!(w.meta.preset, "freeport");
    assert_eq!(w.land.slots.len(), 14);
    assert!(w.store.is_none());
    conservation_check(&w).unwrap();
    let c = fresh("commune");
    assert!(c.store.is_some());
    assert!(c.state_stock.is_none());
    let d = fresh("directorate");
    assert!(d.state_stock.is_some());
}

#[test]
fn world_serde_round_trip_is_byte_identical() {
    let mut w = fresh("freeport");
    joined(&mut w, 3);
    let bytes = w.canonical_bytes();
    let back: World = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, w);
    assert_eq!(back.canonical_bytes(), bytes);
    assert_eq!(back.hash(), w.hash());
    let json = serde_json::to_string(&w).unwrap();
    let back_json: World = serde_json::from_str(&json).unwrap();
    assert_eq!(back_json, w);
}

#[test]
fn joins_mint_endowment_and_conserve() {
    let mut w = fresh("freeport");
    joined(&mut w, 5);
    assert_eq!(w.citizens.len(), 5);
    assert_eq!(w.ledger_meta.minted, Money::credits(5000));
    assert_eq!(
        w.citizens[&CitizenId(2)].household.balance,
        Money::credits(1000)
    );
    assert_eq!(w.next.citizen, CitizenId(5));
    conservation_check(&w).unwrap();
}

#[test]
fn transfers_move_money_and_goods_and_conserve() {
    let mut w = fresh("freeport");
    joined(&mut w, 2);
    seed_food(&mut w, 0, 10);
    conservation_check(&w).unwrap();
    apply(
        &mut w,
        &Event::Transferred {
            from: Party::Citizen(CitizenId(0)),
            to: Party::Citizen(CitizenId(1)),
            asset: Asset::Money(Money::credits(250)),
            memo: "loan".into(),
        },
    );
    apply(
        &mut w,
        &Event::Transferred {
            from: Party::Citizen(CitizenId(0)),
            to: Party::Citizen(CitizenId(1)),
            asset: Asset::Good(Good::Food, 4),
            memo: String::new(),
        },
    );
    assert_eq!(
        w.citizens[&CitizenId(0)].household.balance,
        Money::credits(750)
    );
    assert_eq!(
        w.citizens[&CitizenId(1)].household.balance,
        Money::credits(1250)
    );
    assert_eq!(goods_of(&w, Party::Citizen(CitizenId(0)), Good::Food), 6);
    assert_eq!(goods_of(&w, Party::Citizen(CitizenId(1)), Good::Food), 4);
    conservation_check(&w).unwrap();
}

#[test]
fn conservation_check_catches_a_leak() {
    let mut w = fresh("freeport");
    joined(&mut w, 1);
    w.citizens.get_mut(&CitizenId(0)).unwrap().household.balance += Money::cents(1);
    assert!(matches!(
        conservation_check(&w),
        Err(Imbalance::Money { .. })
    ));

    let mut w = fresh("freeport");
    joined(&mut w, 1);
    w.citizens
        .get_mut(&CitizenId(0))
        .unwrap()
        .household
        .pantry
        .insert(Good::Ore, 1);
    assert!(matches!(
        conservation_check(&w),
        Err(Imbalance::Good { .. })
    ));
}

#[test]
fn tick_resolved_consumes_from_pantry_and_advances_the_clock() {
    let mut w = fresh("freeport");
    joined(&mut w, 1);
    seed_food(&mut w, 0, 3);
    let mut needs = w.citizens[&CitizenId(0)].needs.clone();
    needs.food = 960;
    apply(
        &mut w,
        &Event::TickResolved {
            tick: 0,
            cycle: 0,
            price_index: None,
            vwap: Vec::new(),
            citizen_deltas: vec![CitizenDelta {
                citizen: CitizenId(0),
                needs: needs.clone(),
                food_eaten: 1,
                wares_consumed: 0,
                output_mult: 1.0,
                budget: 8,
                fatigue_debt: 0,
                consecutive_high_effort_cycles: 0,
                skill: BTreeMap::new(),
                cycle: crate::metrics::CitizenCycle::default(),
            }],
            workplace_deltas: vec![],
        },
    );
    assert_eq!(w.meta.tick, 1);
    assert_eq!(w.citizens[&CitizenId(0)].needs, needs);
    assert_eq!(goods_of(&w, Party::Citizen(CitizenId(0)), Good::Food), 2);
    conservation_check(&w).unwrap();
}

#[test]
#[allow(clippy::too_many_lines)]
fn unimplemented_registry_matches_the_enum() {
    for k in UNIMPLEMENTED {
        assert!(Event::ALL_KINDS.contains(k), "{k} is not an Event kind");
    }
    let implemented = [
        "SocietyCreated",
        "EpochStarted",
        "Seeded",
        "CitizenJoined",
        "HouseholderJoined",
        "PlanChanged",
        "LaborSet",
        "Transferred",
        "TickResolved",
        "CitizenSeen",
        "CycleClosed",
        "EpochEnded",
        "CitizenDormant",
        "CitizenReturned",
        "HardshipBegan",
        "HardshipEnded",
        "DestitutionBegan",
        "DestitutionEnded",
        "OrgFounded",
        "WorkplaceAdded",
        "Produced",
        "Assigned",
        "Unassigned",
        "ManagerAppointed",
        "MachinesInstalled",
        "MachinesUninstalled",
        "MachinesDepreciated",
        "SaleOffered",
        "SaleAccepted",
        "SaleCancelled",
        "WantedPosted",
        "WantedRemoved",
        "OrderPlaced",
        "OrderCancelled",
        "OrderExpired",
        "Trade",
        "EmploymentOffered",
        "EmploymentAccepted",
        "EmploymentTerminated",
        "Paid",
        "PaymentMissed",
        "SharesIssued",
        "SharesTransferred",
        "DividendDeclared",
        "DividendPaid",
        "DwellingBuilt",
        "DwellingTransferred",
        "DwellingOccupied",
        "LeaseOffered",
        "LeaseAccepted",
        "RentPaid",
        "RentMissed",
        "LeaseEnded",
        "CreditOffered",
        "CreditAccepted",
        "CreditInstallment",
        "CreditMissed",
        "CreditRepaid",
        "CreditDefaulted",
        "MembershipRequested",
        "MemberAdmitted",
        "MemberLeft",
        "HouseholderEmigrated",
        "Drew",
        "StoreDrawRequested",
        "StoreReturned",
        "PolicyChanged",
        "Pledged",
        "PledgeClosed",
        "NormsLedgerClosed",
        "StateStoreRequested",
        "StateStoreSold",
        "StateStoreShortage",
        "RationIssued",
        "PlanPublished",
        "TargetSet",
        "TransferRequested",
        "TransferDecided",
        "TaxAssessed",
        "NeedFloorPaid",
        "SurplusDeclared",
        "ShareOutPaid",
        "ShareRuleSet",
        "LevyPaid",
        "BankLoanRequested",
        "BankLoanDecided",
        "AdmissionProposed",
        "AdmissionVoted",
        "ProposalClosed",
        "UnionFormed",
        "DuesPaid",
        "CollectiveAgreementOffered",
        "CollectiveAgreementAccepted",
        "CollectiveAgreementEnded",
        "StrikeCalled",
        "StrikePaid",
        "StrikeEnded",
        "OfferWithdrawn",
    ];
    for k in Event::ALL_KINDS {
        let listed = UNIMPLEMENTED.contains(k);
        let done = implemented.contains(k);
        assert!(
            listed != done,
            "{k}: must be exactly one of implemented or UNIMPLEMENTED"
        );
    }
    assert_eq!(
        Event::ALL_KINDS.len(),
        UNIMPLEMENTED.len() + implemented.len()
    );
}

#[test]
fn all_kinds_is_in_sync_with_kind() {
    // Phase 0b variants are appended after `CycleClosed` (Q64).
    let last = Event::OfferWithdrawn {
        offer: crate::ids::OfferId(0),
        by: crate::ledger::Party::Citizen(crate::ids::CitizenId(0)),
        body: crate::world::OfferBody::Wanted {
            good: crate::kinds::Good::Food,
            qty: 1,
            max_price: crate::money::Money::ZERO,
        },
    };
    assert_eq!(last.kind(), *Event::ALL_KINDS.last().unwrap());
    let first = Event::SocietyCreated {
        society_id: 0,
        seed: 0,
        preset: Box::new(preset("freeport")),
    };
    assert_eq!(first.kind(), Event::ALL_KINDS[0]);
}
