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
use std::collections::BTreeMap;

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
    /// Q109 (S1.15): whoever posted an open employment, credit, lease, sale or
    /// wanted offer may take it back; accepted contracts are untouched. Sale
    /// and wanted offers keep their own events (`SaleCancelled`, `WantedRemoved`).
    WithdrawOffer {
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
    /// An owner occupies their own dwelling (Q34).
    MoveIn {
        dwelling: crate::ids::DwellingId,
    },
    MoveOut {
        dwelling: crate::ids::DwellingId,
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
        /// The org pledged to; `None` pledges to the assembly (Q57).
        to: Option<OrgId>,
    },
    // authority
    SetPolicy {
        policy: Box<Policy>,
    },
    // admin
    EndEpoch {
        reason: String,
    },
    // --- Phase 0b (appended, Q50) ---------------------------------------------
    /// Draw from the Common Store this tick, within the need entitlement (S0.15).
    RequestStoreDraw {
        good: Good,
        qty: u32,
    },
    /// Take or give up a position without a contract (norm systems, S0.15c).
    JoinWorkplace {
        workplace: WorkplaceId,
    },
    LeaveWorkplace {
        workplace: WorkplaceId,
    },
    /// Buy from the state store at the list price this tick (S0.16a).
    RequestStateStore {
        good: Good,
        qty: u32,
    },
    /// The Committee publishes the Plan (S0.16b): targets per workplace and,
    /// optionally, the policy parts it also sets.
    SetPlan {
        targets: BTreeMap<WorkplaceId, f64>,
        materials_split: Option<crate::policy::MaterialsSplit>,
        price_list: Option<BTreeMap<Good, Money>>,
        wage_grades: Option<Vec<Money>>,
        ration_caps: Option<BTreeMap<Good, u32>>,
    },
    /// Ask the Committee for a different workplace (assigned-labor systems).
    RequestTransfer {
        to_workplace: WorkplaceId,
    },
    DecideTransfer {
        citizen: CitizenId,
        approve: bool,
    },
    /// A cooperative decides how it shares its surplus (S0.17b).
    SetShareRule {
        org: OrgId,
        rule: crate::world::ShareRule,
    },
    /// A cooperative applies to the Public Investment Bank (S0.17c).
    RequestBankLoan {
        org: OrgId,
        principal: Money,
        term_cycles: u32,
    },
    /// A member proposes a candidate; the members vote (S0.17c).
    ProposeAdmission {
        org: OrgId,
        citizen: CitizenId,
    },
    VoteAdmission {
        proposal: crate::ids::ProposalId,
        approve: bool,
    },
    /// Unions (S0.17d): an employee founds the firm's union and is its steward.
    FormUnion {
        firm: OrgId,
        name: String,
    },
    OfferCollectiveAgreement {
        union: OrgId,
        wage_floor: Money,
        hours: u8,
        term_cycles: u32,
    },
    AcceptCollectiveAgreement {
        offer: OfferId,
    },
    CallStrike {
        union: OrgId,
        cycles: u32,
    },
    // --- Phase 2 (appended) ---------------------------------------------------
    /// Open a proposal before the assembly (S2.1; GDD 8.1).
    Propose {
        title: String,
        text: String,
        kind: crate::world::ProposalKind,
    },
    /// Cast or replace a ballot on an open proposal (S2.1).
    Vote {
        proposal: crate::ids::ProposalId,
        ballot: crate::world::Ballot,
    },
    /// Stand in the open election for `office` (S2.2).
    Stand {
        office: crate::constitution::OfficeKind,
    },
    /// Withdraw a candidacy from the open election for `office` (S2.2).
    Withdraw {
        office: crate::constitution::OfficeKind,
    },
    /// Cast or replace an approval ballot in the open election for `office`:
    /// any subset of its candidates (S2.2, Q118).
    Approve {
        office: crate::constitution::OfficeKind,
        candidates: std::collections::BTreeSet<crate::ids::CitizenId>,
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
            Command::WithdrawOffer { .. } => "WithdrawOffer",
            Command::OfferCredit { .. } => "OfferCredit",
            Command::AcceptCredit { .. } => "AcceptCredit",
            Command::OfferLease { .. } => "OfferLease",
            Command::AcceptLease { .. } => "AcceptLease",
            Command::EndLease { .. } => "EndLease",
            Command::MoveIn { .. } => "MoveIn",
            Command::MoveOut { .. } => "MoveOut",
            Command::RequestMembership { .. } => "RequestMembership",
            Command::AdmitMember { .. } => "AdmitMember",
            Command::LeaveOrg { .. } => "LeaveOrg",
            Command::Pledge { .. } => "Pledge",
            Command::SetPolicy { .. } => "SetPolicy",
            Command::EndEpoch { .. } => "EndEpoch",
            Command::RequestStoreDraw { .. } => "RequestStoreDraw",
            Command::JoinWorkplace { .. } => "JoinWorkplace",
            Command::LeaveWorkplace { .. } => "LeaveWorkplace",
            Command::RequestStateStore { .. } => "RequestStateStore",
            Command::SetPlan { .. } => "SetPlan",
            Command::RequestTransfer { .. } => "RequestTransfer",
            Command::DecideTransfer { .. } => "DecideTransfer",
            Command::SetShareRule { .. } => "SetShareRule",
            Command::RequestBankLoan { .. } => "RequestBankLoan",
            Command::ProposeAdmission { .. } => "ProposeAdmission",
            Command::VoteAdmission { .. } => "VoteAdmission",
            Command::FormUnion { .. } => "FormUnion",
            Command::OfferCollectiveAgreement { .. } => "OfferCollectiveAgreement",
            Command::AcceptCollectiveAgreement { .. } => "AcceptCollectiveAgreement",
            Command::CallStrike { .. } => "CallStrike",
            Command::Propose { .. } => "Propose",
            Command::Vote { .. } => "Vote",
            Command::Stand { .. } => "Stand",
            Command::Withdraw { .. } => "Withdraw",
            Command::Approve { .. } => "Approve",
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
    // Phase 0b (appended)
    /// The society has no Common Store.
    NoStore,
    /// A store draw above the citizen's need entitlement.
    OverEntitlement,
    /// The state store does not carry this good.
    NotOnPriceList,
    /// No transfer request is pending for the citizen.
    NoRequestPending,
    /// An employment offer below the society's wage floor.
    BelowMinimumWage,
    // Phase 2 (appended)
    /// The citizen already holds `governance.open_proposals_per_citizen` open proposals.
    TooManyProposals,
    /// No open proposal with this id.
    UnknownProposal,
    /// No election is open for this office (S2.2).
    NoElection,
    /// The citizen is not a candidate in this election (S2.2).
    NotACandidate,
    /// The office bars consecutive terms and this citizen's just ended (S2.2).
    ConsecutiveTerm,
    /// The citizen does not hold the office the command needs (S2.2).
    NotAnOfficeHolder,
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
            | Command::WithdrawOffer { .. }
            | Command::RequestMembership { .. }
            | Command::AdmitMember { .. }
            | Command::LeaveOrg { .. }
            | Command::EndEpoch { .. }
            | Command::MoveIn { .. }
            | Command::MoveOut { .. }
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
            }
            Command::RequestBankLoan { .. } => self.allows_contract(ContractKind::PublicCredit),
            Command::ProposeAdmission { .. } | Command::VoteAdmission { .. } => {
                self.allows_org(OrgKind::Cooperative)
            }
            Command::FormUnion { .. } | Command::CallStrike { .. } => self.allows_org(OrgKind::Union),
            Command::OfferCollectiveAgreement { .. } | Command::AcceptCollectiveAgreement { .. } => {
                self.allows_contract(ContractKind::CollectiveAgreement)
            }
            Command::OfferLease { .. } | Command::AcceptLease { .. } | Command::EndLease { .. } => {
                self.allows_contract(ContractKind::Lease)
            }
            Command::Pledge { .. } => self.allows_contract(ContractKind::Pledge),
            Command::RequestStoreDraw { .. } => self.common_store,
            Command::JoinWorkplace { .. } | Command::LeaveWorkplace { .. } => {
                self.labor == crate::constitution::LaborMode::Norm
            }
            Command::RequestStateStore { .. } | Command::SetPlan { .. } => {
                self.administered_prices
            }
            Command::RequestTransfer { .. } | Command::DecideTransfer { .. } => {
                self.labor == crate::constitution::LaborMode::Assigned
            }
            Command::SetShareRule { .. } => self.allows_org(OrgKind::Cooperative),
            Command::Propose { kind, .. } => self.proposal_kinds.contains(&kind.tag()),
            Command::Vote { .. } => {
                !self.proposal_kinds.is_empty() || self.allows_org(OrgKind::Cooperative)
            }
            Command::Stand { office } | Command::Withdraw { office } | Command::Approve { office, .. } => {
                self.offices.iter().any(|o| o.kind == *office)
            }
        }
    }
}

/// Validate and execute a command against the world. Never mutates.
#[allow(clippy::too_many_lines)] // one arm per command
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
        Command::SetStandingPlan { plan } => crate::plan::set_standing_plan(world, envelope, plan),
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
        Command::WithdrawOffer { offer } => {
            crate::transfers::withdraw_offer(world, envelope, *offer)
        }
        Command::PlaceOrder {
            instrument,
            side,
            qty,
            limit_price,
            expires_tick,
        } => crate::market::place_order(
            world,
            envelope,
            *instrument,
            *side,
            *qty,
            *limit_price,
            *expires_tick,
        ),
        Command::CancelOrder { order } => crate::market::cancel_order(world, envelope, *order),
        Command::OfferEmployment {
            org,
            workplace,
            pay,
            max_hours,
            term_cycles,
            notice_cycles,
            places,
        } => crate::employment::offer_employment(
            world,
            envelope,
            *org,
            *workplace,
            *pay,
            *max_hours,
            *term_cycles,
            *notice_cycles,
            *places,
        ),
        Command::AcceptEmployment { offer } => {
            crate::employment::accept_employment(world, envelope, *offer)
        }
        Command::TerminateEmployment { contract } => {
            crate::employment::terminate_employment(world, envelope, *contract)
        }
        Command::IssueShares { org, qty } => {
            crate::shares::issue_shares(world, envelope, *org, *qty)
        }
        Command::DeclareDividend { org, per_share } => {
            crate::shares::declare_dividend(world, envelope, *org, *per_share)
        }
        Command::OfferLease {
            asset,
            rent_per_cycle,
            term_cycles,
        } => crate::housing::offer_lease(world, envelope, *asset, *rent_per_cycle, *term_cycles),
        Command::AcceptLease { offer } => crate::housing::accept_lease(world, envelope, *offer),
        Command::EndLease { contract } => crate::housing::end_lease(world, envelope, *contract),
        Command::MoveIn { dwelling } => crate::housing::move_in(world, envelope, *dwelling),
        Command::MoveOut { dwelling } => crate::housing::move_out(world, envelope, *dwelling),
        Command::OfferCredit {
            to,
            principal,
            rate_per_cycle_bp,
            term_cycles,
            collateral,
        } => crate::credit::offer_credit(
            world,
            envelope,
            *to,
            *principal,
            *rate_per_cycle_bp,
            *term_cycles,
            *collateral,
        ),
        Command::AcceptCredit { offer } => crate::credit::accept_credit(world, envelope, *offer),
        Command::RequestMembership { org } => {
            crate::credit::request_membership(world, envelope, *org)
        }
        Command::AdmitMember { org, citizen } => {
            crate::coop::admit(world, envelope, *org, *citizen)
                .or_else(|| crate::union::admit(world, envelope, *org, *citizen))
                .unwrap_or_else(|| crate::credit::admit_member(world, envelope, *org, *citizen))
        }
        Command::LeaveOrg { org } => crate::credit::leave_org(world, envelope, *org),
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
        Command::RequestStoreDraw { good, qty } => {
            crate::store::request_store_draw(world, envelope, *good, *qty)
        }
        Command::Pledge {
            hours,
            goods,
            term_cycles,
            to,
        } => crate::norms::pledge(world, envelope, *hours, *goods, *term_cycles, *to),
        Command::SetPolicy { policy } => set_policy(world, envelope, policy),
        Command::JoinWorkplace { workplace } => {
            crate::norms::join_workplace(world, envelope, *workplace)
        }
        Command::LeaveWorkplace { workplace } => {
            crate::norms::leave_workplace(world, envelope, *workplace)
        }
        Command::RequestStateStore { good, qty } => {
            crate::state_store::request_state_store(world, envelope, *good, *qty)
        }
        Command::SetPlan {
            targets,
            materials_split,
            price_list,
            wage_grades,
            ration_caps,
        } => crate::planning::set_plan(
            world,
            envelope,
            targets,
            *materials_split,
            price_list.as_ref(),
            wage_grades.as_deref(),
            ration_caps.as_ref(),
        ),
        Command::RequestTransfer { to_workplace } => {
            crate::planning::request_transfer(world, envelope, *to_workplace)
        }
        Command::DecideTransfer { citizen, approve } => {
            crate::planning::decide_transfer(world, envelope, *citizen, *approve)
        }
        Command::SetShareRule { org, rule } => {
            crate::coop::set_share_rule(world, envelope, *org, *rule)
        }
        Command::RequestBankLoan {
            org,
            principal,
            term_cycles,
        } => crate::bank::request_bank_loan(world, envelope, *org, *principal, *term_cycles),
        Command::ProposeAdmission { org, citizen } => {
            crate::bank::propose_admission(world, envelope, *org, *citizen)
        }
        Command::VoteAdmission { proposal, approve } => {
            let ballot = if *approve {
                crate::world::Ballot::Yes
            } else {
                crate::world::Ballot::No
            };
            crate::bank::vote_admission(world, envelope, *proposal, ballot)
        }
        Command::FormUnion { firm, name } => crate::union::form_union(world, envelope, *firm, name),
        Command::OfferCollectiveAgreement {
            union,
            wage_floor,
            hours,
            term_cycles,
        } => crate::union::offer_agreement(
            world,
            envelope,
            *union,
            *wage_floor,
            *hours,
            *term_cycles,
        ),
        Command::AcceptCollectiveAgreement { offer } => {
            crate::union::accept_agreement(world, envelope, *offer)
        }
        Command::CallStrike { union, cycles } => {
            crate::union::call_strike(world, envelope, *union, *cycles)
        }
        Command::Propose { title, text, kind } => {
            crate::governance::propose(world, envelope, title, text, *kind)
        }
        Command::Vote { proposal, ballot } => {
            crate::governance::vote(world, envelope, *proposal, *ballot)
        }
        Command::Stand { office } => crate::offices::stand(world, envelope, *office),
        Command::Withdraw { office } => crate::offices::withdraw(world, envelope, *office),
        Command::Approve { office, candidates } => {
            crate::offices::approve(world, envelope, *office, candidates)
        }
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
    // Collective systems assign a dwelling from the society's stock at join (GDD 6.2).
    let dwelling = crate::housing::free_society_dwelling(world);
    // Assigned-labor systems place the joiner by the balancing rule (GDD 6.3, Q62).
    let assignment = (world.constitution.labor == crate::constitution::LaborMode::Assigned)
        .then(|| crate::orgs::least_staffed(world))
        .flatten()
        .map(|workplace| Event::Assigned {
            workplace,
            citizen,
            contract: None,
        });
    let mut events = vec![match kind {
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
    }];
    events.extend(assignment);
    Ok(events)
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
    let mut events = vec![Event::CitizenSeen {
        citizen: id,
        tick: envelope.received_at_tick,
        client_kind: envelope.client_kind,
    }];
    if citizen.dormant {
        events.push(Event::CitizenReturned { citizen: id });
    }
    Ok(events)
}

/// `SetPolicy`: a full replacement of the society's policy by `System` (the
/// sim's stand-in for the assembly, committee or legislature until Phase 2
/// governance lands; Q56), validated against the constitution.
fn set_policy(
    world: &World,
    envelope: &Envelope<Command>,
    policy: &Policy,
) -> Result<Vec<Event>, Reject> {
    if envelope.actor != Actor::System {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "policy is set by the society's authority (System in Phase 0)",
        ));
    }
    policy
        .validate_against(&world.constitution)
        .map_err(|reason| Reject::new(RejectCode::NotInThisSociety, reason))?;
    Ok(vec![Event::PolicyChanged {
        policy: Box::new(policy.clone()),
        by: envelope.actor,
        proposal: None,
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
    // An operator ends the epoch mid-cycle, so the summary carries the running
    // cycle's figures as they stand rather than a closed cycle's.
    let aggregates = crate::metrics::aggregates(world, world.meta.low_population_cycles);
    let summary = crate::metrics::epoch_summary(world, aggregates);
    Ok(vec![Event::EpochEnded {
        reason: crate::world::EpochEndReason::Operator,
        cycle,
        summary,
    }])
}
