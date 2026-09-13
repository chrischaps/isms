//! Dwellings and leases (GDD §4.1, §6.1 housing, §7.2 lease row; TDD §5.3,
//! §5.5 phase 7 and step 8d, S0.11). Dwellings are assets with identity, built
//! by Builders, owned by orgs or citizens, occupied by one citizen at a time.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{ContractId, DwellingId, OfferId};
use crate::ledger::Party;
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::transfers::acting_party;
use crate::world::{ContractBody, ContractStatus, Dwelling, LeaseAsset, OfferBody, Owner, World};

fn dwelling(world: &World, id: DwellingId) -> Result<&Dwelling, Reject> {
    world
        .dwellings
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownDwelling, format!("no dwelling {id}")))
}

/// The owner a party acts as.
#[must_use]
pub fn owner_of(party: Party) -> Owner {
    match party {
        Party::Citizen(c) => Owner::Citizen(c),
        Party::Org(o) => Owner::Org(o),
    }
}

/// Whether the dwelling is under an open sale or lease offer.
#[allow(clippy::match_same_arms)]
fn under_offer(world: &World, id: DwellingId) -> bool {
    world.offers.values().any(|o| match &o.body {
        OfferBody::Sale {
            asset: crate::world::SaleAsset::Dwelling(d),
            ..
        } => *d == id,
        OfferBody::Lease {
            asset: LeaseAsset::Dwelling(d),
            ..
        } => *d == id,
        _ => false,
    })
}

/// Checks for `OfferSale { Dwelling }`: the seller owns it and it is not already offered.
pub fn check_dwelling_sale(world: &World, seller: Party, id: DwellingId) -> Result<(), Reject> {
    let d = dwelling(world, id)?;
    if d.owner != owner_of(seller) {
        return Err(Reject::new(
            RejectCode::NotOwner,
            format!("{seller:?} does not own {id}"),
        ));
    }
    if under_offer(world, id) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{id} is already offered"),
        ));
    }
    Ok(())
}

/// `OfferLease`: the owner lists an unoccupied dwelling at a rent per cycle.
pub fn offer_lease(
    world: &World,
    envelope: &Envelope<Command>,
    asset: LeaseAsset,
    rent_per_cycle: Money,
    term_cycles: Option<u32>,
) -> Result<Vec<Event>, Reject> {
    let party = acting_party(world, envelope)?;
    let LeaseAsset::Dwelling(id) = asset else {
        return Err(Reject::new(
            RejectCode::NotImplemented,
            "workplace leases are not in Phase 0 (Q31)",
        ));
    };
    let d = dwelling(world, id)?;
    if d.owner != owner_of(party) {
        return Err(Reject::new(
            RejectCode::NotOwner,
            format!("{party:?} does not own {id}"),
        ));
    }
    if d.occupant.is_some() {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{id} is occupied"),
        ));
    }
    if under_offer(world, id) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{id} is already offered"),
        ));
    }
    if rent_per_cycle < Money::ZERO || term_cycles == Some(0) {
        return Err(Reject::new(
            RejectCode::InvalidPrice,
            "rent must be non-negative and the term positive",
        ));
    }
    if rent_per_cycle > Money::ZERO && !world.constitution.has_money() {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            "rent needs money",
        ));
    }
    Ok(vec![Event::LeaseOffered {
        offer: world.next.offer,
        body: OfferBody::Lease {
            asset,
            rent_per_cycle,
            term_cycles,
        },
    }])
}

/// `AcceptLease`: an unhoused citizen takes the dwelling.
pub fn accept_lease(
    world: &World,
    envelope: &Envelope<Command>,
    id: OfferId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    if envelope.on_behalf_of.is_some() {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "orgs do not rent dwellings",
        ));
    }
    let offer = world
        .offers
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOffer, format!("no offer {id}")))?;
    let OfferBody::Lease {
        asset,
        rent_per_cycle,
        term_cycles,
    } = offer.body
    else {
        return Err(Reject::new(
            RejectCode::UnknownOffer,
            format!("offer {id} is not a lease"),
        ));
    };
    let LeaseAsset::Dwelling(did) = asset else {
        return Err(Reject::new(
            RejectCode::NotImplemented,
            "workplace leases are not in Phase 0",
        ));
    };
    if citizen.household.dwelling.is_some() {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{} is already housed", citizen.id),
        ));
    }
    if offer.by == Party::Citizen(citizen.id) {
        return Err(Reject::new(
            RejectCode::SelfDeal,
            "cannot rent from yourself",
        ));
    }
    let long = term_cycles.is_none_or(|t| t > world.params.contracts.long_contract_cycles);
    if citizen.flags.options_narrowed && long {
        return Err(Reject::new(
            RejectCode::OptionsNarrowed,
            "a destitute citizen cannot sign a long lease",
        ));
    }
    let contract = world.next.contract;
    Ok(vec![
        Event::LeaseAccepted {
            contract,
            owner: offer.by,
            tenant: Party::Citizen(citizen.id),
            asset,
            rent_per_cycle,
            term_cycles,
        },
        Event::DwellingOccupied {
            dwelling: did,
            citizen: Some(citizen.id),
        },
    ])
}

fn lease(
    world: &World,
    id: ContractId,
) -> Result<(&crate::world::Contract, DwellingId, Money), Reject> {
    let k = world
        .contracts
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownContract, format!("no contract {id}")))?;
    match k.body {
        ContractBody::Lease {
            asset: LeaseAsset::Dwelling(d),
            rent_per_cycle,
            ..
        } if k.status != ContractStatus::Ended => Ok((k, d, rent_per_cycle)),
        _ => Err(Reject::new(
            RejectCode::UnknownContract,
            format!("contract {id} is not an open lease"),
        )),
    }
}

/// `EndLease`: either party ends the lease at once (Q32).
pub fn end_lease(
    world: &World,
    envelope: &Envelope<Command>,
    id: ContractId,
) -> Result<Vec<Event>, Reject> {
    let party = acting_party(world, envelope)?;
    let (k, d, _) = lease(world, id)?;
    if k.parties.0 != party && k.parties.1 != party {
        return Err(Reject::new(
            RejectCode::NotParty,
            "only the owner or the tenant ends a lease",
        ));
    }
    Ok(vec![
        Event::LeaseEnded {
            contract: id,
            evicted: false,
        },
        Event::DwellingOccupied {
            dwelling: d,
            citizen: None,
        },
    ])
}

/// `MoveIn`: a citizen occupies a dwelling they own (Q34).
pub fn move_in(
    world: &World,
    envelope: &Envelope<Command>,
    id: DwellingId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let d = dwelling(world, id)?;
    if d.owner != Owner::Citizen(citizen.id) {
        return Err(Reject::new(
            RejectCode::NotOwner,
            format!("{} does not own {id}", citizen.id),
        ));
    }
    if d.occupant.is_some() || citizen.household.dwelling.is_some() {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            "occupied, or already housed",
        ));
    }
    Ok(vec![Event::DwellingOccupied {
        dwelling: id,
        citizen: Some(citizen.id),
    }])
}

/// `MoveOut`: an owner-occupier leaves their own dwelling.
pub fn move_out(
    world: &World,
    envelope: &Envelope<Command>,
    id: DwellingId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let d = dwelling(world, id)?;
    if d.occupant != Some(citizen.id) || d.lease.is_some() {
        return Err(Reject::new(
            RejectCode::NotParty,
            "not the owner-occupant (tenants end their lease)",
        ));
    }
    Ok(vec![Event::DwellingOccupied {
        dwelling: id,
        citizen: None,
    }])
}

/// Step 8d: rent. A tenant who cannot pay the full rent pays nothing and the
/// miss is recorded (Q33); leases at the end of their term end after rent.
#[allow(clippy::type_complexity)]
pub fn cycle_end_8d_rent(b: &mut TickBuilder) {
    let cycle = b.cycle;
    let tpc = b.world.params.time.ticks_per_cycle;
    let leases: Vec<(
        ContractId,
        Party,
        Party,
        DwellingId,
        Money,
        Option<u32>,
        crate::ids::Tick,
    )> = b
        .world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active)
        .filter_map(|k| match k.body {
            ContractBody::Lease {
                asset: LeaseAsset::Dwelling(d),
                rent_per_cycle,
                ..
            } => Some((
                k.id,
                k.parties.0,
                k.parties.1,
                d,
                rent_per_cycle,
                k.term_cycles,
                k.created_tick,
            )),
            _ => None,
        })
        .collect();
    for (id, owner, tenant, d, rent, term, created) in leases {
        if rent > Money::ZERO {
            let have = crate::apply::money_of(&b.world, tenant);
            if have >= rent {
                let explain = Explain::new(RuleId::Rent, "rent_per_cycle", rent)
                    .input("rent_per_cycle", rent);
                b.emit(Event::RentPaid {
                    contract: id,
                    amount: rent,
                    explain,
                });
            } else {
                b.emit(Event::RentMissed {
                    contract: id,
                    owed: rent,
                });
            }
        }
        let _ = owner;
        if term.is_some_and(|t| cycle + 1 >= created / tpc + t) {
            b.emit(Event::LeaseEnded {
                contract: id,
                evicted: false,
            });
            b.emit(Event::DwellingOccupied {
                dwelling: d,
                citizen: None,
            });
        }
    }
}

/// Phase 7 (first tick of a cycle): evict tenants whose misses exceed the grace.
pub fn phase_7_evictions(b: &mut TickBuilder) {
    if !b.tick.is_multiple_of(b.world.params.time.ticks_per_cycle) {
        return;
    }
    let grace = b.world.params.contracts.lease_grace_cycles;
    let evict: Vec<(ContractId, DwellingId)> = b
        .world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active)
        .filter_map(|k| match k.body {
            ContractBody::Lease {
                asset: LeaseAsset::Dwelling(d),
                missed_cycles,
                ..
            } if missed_cycles > grace => Some((k.id, d)),
            _ => None,
        })
        .collect();
    for (id, d) in evict {
        b.emit(Event::LeaseEnded {
            contract: id,
            evicted: true,
        });
        b.emit(Event::DwellingOccupied {
            dwelling: d,
            citizen: None,
        });
    }
}
