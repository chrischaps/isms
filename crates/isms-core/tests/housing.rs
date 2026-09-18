#![allow(clippy::many_single_char_names, clippy::float_cmp)]
//! S0.11a done gate (TDD §18.2 S0.11, housing part): rent debits; a missed rent
//! survives the grace cycle and the eviction lands on the first tick of the
//! following cycle; a dormant tenant's rent is suspended; Shelter recovers when
//! housed; Builders build from Materials; dwelling sales; owner-occupancy.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::{ContractId, DwellingId, OfferId, OrgId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::needs::FULL;
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{LeaseAsset, Owner, Price, SaleAsset};

/// A landlord org (managed by the householder) with two dwellings; humans fed and quiet.
fn town(humans: u32) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000;
        })
        .humans(humans)
        .householders(1)
        .org(OrgKind::Firm, "Legacy Builders")
        .dwelling(0)
        .dwelling(0);
    for i in 0..humans as usize {
        b = b.pantry(i, Good::Food, 48);
    }
    let mut h = b.build();
    for who in h.citizen_ids() {
        let mut p = h.citizen(who).plan.clone();
        p.keep_food_at_least = 0;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(p),
        });
    }
    let mgr = nth(&h, humans as usize);
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(mgr),
    });
    h
}

fn lease_offer(rent_cents: i64, term: Option<u32>) -> Command {
    Command::OfferLease {
        asset: LeaseAsset::Dwelling(DwellingId(0)),
        rent_per_cycle: Money::cents(rent_cents),
        term_cycles: term,
    }
}

#[test]
fn a_lease_houses_the_tenant_and_rent_flows_at_cycle_end() {
    let mut h = town(2);
    let (t, mgr) = (nth(&h, 0), nth(&h, 2));
    assert!(h.citizen(t).household.dwelling.is_none());
    let r = h.cmd_dry(Envelope::citizen(t, lease_offer(800, None), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::NotOwner);
    h.cmd(Envelope::citizen(mgr, lease_offer(800, None), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    let r = h.cmd_dry(Envelope::citizen(mgr, lease_offer(800, None), 0).on_behalf_of(OrgId(0)));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::AlreadyExists,
        "already offered"
    );
    let ev = h
        .cmd(Envelope::citizen(
            t,
            Command::AcceptLease { offer: OfferId(0) },
            0,
        ))
        .unwrap();
    assert_eq!(
        ev.iter().map(Event::kind).collect::<Vec<_>>(),
        ["LeaseAccepted", "DwellingOccupied"]
    );
    assert_eq!(h.citizen(t).household.dwelling, Some(DwellingId(0)));
    assert_eq!(h.world.dwellings[&DwellingId(0)].occupant, Some(t));
    assert_eq!(h.world.dwellings[&DwellingId(0)].lease, Some(ContractId(0)));
    assert!(h.world.offers.is_empty());
    let r = h.cmd_dry(Envelope::citizen(
        nth(&h, 1),
        Command::AcceptLease { offer: OfferId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::UnknownOffer);
    h.check_every_step = false;
    let events = h.run_cycle();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::RentPaid { amount, .. } if *amount == Money::cents(800)))
    );
    assert_eq!(
        h.citizen(t).household.balance,
        Money::credits(1000) - Money::cents(800)
    );
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::cents(800));
    // housed: Shelter stays full, Comfort decays at the housed rate
    assert_eq!(h.citizen(t).needs.shelter, FULL);
    assert_eq!(h.citizen(t).needs.comfort, FULL - 24 * 10);
    assert_eq!(
        h.citizen(nth(&h, 1)).needs.shelter,
        FULL - 24 * 20,
        "unhoused neighbour"
    );
    h.check();
}

#[test]
fn missed_rent_survives_the_grace_cycle_then_eviction_on_the_first_tick() {
    let mut h = town(1);
    let (t, mgr) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(mgr, lease_offer(800, None), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    h.cmd(Envelope::citizen(
        t,
        Command::AcceptLease { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    // give away all money so rent cannot be paid
    h.cmd(Envelope::citizen(
        t,
        Command::Transfer {
            to: Party::Citizen(mgr),
            asset: Asset::Money(Money::credits(1000)),
            memo: String::new(),
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    let c0 = h.run_cycle();
    assert!(c0.iter().any(|e| matches!(e, Event::RentMissed { .. })));
    assert!(!c0.iter().any(|e| matches!(e, Event::RentPaid { .. })));
    let c1 = h.run_cycle();
    assert!(
        c1.iter().any(|e| matches!(e, Event::RentMissed { .. })),
        "second miss at the end of the grace cycle"
    );
    assert!(
        !c1.iter().any(|e| matches!(e, Event::LeaseEnded { .. })),
        "not yet evicted at cycle end"
    );
    assert_eq!(h.citizen(t).household.dwelling, Some(DwellingId(0)));
    let first_tick = h.tick();
    assert!(
        first_tick
            .iter()
            .any(|e| matches!(e, Event::LeaseEnded { evicted: true, .. }))
    );
    assert!(h.citizen(t).household.dwelling.is_none());
    assert_eq!(h.world.dwellings[&DwellingId(0)].occupant, None);
    assert_eq!(h.world.dwellings[&DwellingId(0)].lease, None);
    // a miss followed by a payment resets the count
    let mut h = town(1);
    let (t, mgr) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(mgr, lease_offer(800, None), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    h.cmd(Envelope::citizen(
        t,
        Command::AcceptLease { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        t,
        Command::Transfer {
            to: Party::Citizen(mgr),
            asset: Asset::Money(Money::credits(1000)),
            memo: String::new(),
        },
        0,
    ))
    .unwrap();
    h.check_every_step = false;
    h.run_cycle();
    h.apply(Event::Seeded {
        holder: Holder::Citizen(t),
        asset: Asset::Money(Money::credits(10)),
    });
    let c1 = h.run_cycle();
    assert!(c1.iter().any(|e| matches!(e, Event::RentPaid { .. })));
    h.tick();
    assert_eq!(
        h.citizen(t).household.dwelling,
        Some(DwellingId(0)),
        "paid in the grace cycle: no eviction"
    );
    h.check();
}

#[test]
fn a_dormant_tenant_pays_no_rent_and_a_term_lease_ends() {
    let mut h = town(1);
    let (t, mgr) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(mgr, lease_offer(800, Some(2)), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    h.cmd(Envelope::citizen(
        t,
        Command::AcceptLease { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.apply(Event::CitizenDormant { citizen: t });
    h.check_every_step = false;
    let c0 = h.run_cycle();
    assert!(
        !c0.iter()
            .any(|e| matches!(e, Event::RentPaid { .. } | Event::RentMissed { .. }))
    );
    h.apply(Event::CitizenReturned { citizen: t });
    let c1 = h.run_cycle();
    assert!(c1.iter().any(|e| matches!(e, Event::RentPaid { .. })));
    assert!(
        c1.iter()
            .any(|e| matches!(e, Event::LeaseEnded { evicted: false, .. })),
        "term of 2 cycles"
    );
    assert!(h.citizen(t).household.dwelling.is_none());
    h.check();
}

#[test]
fn end_lease_by_either_party_and_self_rent_refused() {
    let mut h = town(1);
    let (t, mgr) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(mgr, lease_offer(0, None), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    h.cmd(Envelope::citizen(
        t,
        Command::AcceptLease { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        mgr,
        Command::EndLease {
            contract: ContractId(0),
        },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::NotParty,
        "the manager as a citizen is not the owner"
    );
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::EndLease {
                contract: ContractId(0),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert!(h.citizen(t).household.dwelling.is_none());
    h.check();
}

#[test]
fn builders_build_dwellings_from_materials() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| {
            p.params.population.collapse_enabled = false;
            p.params.population.dormancy_absent_cycles = 1000;
        })
        .humans(1)
        .pantry(0, Good::Food, 48)
        .org(OrgKind::Firm, "Builders")
        .workplace(WorkplaceKind::Builder, 0, 0)
        .org_inventory(0, Good::Materials, 25)
        .assign(0, 0, 8, Effort::Normal)
        .build();
    h.check_every_step = false;
    // 0.5 x 8/24 per tick, unhoused x0.7 after tick 0: about 0.13 per tick -> 2 dwellings need ~16 ticks
    let events = h.run_cycle();
    let built = events
        .iter()
        .filter(|e| matches!(e, Event::DwellingBuilt { .. }))
        .count();
    assert_eq!(built, 2, "25 Materials fund two dwellings");
    assert_eq!(h.world.dwellings.len(), 2);
    assert_eq!(
        h.world.dwellings[&DwellingId(0)].owner,
        Owner::Org(OrgId(0))
    );
    assert_eq!(h.world.orgs[&OrgId(0)].inventory[&Good::Materials], 5);
    assert_eq!(h.world.ledger_meta.consumed[&Good::Materials], 20);
    assert_eq!(h.world.ledger_meta.dwellings_built, 2);
    assert_eq!(
        h.world.workplaces[&WorkplaceId(0)].output_remainder,
        0.0,
        "starved of Materials: no carry"
    );
    h.check();
}

#[test]
fn a_dwelling_can_be_sold_and_lived_in_by_its_owner() {
    let mut h = town(2);
    let (buyer, other, mgr) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    let r = h.cmd_dry(Envelope::citizen(
        buyer,
        Command::OfferSale {
            asset: SaleAsset::Dwelling(DwellingId(0)),
            price: Price::Money(Money::credits(1)),
            to: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotOwner);
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::OfferSale {
                asset: SaleAsset::Dwelling(DwellingId(0)),
                price: Price::Money(Money::credits(300)),
                to: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(mgr, lease_offer(1, None), 0).on_behalf_of(OrgId(0)));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::AlreadyExists,
        "under a sale offer"
    );
    h.cmd(Envelope::citizen(
        buyer,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    assert_eq!(
        h.world.dwellings[&DwellingId(0)].owner,
        Owner::Citizen(buyer)
    );
    assert_eq!(h.citizen(buyer).household.balance, Money::credits(700));
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::credits(300));
    let r = h.cmd_dry(Envelope::citizen(
        other,
        Command::MoveIn {
            dwelling: DwellingId(0),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotOwner);
    h.cmd(Envelope::citizen(
        buyer,
        Command::MoveIn {
            dwelling: DwellingId(0),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.citizen(buyer).household.dwelling, Some(DwellingId(0)));
    h.tick();
    assert_eq!(h.citizen(buyer).needs.shelter, FULL);
    h.cmd(Envelope::citizen(
        buyer,
        Command::MoveOut {
            dwelling: DwellingId(0),
        },
        1,
    ))
    .unwrap();
    assert!(h.citizen(buyer).household.dwelling.is_none());
    // and now they can let it
    h.cmd(Envelope::citizen(buyer, lease_offer(500, None), 1))
        .unwrap();
    h.cmd(Envelope::citizen(
        other,
        Command::AcceptLease { offer: OfferId(1) },
        1,
    ))
    .unwrap();
    assert_eq!(h.citizen(other).household.dwelling, Some(DwellingId(0)));
    h.check();
}

/// Q109 (D9): withdrawing a lease offer frees the dwelling for another offer.
#[test]
fn a_withdrawn_lease_offer_frees_the_dwelling() {
    let mut h = town(1);
    let mgr = nth(&h, 1);
    h.cmd(Envelope::citizen(mgr, lease_offer(800, None), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    let r = h.cmd_dry(Envelope::citizen(mgr, lease_offer(900, None), 0).on_behalf_of(OrgId(0)));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::AlreadyExists,
        "under offer"
    );
    let r = h.cmd_dry(Envelope::citizen(
        nth(&h, 0),
        Command::WithdrawOffer { offer: OfferId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotParty);
    let ev = h
        .cmd(
            Envelope::citizen(mgr, Command::WithdrawOffer { offer: OfferId(0) }, 0)
                .on_behalf_of(OrgId(0)),
        )
        .unwrap();
    assert_eq!(
        ev.iter().map(Event::kind).collect::<Vec<_>>(),
        ["OfferWithdrawn"]
    );
    assert!(h.world.offers.is_empty());
    h.cmd(Envelope::citizen(mgr, lease_offer(900, None), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    h.check();
}
