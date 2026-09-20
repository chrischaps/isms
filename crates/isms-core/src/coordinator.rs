//! The coordinator's land powers (GDD §6.2 Governance, §8.2; TDD S2.3): a
//! sitting Coordinator opens and closes the collective's workplaces on the
//! land slots. Opening spends the founding Materials from the Common Store,
//! the same figure a founder or manager pays; closing unassigns the workers
//! that tick, returns the machines to the collective's stock and frees the
//! slot (Q120). Both are powers, not votes: instant at the command. The
//! coordinator's other powers live where their objects do: the advisory Plan
//! in `planning.rs`, the rationing proposal in `governance.rs`.
//!
//! A citizen who does not sit is refused `NotAnOfficeHolder`, the code S2.2
//! chose for every power of an office.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::constitution::OfficeKind;
use crate::event::Event;
use crate::ids::{SlotId, WorkplaceId};
use crate::kinds::{Good, OrgKind, WorkplaceKind};
use crate::world::{Citizen, Org, World};

/// The acting citizen when they sit as `office`; `NotAnOfficeHolder` otherwise.
pub fn office_holder<'w>(
    world: &'w World,
    envelope: &Envelope<Command>,
    office: OfficeKind,
) -> Result<&'w Citizen, Reject> {
    let actor = acting_citizen(world, envelope)?;
    if !world.offices.holds(actor.id, office) {
        return Err(Reject::new(
            RejectCode::NotAnOfficeHolder,
            format!("{} does not sit as {office:?}", actor.id),
        ));
    }
    Ok(actor)
}

/// The society's collective: the org the coordinators open workplaces for.
/// The Commune seeds exactly one (`seeding.rs`); the lowest id wins if a
/// preset ever seeds more.
fn collective(world: &World) -> Result<&Org, Reject> {
    world
        .orgs
        .values()
        .find(|o| o.kind == OrgKind::Collective)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::NotInThisSociety,
                "no collective to open a workplace for",
            )
        })
}

/// `OpenWorkplace`: a coordinator opens a workplace of the collective on a
/// free slot of its kind, spending `founding.materials` from the Common Store.
pub fn open_workplace(
    world: &World,
    envelope: &Envelope<Command>,
    kind: WorkplaceKind,
    wanted: Option<SlotId>,
) -> Result<Vec<Event>, Reject> {
    let actor = office_holder(world, envelope, OfficeKind::Coordinator)?;
    let org = collective(world)?;
    let materials = world.params.founding.materials;
    let stock = crate::ledger::stock_holder(world, org.id);
    let have = crate::ledger::goods_at(world, stock, Good::Materials);
    if have < materials {
        return Err(Reject::new(
            RejectCode::InsufficientGoods,
            format!("a workplace needs {materials} Materials; the Store holds {have}"),
        ));
    }
    let slot = crate::orgs::slot_for(world, kind, wanted)?;
    Ok(vec![Event::WorkplaceOpened {
        workplace: world.next.workplace,
        org: org.id,
        kind,
        slot,
        materials_consumed: materials,
        by: actor.id,
    }])
}

/// `CloseWorkplace`: a coordinator closes a workplace of the collective. Its
/// workers are unassigned first, in id order, then the workplace goes.
pub fn close_workplace(
    world: &World,
    envelope: &Envelope<Command>,
    workplace: WorkplaceId,
) -> Result<Vec<Event>, Reject> {
    let actor = office_holder(world, envelope, OfficeKind::Coordinator)?;
    let wp = world.workplaces.get(&workplace).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownWorkplace,
            format!("no workplace {workplace}"),
        )
    })?;
    let org = world
        .orgs
        .get(&wp.org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {}", wp.org)))?;
    if org.kind != OrgKind::Collective {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            format!("{workplace} is {}'s, not the collective's", org.name),
        ));
    }
    let mut events: Vec<Event> = wp
        .workers
        .keys()
        .map(|citizen| Event::Unassigned {
            workplace,
            citizen: *citizen,
        })
        .collect();
    events.push(Event::WorkplaceClosed {
        workplace,
        org: org.id,
        slot: wp.slot,
        machines_returned: wp.machines,
        by: actor.id,
    });
    Ok(events)
}
