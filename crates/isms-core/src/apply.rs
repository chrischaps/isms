//! `apply`: fold one event into the `World`. Total: never fails, never panics on
//! events that `handle`/`tick` produced. It trusts its input; validation lives
//! upstream. Variants not yet implemented are explicit no-ops listed in
//! [`UNIMPLEMENTED`], which a test keeps honest and which later cards shrink.

use crate::event::{CitizenDelta, Event, WorkplaceDelta};
use crate::ids::CitizenId;
use crate::kinds::{CitizenKind, Good};
use crate::ledger::{Asset, Holder, LedgerMeta, Party};
use crate::money::Money;
use crate::world::{
    Citizen, CitizenFlags, Household, LaborPlan, LaborState, Needs, StandingPlan, VoteDefault,
    World,
};
use std::collections::BTreeMap;

/// Event kinds whose `apply` is still a no-op. Each later card removes its own.
pub const UNIMPLEMENTED: &[&str] = &["HouseholderEmigrated", "Drew", "PolicyChanged"];

/// Fold one event into the world.
#[allow(clippy::too_many_lines, clippy::match_same_arms)] // a flat dispatcher
pub fn apply(world: &mut World, event: &Event) {
    match event {
        Event::SocietyCreated {
            society_id,
            seed,
            preset,
        } => {
            *world = World::new(*society_id, *seed, preset);
        }
        Event::EpochStarted { epoch } => {
            world.meta.epoch = *epoch;
            world.meta.tick = 0;
            world.meta.epoch_ended = None;
        }
        Event::Seeded { holder, asset } => {
            credit(world, *holder, *asset);
            match asset {
                Asset::Money(m) => world.ledger_meta.minted += *m,
                Asset::Good(g, q) => LedgerMeta::add(&mut world.ledger_meta.seeded, *g, *q),
            }
        }
        Event::CitizenJoined {
            citizen,
            handle,
            kind,
            endowment,
            dwelling,
            ..
        } => {
            join(world, *citizen, handle, *kind, *endowment, *dwelling);
        }
        Event::HouseholderJoined {
            citizen,
            handle,
            endowment,
            dwelling,
            ..
        } => {
            join(
                world,
                *citizen,
                handle,
                CitizenKind::Householder,
                *endowment,
                *dwelling,
            );
        }
        Event::PlanChanged { citizen, plan } => {
            if let Some(c) = world.citizens.get_mut(citizen) {
                c.plan = (**plan).clone();
            }
        }
        Event::LaborSet {
            citizen,
            allocations,
        } => set_labor(world, *citizen, allocations),
        Event::Transferred {
            from, to, asset, ..
        } => {
            debit(world, Holder::from(*from), *asset);
            credit(world, Holder::from(*to), *asset);
        }
        Event::CitizenDormant { citizen } => {
            if let Some(c) = world.citizens.get_mut(citizen) {
                c.dormant = true;
            }
            set_contract_status(
                world,
                *citizen,
                crate::world::ContractStatus::Active,
                crate::world::ContractStatus::Suspended,
            );
        }
        Event::CitizenReturned { citizen } => {
            if let Some(c) = world.citizens.get_mut(citizen) {
                c.dormant = false;
            }
            set_contract_status(
                world,
                *citizen,
                crate::world::ContractStatus::Suspended,
                crate::world::ContractStatus::Active,
            );
        }
        Event::HardshipBegan { citizen, .. } => set_flag(world, *citizen, |f| {
            f.in_hardship = true;
        }),
        Event::HardshipEnded { citizen, .. } => set_flag(world, *citizen, |f| {
            f.in_hardship = false;
        }),
        Event::DestitutionBegan { citizen, .. } => set_flag(world, *citizen, |f| {
            f.destitute = true;
            f.options_narrowed = true;
        }),
        Event::DestitutionEnded { citizen, .. } => set_flag(world, *citizen, |f| {
            f.destitute = false;
            f.options_narrowed = false;
        }),
        Event::OrgFounded {
            org,
            kind,
            name,
            founder,
            ownership,
            manager,
            fee_burned,
        } => apply_org_founded(
            world,
            *org,
            *kind,
            name,
            *founder,
            ownership,
            *manager,
            *fee_burned,
        ),
        Event::WorkplaceAdded {
            workplace,
            org,
            kind,
            slot,
            materials_consumed,
        } => apply_workplace_added(world, *workplace, *org, *kind, *slot, *materials_consumed),
        Event::Produced {
            workplace,
            output,
            units,
            inputs_consumed,
            ..
        } => apply_produced(world, *workplace, *output, *units, inputs_consumed),
        Event::Assigned {
            workplace,
            citizen,
            contract,
        } => {
            if let Some(w) = world.workplaces.get_mut(workplace) {
                w.workers
                    .entry(*citizen)
                    .or_insert_with(|| crate::world::Assignment::new(*contract));
            }
        }
        Event::Unassigned { workplace, citizen } => {
            if let Some(w) = world.workplaces.get_mut(workplace) {
                w.workers.remove(citizen);
            }
            if let Some(c) = world.citizens.get_mut(citizen) {
                c.labor.allocations.retain(|a| a.workplace != *workplace);
            }
        }
        Event::ManagerAppointed { org, citizen } => {
            if let Some(o) = world.orgs.get_mut(org) {
                o.manager = *citizen;
            }
        }
        Event::MachinesInstalled { workplace, qty } => {
            if let Some(org) = world.workplaces.get(workplace).map(|w| w.org) {
                debit(world, Holder::Org(org), Asset::Good(Good::Machines, *qty));
                credit(
                    world,
                    Holder::Workplace(*workplace),
                    Asset::Good(Good::Machines, *qty),
                );
            }
        }
        Event::MachinesUninstalled { workplace, qty } => {
            if let Some(org) = world.workplaces.get(workplace).map(|w| w.org) {
                debit(
                    world,
                    Holder::Workplace(*workplace),
                    Asset::Good(Good::Machines, *qty),
                );
                credit(world, Holder::Org(org), Asset::Good(Good::Machines, *qty));
            }
        }
        Event::MachinesDepreciated { workplace, qty, .. } => {
            debit(
                world,
                Holder::Workplace(*workplace),
                Asset::Good(Good::Machines, *qty),
            );
            LedgerMeta::add(&mut world.ledger_meta.depreciated, Good::Machines, *qty);
        }
        Event::SaleOffered {
            offer,
            by,
            asset,
            price,
            to,
        } => {
            match asset {
                crate::world::SaleAsset::Good(g, q) => {
                    let a = Asset::Good(*g, *q);
                    debit(world, Holder::from(*by), a);
                    world
                        .escrow
                        .insert(crate::world::EscrowKey::Offer(*offer), a);
                }
                crate::world::SaleAsset::Shares(org, q) => {
                    if let Some(h) = crate::shares::holder_of(*by, *org) {
                        move_shares(world, *org, Some(h), None, *q);
                    }
                    world
                        .share_escrow
                        .insert(crate::world::EscrowKey::Offer(*offer), (*org, *q));
                }
                crate::world::SaleAsset::Dwelling(_) => {}
            }
            world.offers.insert(
                *offer,
                crate::world::Offer {
                    id: *offer,
                    by: *by,
                    created_tick: world.meta.tick,
                    body: crate::world::OfferBody::Sale {
                        asset: *asset,
                        price: *price,
                        to: *to,
                    },
                },
            );
            bump_offer(world, *offer);
        }
        Event::SaleAccepted {
            offer,
            buyer,
            seller,
            asset,
            price,
        } => {
            if let Some(a) = world.escrow.remove(&crate::world::EscrowKey::Offer(*offer)) {
                credit(world, Holder::from(*buyer), a);
            }
            if let Some((org, q)) = world
                .share_escrow
                .remove(&crate::world::EscrowKey::Offer(*offer))
                && let Some(h) = crate::shares::holder_of(*buyer, org)
            {
                move_shares(world, org, None, Some(h), q);
            }
            if let crate::world::SaleAsset::Dwelling(d) = asset
                && let Some(dw) = world.dwellings.get_mut(d)
            {
                dw.owner = crate::housing::owner_of(*buyer);
            }
            let paid = match price {
                crate::world::Price::Money(m) => Asset::Money(*m),
                crate::world::Price::Good(g, q) => Asset::Good(*g, *q),
            };
            debit(world, Holder::from(*buyer), paid);
            credit(world, Holder::from(*seller), paid);
            world.offers.remove(offer);
        }
        Event::SaleCancelled { offer } => {
            let by = world.offers.get(offer).map(|o| o.by);
            if let (Some(a), Some(by)) = (
                world.escrow.remove(&crate::world::EscrowKey::Offer(*offer)),
                by,
            ) {
                credit(world, Holder::from(by), a);
            }
            if let (Some((org, q)), Some(by)) = (
                world
                    .share_escrow
                    .remove(&crate::world::EscrowKey::Offer(*offer)),
                by,
            ) && let Some(h) = crate::shares::holder_of(by, org)
            {
                move_shares(world, org, None, Some(h), q);
            }
            world.offers.remove(offer);
        }
        Event::WantedPosted {
            offer,
            by,
            good,
            qty,
            max_price,
        } => {
            world.offers.insert(
                *offer,
                crate::world::Offer {
                    id: *offer,
                    by: *by,
                    created_tick: world.meta.tick,
                    body: crate::world::OfferBody::Wanted {
                        good: *good,
                        qty: *qty,
                        max_price: *max_price,
                    },
                },
            );
            bump_offer(world, *offer);
        }
        Event::WantedRemoved { offer } => {
            world.offers.remove(offer);
        }
        Event::OrderPlaced { order, escrow } => {
            if let (crate::world::Side::Ask, crate::world::Instrument::Share(org)) =
                (order.side, order.instrument)
            {
                if let Some(h) = crate::shares::holder_of(order.owner, org) {
                    move_shares(world, org, Some(h), None, u64::from(order.qty));
                }
                world.share_escrow.insert(
                    crate::world::EscrowKey::Order(order.id),
                    (org, u64::from(order.qty)),
                );
            } else {
                debit(world, Holder::from(order.owner), *escrow);
            }
            world
                .escrow
                .insert(crate::world::EscrowKey::Order(order.id), *escrow);
            world
                .books
                .entry(order.instrument)
                .or_default()
                .orders
                .insert(order.id, *order);
            if world.next.order.0 <= order.id.0 {
                world.next.order = order.id.next();
            }
        }
        Event::OrderCancelled { order, released } | Event::OrderExpired { order, released } => {
            let key = crate::world::EscrowKey::Order(*order);
            let owner = world
                .books
                .values()
                .find_map(|b| b.orders.get(order).map(|o| o.owner));
            if let (Some(owner), Some(_)) = (owner, world.escrow.remove(&key)) {
                credit(world, Holder::from(owner), *released);
                if let Some((org, qty)) = world.share_escrow.remove(&key)
                    && let Some(h) = crate::shares::holder_of(owner, org)
                {
                    move_shares(world, org, None, Some(h), qty);
                }
            }
            for b in world.books.values_mut() {
                b.orders.remove(order);
            }
        }
        Event::Trade {
            instrument,
            buyer,
            seller,
            buy_order,
            sell_order,
            qty,
            price,
            tick,
        } => apply_trade(
            world,
            *instrument,
            *buyer,
            *seller,
            *buy_order,
            *sell_order,
            *qty,
            *price,
            *tick,
        ),
        Event::EmploymentOffered { offer, body } => {
            let by = match body {
                crate::world::OfferBody::Employment { org, .. } => Party::Org(*org),
                _ => return,
            };
            world.offers.insert(
                *offer,
                crate::world::Offer {
                    id: *offer,
                    by,
                    created_tick: world.meta.tick,
                    body: body.clone(),
                },
            );
            bump_offer(world, *offer);
        }
        Event::EmploymentAccepted {
            contract,
            offer,
            org,
            workplace,
            citizen,
            pay,
            max_hours,
            term_cycles,
            notice_cycles,
        } => {
            world.contracts.insert(
                *contract,
                crate::world::Contract {
                    id: *contract,
                    parties: (Party::Org(*org), Party::Citizen(*citizen)),
                    created_tick: world.meta.tick,
                    term_cycles: *term_cycles,
                    status: crate::world::ContractStatus::Active,
                    body: crate::world::ContractBody::Employment {
                        org: *org,
                        workplace: *workplace,
                        pay: *pay,
                        max_hours: *max_hours,
                        notice_cycles: *notice_cycles,
                    },
                },
            );
            if let Some(o) = world.orgs.get_mut(org) {
                o.employees.insert(*contract);
            }
            let mut exhausted = false;
            if let Some(off) = world.offers.get_mut(offer)
                && let crate::world::OfferBody::Employment { places, .. } = &mut off.body
            {
                *places = places.saturating_sub(1);
                exhausted = *places == 0;
            }
            if exhausted {
                world.offers.remove(offer);
            }
            if world.next.contract.0 <= contract.0 {
                world.next.contract = contract.next();
            }
        }
        Event::EmploymentTerminated { contract, .. } => {
            if let Some(k) = world.contracts.get_mut(contract) {
                k.status = crate::world::ContractStatus::Ended;
                if let Party::Org(org) = k.parties.0
                    && let Some(o) = world.orgs.get_mut(&org)
                {
                    o.employees.remove(contract);
                }
            }
        }
        Event::Paid {
            citizen,
            org,
            amount,
            ..
        } => {
            debit(world, Holder::Org(*org), Asset::Money(*amount));
            credit(world, Holder::Citizen(*citizen), Asset::Money(*amount));
        }
        Event::PaymentMissed { org, contract, .. } => {
            if let Some(o) = world.orgs.get_mut(org) {
                o.payment_missed = true;
                o.employees.remove(contract);
            }
            if let Some(k) = world.contracts.get_mut(contract) {
                k.status = crate::world::ContractStatus::Ended;
            }
        }
        Event::SharesIssued { org, qty } => {
            if let Some(o) = world.orgs.get_mut(org)
                && let crate::world::Ownership::Shares { issued, holdings } = &mut o.ownership
            {
                *issued += qty;
                *holdings
                    .entry(crate::world::ShareHolder::OrgSelf)
                    .or_insert(0) += qty;
            }
        }
        Event::SharesTransferred { org, from, to, qty } => {
            move_shares(world, *org, Some(*from), Some(*to), *qty);
        }
        Event::DividendDeclared { org, per_share, .. } => {
            if let Some(o) = world.orgs.get_mut(org) {
                o.declared_dividend = Some(*per_share);
            }
        }
        Event::DividendPaid {
            org,
            citizen,
            amount,
            ..
        } => {
            debit(world, Holder::Org(*org), Asset::Money(*amount));
            credit(world, Holder::Citizen(*citizen), Asset::Money(*amount));
        }
        Event::DwellingBuilt {
            dwelling,
            org,
            materials_consumed,
            ..
        } => {
            debit(
                world,
                Holder::Org(*org),
                Asset::Good(Good::Materials, *materials_consumed),
            );
            LedgerMeta::add(
                &mut world.ledger_meta.consumed,
                Good::Materials,
                *materials_consumed,
            );
            world.ledger_meta.dwellings_built += 1;
            world.dwellings.insert(
                *dwelling,
                crate::world::Dwelling {
                    id: *dwelling,
                    owner: crate::world::Owner::Org(*org),
                    occupant: None,
                    lease: None,
                    built_tick: world.meta.tick,
                },
            );
            if world.next.dwelling.0 <= dwelling.0 {
                world.next.dwelling = dwelling.next();
            }
        }
        Event::DwellingTransferred { dwelling, to } => {
            if let Some(d) = world.dwellings.get_mut(dwelling) {
                d.owner = *to;
            }
        }
        Event::DwellingOccupied { dwelling, citizen } => {
            let previous = world.dwellings.get(dwelling).and_then(|d| d.occupant);
            if let Some(p) = previous
                && let Some(c) = world.citizens.get_mut(&p)
            {
                c.household.dwelling = None;
            }
            if let Some(d) = world.dwellings.get_mut(dwelling) {
                d.occupant = *citizen;
                if citizen.is_none() {
                    d.lease = None;
                }
            }
            if let Some(c) = citizen.and_then(|c| world.citizens.get_mut(&c)) {
                c.household.dwelling = Some(*dwelling);
            }
        }
        Event::LeaseOffered { offer, body } => {
            let by = match body {
                crate::world::OfferBody::Lease {
                    asset: crate::world::LeaseAsset::Dwelling(d),
                    ..
                } => match world.dwellings.get(d).map(|d| d.owner) {
                    Some(crate::world::Owner::Citizen(c)) => Party::Citizen(c),
                    Some(crate::world::Owner::Org(o)) => Party::Org(o),
                    _ => return,
                },
                _ => return,
            };
            world.offers.insert(
                *offer,
                crate::world::Offer {
                    id: *offer,
                    by,
                    created_tick: world.meta.tick,
                    body: body.clone(),
                },
            );
            bump_offer(world, *offer);
        }
        Event::LeaseAccepted {
            contract,
            owner,
            tenant,
            asset,
            rent_per_cycle,
            term_cycles,
        } => {
            world.contracts.insert(
                *contract,
                crate::world::Contract {
                    id: *contract,
                    parties: (*owner, *tenant),
                    created_tick: world.meta.tick,
                    term_cycles: *term_cycles,
                    status: crate::world::ContractStatus::Active,
                    body: crate::world::ContractBody::Lease {
                        asset: *asset,
                        rent_per_cycle: *rent_per_cycle,
                        missed_cycles: 0,
                    },
                },
            );
            if let crate::world::LeaseAsset::Dwelling(d) = asset {
                if let Some(dw) = world.dwellings.get_mut(d) {
                    dw.lease = Some(*contract);
                }
                let taken: Vec<crate::ids::OfferId> = world
                    .offers
                    .iter()
                    .filter(|(_, o)| {
                        matches!(o.body, crate::world::OfferBody::Lease { asset: crate::world::LeaseAsset::Dwelling(x), .. } if x == *d)
                    })
                    .map(|(id, _)| *id)
                    .collect();
                for id in taken {
                    world.offers.remove(&id);
                }
            }
            if world.next.contract.0 <= contract.0 {
                world.next.contract = contract.next();
            }
        }
        Event::RentPaid {
            contract, amount, ..
        } => {
            if let Some(k) = world.contracts.get_mut(contract) {
                if let crate::world::ContractBody::Lease { missed_cycles, .. } = &mut k.body {
                    *missed_cycles = 0;
                }
                let (owner, tenant) = k.parties;
                debit(world, Holder::from(tenant), Asset::Money(*amount));
                credit(world, Holder::from(owner), Asset::Money(*amount));
            }
        }
        Event::RentMissed { contract, .. } => {
            if let Some(k) = world.contracts.get_mut(contract)
                && let crate::world::ContractBody::Lease { missed_cycles, .. } = &mut k.body
            {
                *missed_cycles += 1;
            }
        }
        Event::LeaseEnded { contract, .. } => {
            if let Some(k) = world.contracts.get_mut(contract) {
                k.status = crate::world::ContractStatus::Ended;
            }
        }
        Event::CreditOffered { offer, by, body } => {
            if let crate::world::OfferBody::Credit { principal, .. } = body {
                debit(world, Holder::from(*by), Asset::Money(*principal));
                world.escrow.insert(
                    crate::world::EscrowKey::Offer(*offer),
                    Asset::Money(*principal),
                );
            }
            world.offers.insert(
                *offer,
                crate::world::Offer {
                    id: *offer,
                    by: *by,
                    created_tick: world.meta.tick,
                    body: body.clone(),
                },
            );
            bump_offer(world, *offer);
        }
        Event::CreditAccepted {
            contract,
            lender,
            borrower,
            principal,
            rate_per_cycle_bp,
            installment,
            installments,
            collateral,
        } => {
            // The principal leaves the offer's escrow for the borrower.
            let offer_id = world
                .offers
                .iter()
                .find(|(_, o)| {
                    o.by == *lender
                        && matches!(o.body, crate::world::OfferBody::Credit { principal: p, .. } if p == *principal)
                })
                .map(|(id, _)| *id);
            if let Some(id) = offer_id {
                world.escrow.remove(&crate::world::EscrowKey::Offer(id));
                world.offers.remove(&id);
            }
            credit(world, Holder::from(*borrower), Asset::Money(*principal));
            if let Some(crate::world::Collateral::Shares(org, qty)) = collateral
                && let Some(h) = crate::shares::holder_of(*borrower, *org)
            {
                move_shares(world, *org, Some(h), None, *qty);
                world
                    .share_escrow
                    .insert(crate::world::EscrowKey::Contract(*contract), (*org, *qty));
            }
            world.contracts.insert(
                *contract,
                crate::world::Contract {
                    id: *contract,
                    parties: (*lender, *borrower),
                    created_tick: world.meta.tick,
                    term_cycles: Some(*installments),
                    status: crate::world::ContractStatus::Active,
                    body: crate::world::ContractBody::Credit {
                        principal: *principal,
                        rate_per_cycle_bp: *rate_per_cycle_bp,
                        installment: *installment,
                        installments_left: *installments,
                        collateral: *collateral,
                        missed: false,
                    },
                },
            );
            if world.next.contract.0 <= contract.0 {
                world.next.contract = contract.next();
            }
        }
        Event::CreditInstallment {
            contract,
            amount,
            remaining,
            ..
        } => {
            if let Some(k) = world.contracts.get_mut(contract) {
                if let crate::world::ContractBody::Credit {
                    installments_left,
                    missed,
                    ..
                } = &mut k.body
                {
                    *installments_left = *remaining;
                    *missed = false;
                }
                let (lender, borrower) = k.parties;
                debit(world, Holder::from(borrower), Asset::Money(*amount));
                credit(world, Holder::from(lender), Asset::Money(*amount));
            }
        }
        Event::CreditMissed { contract, .. } => {
            if let Some(k) = world.contracts.get_mut(contract)
                && let crate::world::ContractBody::Credit { missed, .. } = &mut k.body
            {
                *missed = true;
            }
        }
        Event::CreditRepaid { contract } => {
            if let Some(k) = world.contracts.get_mut(contract) {
                k.status = crate::world::ContractStatus::Ended;
                let borrower = k.parties.1;
                if let Some((org, qty)) = world
                    .share_escrow
                    .remove(&crate::world::EscrowKey::Contract(*contract))
                    && let Some(h) = crate::shares::holder_of(borrower, org)
                {
                    move_shares(world, org, None, Some(h), qty);
                }
            }
        }
        Event::CreditDefaulted {
            contract,
            collateral_seized,
        } => {
            let Some((lender, borrower)) = world.contracts.get(contract).map(|k| k.parties) else {
                return;
            };
            if let Some(k) = world.contracts.get_mut(contract) {
                k.status = crate::world::ContractStatus::Ended;
            }
            match collateral_seized {
                Some(crate::world::Collateral::Dwelling(d)) => {
                    if let Some(dw) = world.dwellings.get_mut(d) {
                        dw.owner = crate::housing::owner_of(lender);
                    }
                }
                Some(crate::world::Collateral::Shares(org, _)) => {
                    if let Some((_, qty)) = world
                        .share_escrow
                        .remove(&crate::world::EscrowKey::Contract(*contract))
                        && let Some(h) = crate::shares::holder_of(lender, *org)
                    {
                        move_shares(world, *org, None, Some(h), qty);
                    }
                }
                None => {}
            }
            if let Party::Citizen(c) = borrower
                && let Some(cz) = world.citizens.get_mut(&c)
            {
                cz.flags.defaulted = true;
            }
        }
        Event::MembershipRequested {
            offer,
            org,
            citizen,
        } => {
            world.offers.insert(
                *offer,
                crate::world::Offer {
                    id: *offer,
                    by: Party::Citizen(*citizen),
                    created_tick: world.meta.tick,
                    body: crate::world::OfferBody::Membership {
                        org: *org,
                        citizen: *citizen,
                    },
                },
            );
            bump_offer(world, *offer);
        }
        Event::MemberAdmitted { org, citizen } => {
            if let Some(o) = world.orgs.get_mut(org) {
                o.members.insert(*citizen);
            }
            let requests: Vec<crate::ids::OfferId> = world
                .offers
                .iter()
                .filter(|(_, f)| {
                    matches!(f.body, crate::world::OfferBody::Membership { org: x, citizen: c } if x == *org && c == *citizen)
                })
                .map(|(id, _)| *id)
                .collect();
            for id in requests {
                world.offers.remove(&id);
            }
        }
        Event::MemberLeft { org, citizen } => {
            if let Some(o) = world.orgs.get_mut(org) {
                o.members.remove(citizen);
            }
        }
        Event::CitizenSeen { citizen, tick, .. } => {
            if let Some(c) = world.citizens.get_mut(citizen) {
                c.last_seen_tick = *tick;
            }
        }
        Event::CycleClosed {
            low_population_cycles,
            ..
        } => {
            world.meta.low_population_cycles = *low_population_cycles;
            for o in world.orgs.values_mut() {
                o.declared_dividend = None;
            }
        }
        Event::EpochEnded { reason, .. } => {
            world.meta.epoch_ended = Some(*reason);
        }
        Event::TickResolved {
            tick,
            citizen_deltas,
            workplace_deltas,
            price_index,
            ..
        } => {
            world.meta.tick = tick.wrapping_add(1);
            world.price_index = *price_index;
            for b in world.books.values_mut() {
                b.tick_volume = 0;
                b.tick_value = Money::ZERO;
            }
            for d in citizen_deltas {
                apply_citizen_delta(world, d);
            }
            for d in workplace_deltas {
                apply_workplace_delta(world, d);
            }
        }
        _ => {
            debug_assert!(
                UNIMPLEMENTED.contains(&event.kind()),
                "apply: {} is neither implemented nor listed as unimplemented",
                event.kind()
            );
        }
    }
}

fn set_labor(world: &mut World, citizen: CitizenId, allocations: &[crate::world::Allocation]) {
    if let Some(c) = world.citizens.get_mut(&citizen) {
        c.labor.allocations = allocations.to_vec();
    }
    // Mirror into the workplaces' worker tables.
    let by_workplace: BTreeMap<_, _> = allocations
        .iter()
        .map(|a| (a.workplace, (a.hours, a.effort)))
        .collect();
    for wp in world.workplaces.values_mut() {
        if let Some(assign) = wp.workers.get_mut(&citizen) {
            if let Some((hours, effort)) = by_workplace.get(&wp.id) {
                assign.hours = *hours;
                assign.effort = *effort;
            } else {
                assign.hours = 0;
            }
        }
    }
}

/// Settle one fill: goods leave the ask's escrow to the buyer; money leaves the
/// bid's escrow to the seller at the trade price, and the bid's price improvement
/// (`limit - price`) x qty is refunded to the buyer. Fully filled orders are removed.
#[allow(clippy::too_many_arguments)]
fn apply_trade(
    world: &mut World,
    instrument: crate::world::Instrument,
    buyer: Party,
    seller: Party,
    buy_order: crate::ids::OrderId,
    sell_order: crate::ids::OrderId,
    qty: u32,
    price: Money,
    tick: crate::ids::Tick,
) {
    use crate::world::EscrowKey;
    let paid = Money(price.0 * i64::from(qty));
    let Some(book) = world.books.get_mut(&instrument) else {
        return;
    };
    // Buy side.
    let mut buy_done = false;
    let mut refund = Money::ZERO;
    if let Some(b) = book.orders.get_mut(&buy_order) {
        b.remaining = b.remaining.saturating_sub(qty);
        refund = Money((b.limit_price.0 - price.0) * i64::from(qty));
        buy_done = b.remaining == 0;
    }
    // Sell side.
    let mut sell_done = false;
    if let Some(s) = book.orders.get_mut(&sell_order) {
        s.remaining = s.remaining.saturating_sub(qty);
        sell_done = s.remaining == 0;
    }
    book.last_price = Some(price);
    book.last_trade_tick = Some(tick);
    book.tick_volume += qty;
    book.tick_value += paid;
    if buy_done {
        book.orders.remove(&buy_order);
    }
    if sell_done {
        book.orders.remove(&sell_order);
    }
    // Escrow bookkeeping.
    let buy_key = EscrowKey::Order(buy_order);
    if let Some(Asset::Money(m)) = world.escrow.get_mut(&buy_key) {
        *m -= paid + refund;
    }
    if buy_done {
        world.escrow.remove(&buy_key);
    }
    let sell_key = EscrowKey::Order(sell_order);
    match instrument {
        crate::world::Instrument::Good(g) => {
            if let Some(Asset::Good(_, q)) = world.escrow.get_mut(&sell_key) {
                *q = q.saturating_sub(qty);
            }
            if sell_done {
                world.escrow.remove(&sell_key);
            }
            credit(world, Holder::from(buyer), Asset::Good(g, qty));
        }
        crate::world::Instrument::Share(org) => {
            if let Some((_, q)) = world.share_escrow.get_mut(&sell_key) {
                *q = q.saturating_sub(u64::from(qty));
            }
            if sell_done {
                world.share_escrow.remove(&sell_key);
                world.escrow.remove(&sell_key);
            }
            if let Some(h) = crate::shares::holder_of(buyer, org) {
                move_shares(world, org, None, Some(h), u64::from(qty));
            }
        }
    }
    credit(world, Holder::from(seller), Asset::Money(paid));
    if refund > Money::ZERO {
        credit(world, Holder::from(buyer), Asset::Money(refund));
    }
}

/// Move shares between holders; `None` on either side is the escrow.
fn move_shares(
    world: &mut World,
    org: crate::ids::OrgId,
    from: Option<crate::world::ShareHolder>,
    to: Option<crate::world::ShareHolder>,
    qty: u64,
) {
    let Some(o) = world.orgs.get_mut(&org) else {
        return;
    };
    let crate::world::Ownership::Shares { holdings, .. } = &mut o.ownership else {
        return;
    };
    if let Some(f) = from
        && let Some(h) = holdings.get_mut(&f)
    {
        *h = h.saturating_sub(qty);
        if *h == 0 {
            holdings.remove(&f);
        }
    }
    if let Some(t) = to {
        *holdings.entry(t).or_insert(0) += qty;
    }
}

fn bump_offer(world: &mut World, id: crate::ids::OfferId) {
    if world.next.offer.0 <= id.0 {
        world.next.offer = id.next();
    }
}

/// Move every contract the citizen is party to from one status to another.
fn set_contract_status(
    world: &mut World,
    citizen: CitizenId,
    from: crate::world::ContractStatus,
    to: crate::world::ContractStatus,
) {
    let p = Party::Citizen(citizen);
    for k in world.contracts.values_mut() {
        if (k.parties.0 == p || k.parties.1 == p) && k.status == from {
            k.status = to;
        }
    }
}

fn set_flag(world: &mut World, citizen: CitizenId, f: impl FnOnce(&mut CitizenFlags)) {
    if let Some(c) = world.citizens.get_mut(&citizen) {
        f(&mut c.flags);
    }
}

fn join(
    world: &mut World,
    id: CitizenId,
    handle: &str,
    kind: CitizenKind,
    endowment: Money,
    dwelling: Option<crate::ids::DwellingId>,
) {
    let p = &world.params;
    let citizen = Citizen {
        id,
        handle: handle.to_owned(),
        kind,
        joined_tick: world.meta.tick,
        last_seen_tick: world.meta.tick,
        dormant: false,
        household: Household {
            balance: endowment,
            pantry: BTreeMap::new(),
            dwelling,
        },
        labor: LaborState {
            allocations: Vec::new(),
            budget: p.labor.base_budget_hours,
            fatigue_debt: 0,
            consecutive_high_effort_cycles: 0,
            skill: BTreeMap::new(),
            output_mult: 1.0,
        },
        needs: Needs::at_start(p),
        plan: StandingPlan {
            labor: LaborPlan::Explicit,
            keep_food_at_least: p.householder.keep_food_at_least,
            max_food_price: None,
            buy_wares_when: None,
            keep_balance_at_least: Money::ZERO,
            standing_orders: Vec::new(),
            vote_default: VoteDefault::Abstain,
        },
        flags: CitizenFlags::default(),
        api_share: BTreeMap::new(),
    };
    world.ledger_meta.minted += endowment;
    if let Some(d) = dwelling
        && let Some(dw) = world.dwellings.get_mut(&d)
    {
        dw.occupant = Some(id);
    }
    world.citizens.insert(id, citizen);
    if world.next.citizen.0 <= id.0 {
        world.next.citizen = id.next();
    }
}

fn apply_citizen_delta(world: &mut World, d: &CitizenDelta) {
    let Some(c) = world.citizens.get_mut(&d.citizen) else {
        return;
    };
    c.needs = d.needs.clone();
    c.labor.budget = d.budget;
    c.labor.fatigue_debt = d.fatigue_debt;
    c.labor.consecutive_high_effort_cycles = d.consecutive_high_effort_cycles;
    c.labor.output_mult = d.output_mult;
    c.labor.skill.clone_from(&d.skill);
    take_from_pantry(&mut c.household.pantry, Good::Food, d.food_eaten);
    take_from_pantry(&mut c.household.pantry, Good::Wares, d.wares_consumed);
    LedgerMeta::add(&mut world.ledger_meta.consumed, Good::Food, d.food_eaten);
    LedgerMeta::add(
        &mut world.ledger_meta.consumed,
        Good::Wares,
        d.wares_consumed,
    );
}

fn apply_workplace_delta(world: &mut World, d: &WorkplaceDelta) {
    if let Some(w) = world.workplaces.get_mut(&d.workplace) {
        w.machine_wear = d.machine_wear;
        w.output_remainder = d.output_remainder;
        w.cycle_output = d.cycle_output;
        for (cid, wc) in &d.workers {
            if let Some(a) = w.workers.get_mut(cid) {
                a.cycle_tick_hours = wc.tick_hours;
                a.cycle_attributed = wc.attributed;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_org_founded(
    world: &mut World,
    org: crate::ids::OrgId,
    kind: crate::kinds::OrgKind,
    name: &str,
    founder: Option<CitizenId>,
    ownership: &crate::world::Ownership,
    manager: Option<CitizenId>,
    fee_burned: Money,
) {
    if let Some(f) = founder {
        debit(world, Holder::Citizen(f), Asset::Money(fee_burned));
    }
    world.ledger_meta.burned_money += fee_burned;
    world.orgs.insert(
        org,
        crate::world::Org {
            id: org,
            kind,
            name: name.to_owned(),
            ownership: ownership.clone(),
            manager,
            treasury: Money::ZERO,
            inventory: BTreeMap::new(),
            workplaces: std::collections::BTreeSet::new(),
            employees: std::collections::BTreeSet::new(),
            members: founder.into_iter().collect(),
            founded_tick: world.meta.tick,
            payment_missed: false,
            declared_dividend: None,
        },
    );
    if world.next.org.0 <= org.0 {
        world.next.org = org.next();
    }
}

fn apply_workplace_added(
    world: &mut World,
    workplace: crate::ids::WorkplaceId,
    org: crate::ids::OrgId,
    kind: crate::kinds::WorkplaceKind,
    slot: Option<crate::ids::SlotId>,
    materials_consumed: u32,
) {
    debit(
        world,
        Holder::Org(org),
        Asset::Good(Good::Materials, materials_consumed),
    );
    LedgerMeta::add(
        &mut world.ledger_meta.consumed,
        Good::Materials,
        materials_consumed,
    );
    world.workplaces.insert(
        workplace,
        crate::world::Workplace {
            id: workplace,
            kind,
            org,
            slot,
            machines: 0,
            machine_wear: 0.0,
            output_remainder: 0.0,
            workers: BTreeMap::new(),
            cycle_output: 0.0,
            target: None,
        },
    );
    if let Some(o) = world.orgs.get_mut(&org) {
        o.workplaces.insert(workplace);
    }
    if let Some(s) = slot.and_then(|s| world.land.slots.get_mut(&s)) {
        s.workplace = Some(workplace);
    }
    if world.next.workplace.0 <= workplace.0 {
        world.next.workplace = workplace.next();
    }
}

fn apply_produced(
    world: &mut World,
    workplace: crate::ids::WorkplaceId,
    output: Good,
    units: u32,
    inputs_consumed: &BTreeMap<Good, u32>,
) {
    let Some(org) = world.workplaces.get(&workplace).map(|w| w.org) else {
        return;
    };
    for (g, q) in inputs_consumed {
        debit(world, Holder::Org(org), Asset::Good(*g, *q));
        LedgerMeta::add(&mut world.ledger_meta.consumed, *g, *q);
    }
    if units > 0 {
        credit(world, Holder::Org(org), Asset::Good(output, units));
        LedgerMeta::add(&mut world.ledger_meta.produced, output, units);
    }
}

fn take_from_pantry(pantry: &mut BTreeMap<Good, u32>, good: Good, qty: u32) {
    if qty == 0 {
        return;
    }
    if let Some(have) = pantry.get_mut(&good) {
        *have = have.saturating_sub(qty);
        if *have == 0 {
            pantry.remove(&good);
        }
    }
}

/// Add an asset to a holder. Money into a goods-only holder (or vice versa) is
/// a producer bug; `apply` trusts its input and does nothing in that case.
pub(crate) fn credit(world: &mut World, holder: Holder, asset: Asset) {
    match (holder, asset) {
        (Holder::Citizen(id), Asset::Money(m)) => {
            if let Some(c) = world.citizens.get_mut(&id) {
                c.household.balance += m;
            }
        }
        (Holder::Citizen(id), Asset::Good(g, q)) => {
            if let Some(c) = world.citizens.get_mut(&id) {
                *c.household.pantry.entry(g).or_insert(0) += q;
            }
        }
        (Holder::Org(id), Asset::Money(m)) => {
            if let Some(o) = world.orgs.get_mut(&id) {
                o.treasury += m;
            }
        }
        (Holder::Org(id), Asset::Good(g, q)) => {
            if let Some(o) = world.orgs.get_mut(&id) {
                *o.inventory.entry(g).or_insert(0) += q;
            }
        }
        (Holder::Workplace(id), Asset::Good(Good::Machines, q)) => {
            if let Some(w) = world.workplaces.get_mut(&id) {
                w.machines += q;
            }
        }
        (Holder::Store, Asset::Good(g, q)) => {
            if let Some(s) = &mut world.store {
                *s.stock.entry(g).or_insert(0) += q;
            }
        }
        (Holder::StateStock, Asset::Good(g, q)) => {
            if let Some(s) = &mut world.state_stock {
                *s.stock.entry(g).or_insert(0) += q;
            }
        }
        (Holder::StateStock, Asset::Money(m)) => {
            if let Some(s) = &mut world.state_stock {
                s.till += m;
            }
        }
        (Holder::Treasury, Asset::Money(m)) => world.treasury += m,
        // Escrow is keyed per order/offer in `world.escrow` and moved by the
        // market/contract events themselves (S0.7, S0.8); other combinations are
        // producer bugs that `apply` ignores by contract.
        _ => {}
    }
}

/// Remove an asset from a holder (saturating at zero; producers validate).
pub(crate) fn debit(world: &mut World, holder: Holder, asset: Asset) {
    match (holder, asset) {
        (Holder::Citizen(id), Asset::Money(m)) => {
            if let Some(c) = world.citizens.get_mut(&id) {
                c.household.balance -= m;
            }
        }
        (Holder::Citizen(id), Asset::Good(g, q)) => {
            if let Some(c) = world.citizens.get_mut(&id) {
                take_from_pantry(&mut c.household.pantry, g, q);
            }
        }
        (Holder::Org(id), Asset::Money(m)) => {
            if let Some(o) = world.orgs.get_mut(&id) {
                o.treasury -= m;
            }
        }
        (Holder::Org(id), Asset::Good(g, q)) => {
            if let Some(o) = world.orgs.get_mut(&id) {
                take_from_pantry(&mut o.inventory, g, q);
            }
        }
        (Holder::Workplace(id), Asset::Good(Good::Machines, q)) => {
            if let Some(w) = world.workplaces.get_mut(&id) {
                w.machines = w.machines.saturating_sub(q);
            }
        }
        (Holder::Store, Asset::Good(g, q)) => {
            if let Some(s) = &mut world.store {
                take_from_pantry(&mut s.stock, g, q);
            }
        }
        (Holder::StateStock, Asset::Good(g, q)) => {
            if let Some(s) = &mut world.state_stock {
                take_from_pantry(&mut s.stock, g, q);
            }
        }
        (Holder::StateStock, Asset::Money(m)) => {
            if let Some(s) = &mut world.state_stock {
                s.till -= m;
            }
        }
        (Holder::Treasury, Asset::Money(m)) => world.treasury -= m,
        _ => {}
    }
}

/// Convenience for producers: a party's current holding of a good.
#[must_use]
pub fn goods_of(world: &World, party: Party, good: Good) -> u32 {
    match party {
        Party::Citizen(id) => world
            .citizens
            .get(&id)
            .and_then(|c| c.household.pantry.get(&good).copied())
            .unwrap_or(0),
        Party::Org(id) => world
            .orgs
            .get(&id)
            .and_then(|o| o.inventory.get(&good).copied())
            .unwrap_or(0),
    }
}

/// Convenience for producers: a party's current money.
#[must_use]
pub fn money_of(world: &World, party: Party) -> Money {
    match party {
        Party::Citizen(id) => world
            .citizens
            .get(&id)
            .map_or(Money::ZERO, |c| c.household.balance),
        Party::Org(id) => world.orgs.get(&id).map_or(Money::ZERO, |o| o.treasury),
    }
}

#[cfg(test)]
mod tests;
