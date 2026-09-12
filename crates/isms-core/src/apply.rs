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
pub const UNIMPLEMENTED: &[&str] = &[
    "HouseholderEmigrated",
    "OrderPlaced",
    "OrderCancelled",
    "OrderExpired",
    "Trade",
    "MemberAdmitted",
    "MemberLeft",
    "SharesIssued",
    "SharesTransferred",
    "DividendDeclared",
    "DividendPaid",
    "DwellingBuilt",
    "DwellingTransferred",
    "DwellingOccupied",
    "EmploymentOffered",
    "EmploymentAccepted",
    "EmploymentTerminated",
    "CreditOffered",
    "CreditAccepted",
    "CreditInstallment",
    "CreditRepaid",
    "CreditDefaulted",
    "LeaseOffered",
    "LeaseAccepted",
    "RentPaid",
    "RentMissed",
    "LeaseEnded",
    "Paid",
    "PaymentMissed",
    "Drew",
    "PolicyChanged",
];

/// Fold one event into the world.
#[allow(clippy::too_many_lines)] // a flat dispatcher; arms stay one-liners or calls
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
        }
        Event::CitizenReturned { citizen } => {
            if let Some(c) = world.citizens.get_mut(citizen) {
                c.dormant = false;
            }
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
            let escrowed = match asset {
                crate::world::SaleAsset::Good(g, q) => Some(Asset::Good(*g, *q)),
                crate::world::SaleAsset::Shares(..) | crate::world::SaleAsset::Dwelling(_) => None,
            };
            if let Some(a) = escrowed {
                debit(world, Holder::from(*by), a);
                world
                    .escrow
                    .insert(crate::world::EscrowKey::Offer(*offer), a);
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
            price,
            ..
        } => {
            if let Some(a) = world.escrow.remove(&crate::world::EscrowKey::Offer(*offer)) {
                credit(world, Holder::from(*buyer), a);
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
            if let (Some(a), Some(o)) = (
                world.escrow.remove(&crate::world::EscrowKey::Offer(*offer)),
                world.offers.get(offer),
            ) {
                let by = o.by;
                credit(world, Holder::from(by), a);
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
        }
        Event::EpochEnded { reason, .. } => {
            world.meta.epoch_ended = Some(*reason);
        }
        Event::TickResolved {
            tick,
            citizen_deltas,
            workplace_deltas,
            ..
        } => {
            world.meta.tick = tick.wrapping_add(1);
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

fn bump_offer(world: &mut World, id: crate::ids::OfferId) {
    if world.next.offer.0 <= id.0 {
        world.next.offer = id.next();
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
