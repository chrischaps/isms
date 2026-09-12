//! Commands (TDD §5.3), the envelope they arrive in, rejections, and `handle`.
//!
//! `handle` never mutates: it validates against the world and the rules and
//! returns the events that would result, or a `Reject`. Capability gating runs
//! first, so a command the constitution does not enable is rejected with
//! `NotInThisSociety` before any other validation (TDD §5.2).

use crate::capabilities::Capabilities;
use crate::event::{Actor, Event};
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, ContractId, OfferId, OrderId, OrgId, SlotId, Tick, WorkplaceId};
use crate::kinds::{CitizenKind, ClientKind, ContractKind, Good, OrgKind, WorkplaceKind};
use crate::ledger::{Asset, Party};
use crate::money::Money;
use crate::policy::Policy;
use crate::rules::Rules;
use crate::world::{
    Allocation, Collateral, Instrument, LeaseAsset, Pay, Price, SaleAsset, Side, StandingPlan,
    World,
};
use serde::{Deserialize, Serialize};

/// What arrives at `handle`: who, for whom, from where, when.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope<C> {
    pub actor: Actor,
    /// A manager acting for an org: asks from org inventory, treasury transfers.
    pub on_behalf_of: Option<OrgId>,
    pub client_kind: ClientKind,
    pub received_at_tick: Tick,
    pub command: C,
}

impl<C> Envelope<C> {
    #[must_use]
    pub fn system(command: C, tick: Tick) -> Self {
        Envelope {
            actor: Actor::System,
            on_behalf_of: None,
            client_kind: ClientKind::Sim,
            received_at_tick: tick,
            command,
        }
    }

    #[must_use]
    pub fn citizen(citizen: CitizenId, command: C, tick: Tick) -> Self {
        Envelope {
            actor: Actor::Citizen(citizen),
            on_behalf_of: None,
            client_kind: ClientKind::Sim,
            received_at_tick: tick,
            command,
        }
    }

    #[must_use]
    pub fn on_behalf_of(mut self, org: OrgId) -> Self {
        self.on_behalf_of = Some(org);
        self
    }

    #[must_use]
    pub fn via(mut self, client_kind: ClientKind) -> Self {
        self.client_kind = client_kind;
        self
    }
}

/// The command catalog (TDD §5.3, Phase 0–1 scope). Later cards implement the
/// arms; until then they are rejected with `NotImplemented`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // citizen
    Join {
        handle: String,
        kind: CitizenKind,
    },
    Seen,
    SetStandingPlan {
        plan: Box<StandingPlan>,
    },
    SetLabor {
        allocations: Vec<Allocation>,
    },
    Transfer {
        to: Party,
        asset: Asset,
        memo: String,
    },
    // market
    PlaceOrder {
        instrument: Instrument,
        side: Side,
        qty: u32,
        limit_price: Money,
        expires_tick: Option<Tick>,
    },
    CancelOrder {
        order: OrderId,
    },
    // orgs
    FoundOrg {
        kind: OrgKind,
        name: String,
        first_workplace: Option<(WorkplaceKind, Option<SlotId>)>,
    },
    AddWorkplace {
        org: OrgId,
        kind: WorkplaceKind,
        slot: Option<SlotId>,
    },
    AppointManager {
        org: OrgId,
        citizen: Option<CitizenId>,
    },
    InstallMachines {
        org: OrgId,
        workplace: WorkplaceId,
        qty: u32,
    },
    UninstallMachines {
        org: OrgId,
        workplace: WorkplaceId,
        qty: u32,
    },
    DeclareDividend {
        org: OrgId,
        per_share: Money,
    },
    IssueShares {
        org: OrgId,
        qty: u64,
    },
    // contracts
    OfferEmployment {
        org: OrgId,
        workplace: WorkplaceId,
        pay: Pay,
        max_hours: u8,
        term_cycles: Option<u32>,
        notice_cycles: u32,
        places: u32,
    },
    AcceptEmployment {
        offer: OfferId,
    },
    TerminateEmployment {
        contract: ContractId,
    },
    OfferSale {
        asset: SaleAsset,
        price: Price,
        to: Option<Party>,
    },
    AcceptSale {
        offer: OfferId,
    },
    CancelSale {
        offer: OfferId,
    },
    PostWanted {
        good: Good,
        qty: u32,
        max_price: Money,
    },
    RemoveWanted {
        offer: OfferId,
    },
    OfferCredit {
        to: Option<Party>,
        principal: Money,
        rate_per_cycle_bp: u32,
        term_cycles: u32,
        collateral: Option<Collateral>,
    },
    AcceptCredit {
        offer: OfferId,
    },
    OfferLease {
        asset: LeaseAsset,
        rent_per_cycle: Money,
        term_cycles: Option<u32>,
    },
    AcceptLease {
        offer: OfferId,
    },
    EndLease {
        contract: ContractId,
    },
    RequestMembership {
        org: OrgId,
    },
    AdmitMember {
        org: OrgId,
        citizen: CitizenId,
    },
    LeaveOrg {
        org: OrgId,
    },
    Pledge {
        hours: Option<u8>,
        goods: Option<(Good, u32)>,
        term_cycles: u32,
    },
    // authority
    SetPolicy {
        policy: Box<Policy>,
    },
    // admin
    EndEpoch {
        reason: String,
    },
}

impl Command {
    /// The variant name, for logs and telemetry.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Command::Join { .. } => "Join",
            Command::Seen => "Seen",
            Command::SetStandingPlan { .. } => "SetStandingPlan",
            Command::SetLabor { .. } => "SetLabor",
            Command::Transfer { .. } => "Transfer",
            Command::PlaceOrder { .. } => "PlaceOrder",
            Command::CancelOrder { .. } => "CancelOrder",
            Command::FoundOrg { .. } => "FoundOrg",
            Command::AddWorkplace { .. } => "AddWorkplace",
            Command::AppointManager { .. } => "AppointManager",
            Command::InstallMachines { .. } => "InstallMachines",
            Command::UninstallMachines { .. } => "UninstallMachines",
            Command::DeclareDividend { .. } => "DeclareDividend",
            Command::IssueShares { .. } => "IssueShares",
            Command::OfferEmployment { .. } => "OfferEmployment",
            Command::AcceptEmployment { .. } => "AcceptEmployment",
            Command::TerminateEmployment { .. } => "TerminateEmployment",
            Command::OfferSale { .. } => "OfferSale",
            Command::AcceptSale { .. } => "AcceptSale",
            Command::CancelSale { .. } => "CancelSale",
            Command::PostWanted { .. } => "PostWanted",
            Command::RemoveWanted { .. } => "RemoveWanted",
            Command::OfferCredit { .. } => "OfferCredit",
            Command::AcceptCredit { .. } => "AcceptCredit",
            Command::OfferLease { .. } => "OfferLease",
            Command::AcceptLease { .. } => "AcceptLease",
            Command::EndLease { .. } => "EndLease",
            Command::RequestMembership { .. } => "RequestMembership",
            Command::AdmitMember { .. } => "AdmitMember",
            Command::LeaveOrg { .. } => "LeaveOrg",
            Command::Pledge { .. } => "Pledge",
            Command::SetPolicy { .. } => "SetPolicy",
            Command::EndEpoch { .. } => "EndEpoch",
        }
    }
}

/// Why a command was rejected. Part of the API contract; tests assert on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectCode {
    /// The constitution does not enable this command here.
    NotInThisSociety,
    /// The engine does not implement this command yet.
    NotImplemented,
    EpochEnded,
    NotAuthorized,
    UnknownCitizen,
    UnknownOrg,
    UnknownWorkplace,
    UnknownOffer,
    UnknownOrder,
    UnknownContract,
    UnknownDwelling,
    NotManager,
    NotOwner,
    NotControllingOwner,
    NotParty,
    Dormant,
    OptionsNarrowed,
    InsufficientFunds,
    InsufficientGoods,
    PantryFull,
    NoSlotAvailable,
    WorkplaceFull,
    TooManyWorkplaces,
    OverBudget,
    NotAssigned,
    OverContractHours,
    InvalidQuantity,
    InvalidPrice,
    InvalidTerm,
    SelfDeal,
    AlreadyExists,
    HandleTaken,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct Reject {
    pub code: RejectCode,
    pub message: String,
}

impl Reject {
    #[must_use]
    pub fn new(code: RejectCode, message: impl Into<String>) -> Self {
        Reject {
            code,
            message: message.into(),
        }
    }
}

impl Capabilities {
    /// Whether the constitution enables this command at all (TDD §5.2).
    #[must_use]
    pub fn allows(&self, cmd: &Command) -> bool {
        match cmd {
            Command::Join { .. }
            | Command::Seen
            | Command::SetStandingPlan { .. }
            | Command::SetLabor { .. }
            | Command::PostWanted { .. }
            | Command::RemoveWanted { .. }
            | Command::RequestMembership { .. }
            | Command::AdmitMember { .. }
            | Command::LeaveOrg { .. }
            | Command::EndEpoch { .. }
            | Command::SetPolicy { .. }
            | Command::AddWorkplace { .. }
            | Command::AppointManager { .. }
            | Command::InstallMachines { .. }
            | Command::UninstallMachines { .. }
            // Transfers are a primitive (GDD 7.3); only money needs money.
            | Command::Transfer {
                asset: Asset::Good(..),
                ..
            } => true,
            Command::Transfer {
                asset: Asset::Money(_),
                ..
            } => self.money,
            Command::PlaceOrder { .. } | Command::CancelOrder { .. } => self.order_books,
            Command::FoundOrg { kind, .. } => self.allows_org(*kind),
            Command::DeclareDividend { .. } | Command::IssueShares { .. } => {
                self.allows_contract(ContractKind::Share)
            }
            Command::OfferEmployment { .. }
            | Command::AcceptEmployment { .. }
            | Command::TerminateEmployment { .. } => self.allows_contract(ContractKind::Employment),
            Command::OfferSale {
                price: Price::Money(_),
                ..
            } => self.money && self.allows_contract(ContractKind::SaleDirect),
            Command::OfferSale {
                price: Price::Good(..),
                ..
            }
            | Command::AcceptSale { .. }
            | Command::CancelSale { .. } => self.allows_contract(ContractKind::SaleDirect),
            Command::OfferCredit { .. } | Command::AcceptCredit { .. } => {
                self.allows_contract(ContractKind::Credit)
                    || self.allows_contract(ContractKind::PublicCredit)
            }
            Command::OfferLease { .. } | Command::AcceptLease { .. } | Command::EndLease { .. } => {
                self.allows_contract(ContractKind::Lease)
            }
            Command::Pledge { .. } => self.allows_contract(ContractKind::Pledge),
        }
    }
}

/// Validate and execute a command against the world. Never mutates.
pub fn handle(
    world: &World,
    rules: &Rules,
    envelope: &Envelope<Command>,
) -> Result<Vec<Event>, Reject> {
    if world.meta.epoch_ended.is_some() {
        return Err(Reject::new(RejectCode::EpochEnded, "the epoch has ended"));
    }
    if !rules.capabilities.allows(&envelope.command) {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} does not exist in this society", envelope.command.kind()),
        ));
    }
    match &envelope.command {
        Command::Join { handle, kind } => join(world, envelope, handle, *kind),
        Command::Seen => seen(world, envelope),
        Command::SetLabor { allocations } => crate::labor::set_labor(world, envelope, allocations),
        Command::Transfer { to, asset, memo } => {
            crate::transfers::transfer(world, envelope, *to, *asset, memo)
        }
        Command::OfferSale { asset, price, to } => {
            crate::transfers::offer_sale(world, envelope, *asset, *price, *to)
        }
        Command::AcceptSale { offer } => crate::transfers::accept_sale(world, envelope, *offer),
        Command::CancelSale { offer } => crate::transfers::cancel_sale(world, envelope, *offer),
        Command::PostWanted {
            good,
            qty,
            max_price,
        } => crate::transfers::post_wanted(world, envelope, *good, *qty, *max_price),
        Command::RemoveWanted { offer } => crate::transfers::remove_wanted(world, envelope, *offer),
        Command::FoundOrg {
            kind,
            name,
            first_workplace,
        } => crate::orgs::found_org(world, envelope, *kind, name, *first_workplace),
        Command::AddWorkplace { org, kind, slot } => {
            crate::orgs::add_workplace(world, envelope, *org, *kind, *slot)
        }
        Command::AppointManager { org, citizen } => {
            crate::orgs::appoint_manager(world, envelope, *org, *citizen)
        }
        Command::InstallMachines {
            org,
            workplace,
            qty,
        } => crate::orgs::install_machines(world, envelope, *org, *workplace, *qty),
        Command::UninstallMachines {
            org,
            workplace,
            qty,
        } => crate::orgs::uninstall_machines(world, envelope, *org, *workplace, *qty),
        Command::EndEpoch { reason: _ } => end_epoch(world, envelope),
        other => Err(Reject::new(
            RejectCode::NotImplemented,
            format!("{} is not implemented yet", other.kind()),
        )),
    }
}

/// The citizen an envelope acts as, or a rejection when the actor is missing,
/// dormant, or `System` where a citizen is required.
pub fn acting_citizen<'w>(
    world: &'w World,
    envelope: &Envelope<Command>,
) -> Result<&'w crate::world::Citizen, Reject> {
    let Actor::Citizen(id) = envelope.actor else {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "a citizen must issue this command",
        ));
    };
    let citizen = world
        .citizens
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownCitizen, format!("no citizen {id}")))?;
    if citizen.dormant {
        return Err(Reject::new(RejectCode::Dormant, format!("{id} is dormant")));
    }
    Ok(citizen)
}

fn join(
    world: &World,
    envelope: &Envelope<Command>,
    handle: &str,
    kind: CitizenKind,
) -> Result<Vec<Event>, Reject> {
    if envelope.actor != Actor::System {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "Join is issued by the system",
        ));
    }
    if handle.trim().is_empty() {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "handle must not be empty",
        ));
    }
    if world.citizens.values().any(|c| c.handle == handle) {
        return Err(Reject::new(
            RejectCode::HandleTaken,
            format!("handle {handle} is taken"),
        ));
    }
    let citizen = world.next.citizen;
    let (endowment, explain) = if world.constitution.has_money() {
        let amount = world.params.money.endowment;
        (
            amount,
            Some(Explain::new(RuleId::Endowment, "endowment", amount).input("endowment", amount)),
        )
    } else {
        (Money::ZERO, None)
    };
    // Dwelling assignment in collective systems arrives with S0.15.
    let dwelling = None;
    Ok(vec![match kind {
        CitizenKind::Human => Event::CitizenJoined {
            citizen,
            handle: handle.to_owned(),
            kind,
            endowment,
            dwelling,
            explain,
        },
        CitizenKind::Householder => Event::HouseholderJoined {
            citizen,
            handle: handle.to_owned(),
            endowment,
            dwelling,
            explain,
        },
    }])
}

fn seen(world: &World, envelope: &Envelope<Command>) -> Result<Vec<Event>, Reject> {
    let Actor::Citizen(id) = envelope.actor else {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "Seen needs a citizen",
        ));
    };
    let citizen = world
        .citizens
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownCitizen, format!("no citizen {id}")))?;
    // Throttled to once per tick per citizen; a repeat is a no-op, not an error.
    if citizen.last_seen_tick == envelope.received_at_tick && !citizen.dormant {
        return Ok(Vec::new());
    }
    Ok(vec![Event::CitizenSeen {
        citizen: id,
        tick: envelope.received_at_tick,
        client_kind: envelope.client_kind,
    }])
}

fn end_epoch(world: &World, envelope: &Envelope<Command>) -> Result<Vec<Event>, Reject> {
    if envelope.actor != Actor::System {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "only the operator ends an epoch",
        ));
    }
    let cycle = world.cycle_of(world.meta.tick.saturating_sub(1));
    Ok(vec![Event::EpochEnded {
        reason: crate::world::EpochEndReason::Operator,
        cycle,
    }])
}
