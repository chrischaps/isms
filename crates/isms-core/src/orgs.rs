//! Orgs, workplaces, land and machines (GDD §4.4, §6.1, §7.1; TDD §5.3 org rows,
//! step 8f). Founding, adding workplaces on slots, installing machines, and
//! depreciation. Ownership, contracts and payroll arrive in S0.10.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::{Actor, Event};
use crate::explain::{Explain, RuleId};
use crate::ids::{OrgId, SlotId, WorkplaceId};
use crate::kinds::{Good, OrgKind, WorkplaceKind};
use crate::ledger::{Asset, Party};
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::world::{Org, Ownership, ShareHolder, Workplace, World};
use std::collections::BTreeMap;

/// The org an envelope may manage: the actor must be its manager, and if
/// `on_behalf_of` is set it must name the same org.
pub fn managed_org<'w>(
    world: &'w World,
    envelope: &Envelope<Command>,
    org: OrgId,
) -> Result<&'w Org, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let o = world
        .orgs
        .get(&org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
    if let Some(claimed) = envelope.on_behalf_of
        && claimed != org
    {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "on_behalf_of names another org",
        ));
    }
    if o.manager != Some(citizen.id) {
        return Err(Reject::new(
            RejectCode::NotManager,
            format!("{} does not manage {}", citizen.id, o.name),
        ));
    }
    Ok(o)
}

/// A workplace of `kind` needs a free slot if the kind is slot-limited.
pub fn slot_for(
    world: &World,
    kind: WorkplaceKind,
    wanted: Option<SlotId>,
) -> Result<Option<SlotId>, Reject> {
    if !world.land.is_limited(kind) {
        return Ok(None);
    }
    match wanted {
        Some(s) => match world.land.slots.get(&s) {
            Some(slot) if slot.kind == kind && slot.workplace.is_none() => Ok(Some(s)),
            Some(_) => Err(Reject::new(
                RejectCode::NoSlotAvailable,
                format!("slot {s} is not a free {kind:?} slot"),
            )),
            None => Err(Reject::new(
                RejectCode::NoSlotAvailable,
                format!("no slot {s}"),
            )),
        },
        None => world.land.free_slot(kind).map(Some).ok_or_else(|| {
            Reject::new(
                RejectCode::NoSlotAvailable,
                format!("no free {kind:?} slot"),
            )
        }),
    }
}

/// Whether one more worker may hold a position at `workplace` (`max_workers_per_workplace`).
pub fn check_room(world: &World, workplace: WorkplaceId) -> Result<&Workplace, Reject> {
    let wp = world.workplaces.get(&workplace).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownWorkplace,
            format!("no workplace {workplace}"),
        )
    })?;
    let max = world.params.labor.max_workers_per_workplace;
    if u32::try_from(wp.workers.len()).unwrap_or(u32::MAX) >= max {
        return Err(Reject::new(
            RejectCode::WorkplaceFull,
            format!("{workplace} already has {max} workers"),
        ));
    }
    Ok(wp)
}

/// `FoundOrg`: a citizen founds a firm, cooperative or association (collectives
/// and state enterprises belong to the society and are seeded, not founded).
pub fn found_org(
    world: &World,
    envelope: &Envelope<Command>,
    kind: OrgKind,
    name: &str,
    first_workplace: Option<(WorkplaceKind, Option<SlotId>)>,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    if matches!(kind, OrgKind::Collective | OrgKind::StateEnterprise) {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "collective orgs are seeded by the society",
        ));
    }
    if citizen.flags.options_narrowed {
        return Err(Reject::new(
            RejectCode::OptionsNarrowed,
            "a destitute citizen cannot found an org",
        ));
    }
    if name.trim().is_empty() {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "name must not be empty",
        ));
    }
    let p = &world.params;
    let fee = if world.constitution.has_money() {
        p.money.founding_cost_money
    } else {
        Money::ZERO
    };
    if citizen.household.balance < fee {
        return Err(Reject::new(
            RejectCode::InsufficientFunds,
            format!(
                "founding costs {fee}; balance is {}",
                citizen.household.balance
            ),
        ));
    }
    let org = world.next.org;
    let ownership = match kind {
        OrgKind::Firm => Ownership::Shares {
            issued: u64::from(p.founding.initial_shares),
            holdings: BTreeMap::from([(
                ShareHolder::Citizen(citizen.id),
                u64::from(p.founding.initial_shares),
            )]),
        },
        OrgKind::Cooperative | OrgKind::Association | OrgKind::Union => Ownership::Members,
        OrgKind::Collective | OrgKind::StateEnterprise => Ownership::Society,
    };
    let mut events = vec![Event::OrgFounded {
        org,
        kind,
        name: name.to_owned(),
        founder: Some(citizen.id),
        ownership,
        manager: Some(citizen.id),
        fee_burned: fee,
    }];
    if let Some((wp_kind, wanted)) = first_workplace {
        let materials = p.founding.materials;
        let have = citizen
            .household
            .pantry
            .get(&Good::Materials)
            .copied()
            .unwrap_or(0);
        if have < materials {
            return Err(Reject::new(
                RejectCode::InsufficientGoods,
                format!("a workplace needs {materials} Materials; pantry holds {have}"),
            ));
        }
        let slot = slot_for(world, wp_kind, wanted)?;
        events.push(Event::Transferred {
            from: Party::Citizen(citizen.id),
            to: Party::Org(org),
            asset: Asset::Good(Good::Materials, materials),
            memo: "founding".into(),
        });
        events.push(Event::WorkplaceAdded {
            workplace: world.next.workplace,
            org,
            kind: wp_kind,
            slot,
            materials_consumed: materials,
        });
    }
    Ok(events)
}

/// `AddWorkplace`: the manager adds a workplace from org Materials.
pub fn add_workplace(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    kind: WorkplaceKind,
    wanted: Option<SlotId>,
) -> Result<Vec<Event>, Reject> {
    let o = managed_org(world, envelope, org)?;
    let materials = world.params.founding.materials;
    let have = o.inventory.get(&Good::Materials).copied().unwrap_or(0);
    if have < materials {
        return Err(Reject::new(
            RejectCode::InsufficientGoods,
            format!(
                "a workplace needs {materials} Materials; {} holds {have}",
                o.name
            ),
        ));
    }
    let slot = slot_for(world, kind, wanted)?;
    Ok(vec![Event::WorkplaceAdded {
        workplace: world.next.workplace,
        org,
        kind,
        slot,
        materials_consumed: materials,
    }])
}

fn workplace_of_org<'w>(
    world: &'w World,
    org: &Org,
    workplace: WorkplaceId,
) -> Result<&'w Workplace, Reject> {
    let wp = world.workplaces.get(&workplace).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownWorkplace,
            format!("no workplace {workplace}"),
        )
    })?;
    if wp.org != org.id {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            format!("{workplace} is not {}'s", org.name),
        ));
    }
    Ok(wp)
}

/// `InstallMachines`: org inventory -> workplace.
pub fn install_machines(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    workplace: WorkplaceId,
    qty: u32,
) -> Result<Vec<Event>, Reject> {
    let o = managed_org(world, envelope, org)?;
    workplace_of_org(world, o, workplace)?;
    if qty == 0 {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "qty must be positive",
        ));
    }
    let have = o.inventory.get(&Good::Machines).copied().unwrap_or(0);
    if have < qty {
        return Err(Reject::new(
            RejectCode::InsufficientGoods,
            format!("{} holds {have} Machines", o.name),
        ));
    }
    Ok(vec![Event::MachinesInstalled { workplace, qty }])
}

/// `UninstallMachines`: workplace -> org inventory.
pub fn uninstall_machines(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    workplace: WorkplaceId,
    qty: u32,
) -> Result<Vec<Event>, Reject> {
    let o = managed_org(world, envelope, org)?;
    let wp = workplace_of_org(world, o, workplace)?;
    if qty == 0 || qty > wp.machines {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            format!("{workplace} has {} machines", wp.machines),
        ));
    }
    Ok(vec![Event::MachinesUninstalled { workplace, qty }])
}

/// `AppointManager` by the controlling owner (firms) or, until member votes
/// arrive, by the current manager (associations and cooperatives).
pub fn appoint_manager(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    citizen: Option<crate::ids::CitizenId>,
) -> Result<Vec<Event>, Reject> {
    let actor = acting_citizen(world, envelope)?;
    let o = world
        .orgs
        .get(&org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
    let allowed = match &o.ownership {
        Ownership::Shares { .. } => controlling_owner(o) == Some(actor.id),
        Ownership::Members => o.manager == Some(actor.id),
        Ownership::Society => envelope.actor == Actor::System,
    };
    if !allowed {
        return Err(Reject::new(
            RejectCode::NotControllingOwner,
            "only the controlling owner appoints",
        ));
    }
    if let Some(c) = citizen
        && !world.citizens.contains_key(&c)
    {
        return Err(Reject::new(
            RejectCode::UnknownCitizen,
            format!("no citizen {c}"),
        ));
    }
    Ok(vec![Event::ManagerAppointed { org, citizen }])
}

/// The citizen holding more than half of the issued shares, if any (ADR-0005).
#[must_use]
pub fn controlling_owner(org: &Org) -> Option<crate::ids::CitizenId> {
    let Ownership::Shares { issued, holdings } = &org.ownership else {
        return None;
    };
    holdings.iter().find_map(|(h, q)| match h {
        ShareHolder::Citizen(c) if *q * 2 > *issued => Some(*c),
        _ => None,
    })
}

/// Step 8f: machines wear at `machine_depreciation_per_cycle`; whole machines
/// lost are emitted as `MachinesDepreciated`, the fraction is carried as wear.
pub fn cycle_end_8f_depreciation(b: &mut TickBuilder) {
    let rate = b.world.params.capital.machine_depreciation_per_cycle;
    let ids: Vec<WorkplaceId> = b.world.workplaces.keys().copied().collect();
    for id in ids {
        let wp = b.world.workplaces.get_mut(&id).expect("workplace exists");
        if wp.machines == 0 {
            wp.machine_wear = 0.0;
            continue;
        }
        let wear = wp.machine_wear + f64::from(wp.machines) * rate;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let lost = (wear.floor() as u32).min(wp.machines);
        wp.machine_wear = wear - f64::from(lost);
        if lost > 0 {
            let explain = Explain::new(
                RuleId::Depreciation,
                "machines * rate, whole units lost",
                lost,
            )
            .input("machines", wp.machines)
            .input("rate_per_cycle", rate)
            .input("wear_before", wear);
            b.emit(Event::MachinesDepreciated {
                workplace: id,
                qty: lost,
                explain,
            });
        }
    }
}
