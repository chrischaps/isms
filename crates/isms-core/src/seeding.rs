//! Epoch seeding and the householder population (GDD §11.3; TDD §5.5 step 8l,
//! §9.3, ADR-0003, ADR-0004, T19). Everything here is events: `start_epoch`
//! returns them and the tick emits fill and emigration directly.

use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, DwellingId, Epoch, OrgId, WorkplaceId};
use crate::kinds::{CitizenKind, ContractKind, Good, OrgKind, WorkplaceKind};
use crate::ledger::{Asset, Holder, Party};
use crate::money::Money;
use crate::rules::Rules;
use crate::tick::TickBuilder;
use crate::world::{
    ContractBody, ContractStatus, LandRegistry, LeaseAsset, Ownership, ShareHolder, World,
};
use std::collections::BTreeMap;

/// The org kind a seeded workplace belongs to, per the constitution.
#[must_use]
pub fn seeded_org_kind(world: &World) -> OrgKind {
    match world.constitution.ownership {
        crate::constitution::Ownership::Private => OrgKind::Firm,
        crate::constitution::Ownership::Cooperative => OrgKind::Cooperative,
        crate::constitution::Ownership::Collective => match world.constitution.pricing {
            crate::constitution::Pricing::Administered => OrgKind::StateEnterprise,
            _ => OrgKind::Collective,
        },
    }
}

fn ownership_for(kind: OrgKind, shares: u64) -> Ownership {
    match kind {
        OrgKind::Firm => Ownership::Shares {
            issued: shares,
            holdings: BTreeMap::from([(ShareHolder::OrgSelf, shares)]),
        },
        OrgKind::Cooperative | OrgKind::Association | OrgKind::Union => Ownership::Members,
        OrgKind::Collective | OrgKind::StateEnterprise => Ownership::Society,
    }
}

fn display(kind: WorkplaceKind) -> &'static str {
    match kind {
        WorkplaceKind::Farm => "Farm",
        WorkplaceKind::Mine => "Mine",
        WorkplaceKind::Mill => "Mill",
        WorkplaceKind::Foundry => "Foundry",
        WorkplaceKind::Workshop => "Workshop",
        WorkplaceKind::MachineShop => "Machine Shop",
        WorkplaceKind::Builder => "Builders",
    }
}

/// Start an epoch: `EpochStarted`, then the legacy orgs (one per seeded
/// workplace, with a treasury where money exists), the initial dwellings
/// (owned by the Builders, round-robin), householders up to the population
/// floor, and a householder manager per org. `world` must be a fresh society
/// (epoch 0) or one whose epoch has ended (S0.13 resets material state).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn start_epoch(world: &World, _rules: &Rules, epoch: Epoch) -> Vec<Event> {
    let mut events = vec![Event::EpochStarted { epoch }];
    let p = &world.params;
    let money = world.constitution.has_money();
    let kind = seeded_org_kind(world);
    let mut land = LandRegistry::from_params(p);
    let mut next_org = world.next.org;
    let mut next_wp = world.next.workplace;
    let mut next_dw = world.next.dwelling;
    let mut next_cit = world.next.citizen;
    let mut builders: Vec<OrgId> = Vec::new();
    let mut orgs: Vec<OrgId> = Vec::new();
    let mut seeded_workplaces: Vec<(WorkplaceId, WorkplaceKind, u32)> = Vec::new();
    // Society-owned orgs keep their goods in the society's stock (Q48).
    let society_owned = matches!(ownership_for(kind, 0), Ownership::Society);
    let society_stock = if society_owned && world.store.is_some() {
        Some(Holder::Store)
    } else if society_owned && world.state_stock.is_some() {
        Some(Holder::StateStock)
    } else {
        None
    };

    for wk in WorkplaceKind::ALL {
        let n = p.seeded_workplaces.get(&wk).copied().unwrap_or(0);
        for i in 0..n {
            let org = next_org;
            next_org = next_org.next();
            let workplace = next_wp;
            next_wp = next_wp.next();
            let slot = land.free_slot(wk);
            if let Some(s) = slot {
                land.slots.get_mut(&s).expect("slot").workplace = Some(workplace);
            }
            events.push(Event::OrgFounded {
                org,
                kind,
                name: format!("Legacy {} No. {}", display(wk), i + 1),
                founder: None,
                ownership: ownership_for(kind, u64::from(p.founding.initial_shares)),
                manager: None,
                fee_burned: Money::ZERO,
            });
            events.push(Event::WorkplaceAdded {
                workplace,
                org,
                kind: wk,
                slot,
                materials_consumed: 0,
            });
            seeded_workplaces.push((workplace, wk, 0u32));
            if money && p.money.legacy_treasury > Money::ZERO {
                // State enterprises share one purse: the till (Q67).
                let purse = match society_stock {
                    Some(Holder::StateStock) => Holder::StateStock,
                    _ => Holder::Org(org),
                };
                events.push(Event::Seeded {
                    holder: purse,
                    asset: Asset::Money(p.money.legacy_treasury),
                });
            }
            if let Some(stock) = p.seeding.legacy_inventory.get(&wk) {
                for (good, qty) in stock {
                    if *qty > 0 {
                        events.push(Event::Seeded {
                            holder: society_stock.unwrap_or(Holder::Org(org)),
                            asset: Asset::Good(*good, *qty),
                        });
                    }
                }
            }
            if wk == WorkplaceKind::Builder {
                builders.push(org);
            }
            orgs.push(org);
        }
    }
    // Initial dwellings, owned by the Builders (or the first org if there are none).
    let owners: Vec<OrgId> = if builders.is_empty() {
        orgs.iter().take(1).copied().collect()
    } else {
        builders
    };
    let first_dwelling = next_dw;
    if !owners.is_empty() {
        for i in 0..p.initial_dwellings {
            let dwelling = next_dw;
            next_dw = next_dw.next();
            events.push(Event::DwellingBuilt {
                dwelling,
                org: owners[usize::try_from(i).unwrap_or(0) % owners.len()],
                workplace: None,
                materials_consumed: 0,
            });
        }
    }
    // Public dwellings (tax-transfer systems, Q81): built by the Builders and
    // handed to the society, so the shared assignment rule houses the unhoused.
    if let Some(n) = world.policy.public_dwellings
        && !owners.is_empty()
    {
        for i in 0..n {
            let dwelling = next_dw;
            next_dw = next_dw.next();
            events.push(Event::DwellingBuilt {
                dwelling,
                org: owners[usize::try_from(i).unwrap_or(0) % owners.len()],
                workplace: None,
                materials_consumed: 0,
            });
            events.push(Event::DwellingTransferred {
                dwelling,
                to: crate::world::Owner::Society,
            });
        }
    }
    // Householders up to the floor.
    // A later epoch resets the roster first (Q41): humans start dormant and the
    // old householders are gone, so the fill is the whole floor.
    let (active_humans, householders) = if epoch == 0 {
        (
            world
                .citizens
                .values()
                .filter(|c| c.kind == CitizenKind::Human && !c.dormant)
                .count(),
            world
                .citizens
                .values()
                .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
                .count(),
        )
    } else {
        (0, 0)
    };
    let fill = usize::try_from(p.population.floor)
        .unwrap_or(0)
        .saturating_sub(active_humans)
        .saturating_sub(householders);
    // In collective systems the dwellings just built are the society's and are
    // assigned at join (GDD 6.2), one per householder while they last.
    let public_from = first_dwelling.0 + p.initial_dwellings;
    let mut society_dwellings = (first_dwelling.0..next_dw.0)
        .map(DwellingId)
        .filter(|d| society_stock.is_some() || d.0 >= public_from);
    let assigned = world.constitution.labor == crate::constitution::LaborMode::Assigned;
    let mut new_hh: Vec<CitizenId> = Vec::new();
    for _ in 0..fill {
        let citizen = next_cit;
        next_cit = next_cit.next();
        events.push(householder_joined(world, citizen, society_dwellings.next()));
        if assigned && let Some(workplace) = balance_seeded(&mut seeded_workplaces, p) {
            events.push(Event::Assigned {
                workplace,
                citizen,
                contract: None,
            });
        }
        new_hh.push(citizen);
    }
    // A householder manager per seeded org, round-robin.
    let existing: Vec<CitizenId> = if epoch == 0 {
        world
            .citizens
            .values()
            .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
            .map(|c| c.id)
            .collect()
    } else {
        Vec::new()
    };
    let pool: Vec<CitizenId> = existing.into_iter().chain(new_hh).collect();
    if !pool.is_empty() {
        for (i, org) in orgs.iter().enumerate() {
            let manager = pool[i % pool.len()];
            events.push(Event::ManagerAppointed {
                org: *org,
                citizen: Some(manager),
            });
            // A cooperative's manager is its first member, at its workplace (Q87).
            if kind == OrgKind::Cooperative {
                events.push(Event::MemberAdmitted {
                    org: *org,
                    citizen: manager,
                });
                if let Some((workplace, _, _)) = seeded_workplaces.get(i) {
                    events.push(Event::Assigned {
                        workplace: *workplace,
                        citizen: manager,
                        contract: None,
                    });
                }
            }
        }
    }
    events
}

/// The balancing rule over workplaces that exist only as events so far: the
/// same choice `orgs::least_staffed` makes on a live world (Q62).
#[allow(clippy::cast_precision_loss)]
fn balance_seeded(
    seeded: &mut [(WorkplaceId, WorkplaceKind, u32)],
    p: &crate::params::Params,
) -> Option<WorkplaceId> {
    let max = p.labor.max_workers_per_workplace;
    let pick = seeded
        .iter()
        .enumerate()
        .filter(|(_, (_, _, n))| *n < max)
        .filter_map(|(i, (id, kind, n))| {
            let w = p.labor.balance_weights.get(kind).copied().unwrap_or(0);
            (w > 0).then(|| (f64::from(*n) / f64::from(w), *id, i))
        })
        .min_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        })?;
    seeded[pick.2].2 += 1;
    Some(pick.1)
}

fn householder_joined(world: &World, citizen: CitizenId, dwelling: Option<DwellingId>) -> Event {
    let (endowment, explain) = if world.constitution.has_money() {
        let e = world.params.money.endowment;
        (
            e,
            Some(Explain::new(RuleId::Endowment, "endowment", e).input("endowment", e)),
        )
    } else {
        (Money::ZERO, None)
    };
    Event::HouseholderJoined {
        citizen,
        handle: format!("H-{}", citizen.0),
        endowment,
        dwelling,
        explain,
    }
}

/// Step 8l: keep householders at `max(0, floor - active_humans)`: join new ones
/// or emigrate the most recently joined (Q37), liquidating per T19.
pub fn cycle_end_8l_householder_fill(b: &mut TickBuilder) {
    let floor = b.world.params.population.floor;
    let active_humans = b.active_humans();
    let target = floor.saturating_sub(active_humans);
    let current: Vec<CitizenId> = b
        .world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
        .map(|c| c.id)
        .collect();
    let have = u32::try_from(current.len()).unwrap_or(u32::MAX);
    if have < target {
        for _ in 0..(target - have) {
            let citizen = b.world.next.citizen;
            let dwelling = crate::housing::free_society_dwelling(&b.world);
            let e = householder_joined(&b.world, citizen, dwelling);
            b.emit(e);
            if b.rules.capabilities.labor == crate::constitution::LaborMode::Assigned
                && let Some(workplace) = crate::orgs::least_staffed(&b.world)
            {
                b.emit(Event::Assigned {
                    workplace,
                    citizen,
                    contract: None,
                });
            }
        }
    } else if have > target {
        let excess = usize::try_from(have - target).unwrap_or(0);
        let leaving: Vec<CitizenId> = current.iter().rev().take(excess).copied().collect();
        for id in leaving {
            emigrate(b, id);
        }
        // Dwellings the emigrants released go to anyone still waiting (S0.15).
        crate::housing::cycle_end_8b_assign_dwellings(b);
    }
}

/// Wind a householder down: cancel orders and offers, end contracts by their
/// rules, hand any managed org to another householder, then burn the rest.
#[allow(clippy::too_many_lines)]
fn emigrate(b: &mut TickBuilder, id: CitizenId) {
    let party = Party::Citizen(id);
    // Orders.
    let orders: Vec<(crate::ids::OrderId, Asset)> = b
        .world
        .books
        .values()
        .flat_map(|bk| bk.orders.values())
        .filter(|o| o.owner == party)
        .map(|o| (o.id, crate::market::escrow_of(o)))
        .collect();
    for (order, released) in orders {
        b.emit(Event::OrderCancelled { order, released });
    }
    // Sale and wanted offers.
    let offers: Vec<(crate::ids::OfferId, bool)> = b
        .world
        .offers
        .values()
        .filter(|o| o.by == party)
        .map(|o| (o.id, matches!(o.body, crate::world::OfferBody::Sale { .. })))
        .collect();
    for (offer, sale) in offers {
        if sale {
            b.emit(Event::SaleCancelled { offer });
        } else if matches!(
            b.world.offers.get(&offer).map(|o| &o.body),
            Some(crate::world::OfferBody::Wanted { .. })
        ) {
            b.emit(Event::WantedRemoved { offer });
        }
    }
    // Contracts: employment (worker leaves: forfeits accrued), leases as tenant.
    let contracts: Vec<(crate::ids::ContractId, ContractBody)> = b
        .world
        .contracts
        .values()
        .filter(|k| {
            k.status != ContractStatus::Ended && (k.parties.0 == party || k.parties.1 == party)
        })
        .map(|k| (k.id, k.body.clone()))
        .collect();
    for (cid, body) in contracts {
        match body {
            ContractBody::Employment { workplace, .. } => {
                b.emit(Event::EmploymentTerminated {
                    contract: cid,
                    by: party,
                    notice_pay: Money::ZERO,
                    forfeited: Money::ZERO,
                });
                b.emit(Event::Unassigned {
                    workplace,
                    citizen: id,
                });
            }
            ContractBody::Lease {
                asset: LeaseAsset::Dwelling(d),
                ..
            } => {
                b.emit(Event::LeaseEnded {
                    contract: cid,
                    evicted: false,
                });
                b.emit(Event::DwellingOccupied {
                    dwelling: d,
                    citizen: None,
                });
            }
            _ => {}
        }
    }
    // Positions held without a contract (norm and assigned systems).
    let assigned: Vec<WorkplaceId> = b
        .world
        .workplaces
        .values()
        .filter(|w| w.workers.get(&id).is_some_and(|a| a.contract.is_none()))
        .map(|w| w.id)
        .collect();
    for workplace in assigned {
        b.emit(Event::Unassigned {
            workplace,
            citizen: id,
        });
    }
    // Memberships end (a coop share is forfeited, Q86).
    let memberships: Vec<OrgId> = b
        .world
        .orgs
        .values()
        .filter(|o| o.members.contains(&id))
        .map(|o| o.id)
        .collect();
    for org in memberships {
        b.emit(Event::MemberLeft { org, citizen: id });
    }
    // A society dwelling goes back to the stock.
    if let Some(dwelling) = crate::housing::society_dwelling_of(&b.world, id) {
        b.emit(Event::DwellingOccupied {
            dwelling,
            citizen: None,
        });
    }
    // Managed orgs pass to another householder, or fall vacant.
    let managed: Vec<OrgId> = b
        .world
        .orgs
        .values()
        .filter(|o| o.manager == Some(id))
        .map(|o| o.id)
        .collect();
    let successor = b
        .world
        .citizens
        .values()
        .find(|c| c.kind == CitizenKind::Householder && !c.dormant && c.id != id)
        .map(|c| c.id);
    for org in managed {
        b.emit(Event::ManagerAppointed {
            org,
            citizen: successor,
        });
    }
    // What is left returns to the society's stock where one exists (Q47), else
    // it is burned (T19).
    let c = &b.world.citizens[&id];
    let balance = c.household.balance;
    let pantry = c.household.pantry.clone();
    let society_stock = if b.world.store.is_some() {
        Some(Holder::Store)
    } else if b.world.state_stock.is_some() {
        Some(Holder::StateStock)
    } else {
        None
    };
    if let Some(holder) = society_stock {
        b.emit(Event::StoreReturned {
            citizen: id,
            holder,
            goods: pantry,
            money: balance,
        });
        b.emit(Event::HouseholderEmigrated {
            citizen: id,
            burned_money: Money::ZERO,
            burned_goods: BTreeMap::new(),
            explain: Explain::new(
                RuleId::Emigration,
                "balance and pantry returned to the society",
                Money::ZERO,
            )
            .input("balance", balance),
        });
    } else {
        let explain = Explain::new(RuleId::Emigration, "balance and pantry burned", balance)
            .input("balance", balance);
        b.emit(Event::HouseholderEmigrated {
            citizen: id,
            burned_money: balance,
            burned_goods: pantry,
            explain,
        });
    }
    let _ = (ContractKind::Employment, Good::Food);
}
