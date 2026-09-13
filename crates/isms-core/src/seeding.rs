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
            if money && p.money.legacy_treasury > Money::ZERO {
                events.push(Event::Seeded {
                    holder: Holder::Org(org),
                    asset: Asset::Money(p.money.legacy_treasury),
                });
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
    let mut new_hh: Vec<CitizenId> = Vec::new();
    for _ in 0..fill {
        let citizen = next_cit;
        next_cit = next_cit.next();
        events.push(householder_joined(world, citizen));
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
            events.push(Event::ManagerAppointed {
                org: *org,
                citizen: Some(pool[i % pool.len()]),
            });
        }
    }
    let _ = DwellingId(0);
    events
}

fn householder_joined(world: &World, citizen: CitizenId) -> Event {
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
        dwelling: None,
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
            let e = householder_joined(&b.world, citizen);
            b.emit(e);
        }
    } else if have > target {
        let excess = usize::try_from(have - target).unwrap_or(0);
        let leaving: Vec<CitizenId> = current.iter().rev().take(excess).copied().collect();
        for id in leaving {
            emigrate(b, id);
        }
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
    // Burn what is left.
    let c = &b.world.citizens[&id];
    let burned_money = c.household.balance;
    let burned_goods = c.household.pantry.clone();
    let explain = Explain::new(
        RuleId::Emigration,
        "balance and pantry burned",
        burned_money,
    )
    .input("balance", burned_money);
    b.emit(Event::HouseholderEmigrated {
        citizen: id,
        burned_money,
        burned_goods,
        explain,
    });
    let _ = (ContractKind::Employment, Good::Food, WorkplaceId(0));
}
