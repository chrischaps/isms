//! The event catalog (TDD §5.4). Discrete events record things that happened;
//! `TickResolved` carries the continuous bookkeeping and never a movement of
//! money or goods between holders (D4). Externally tagged so postcard and JSON
//! both work.

use crate::config::Preset;
use crate::explain::Explain;
use crate::ids::{
    CitizenId, ContractId, Cycle, DwellingId, Epoch, OfferId, OrderId, OrgId, Tick, WorkplaceId,
};
use crate::kinds::{CitizenKind, ClientKind, Good, JobFamily, OrgKind, WorkplaceKind};
use crate::ledger::{Asset, Holder, Party};
use crate::money::Money;
use crate::policy::Policy;
use crate::world::{
    Allocation, Collateral, EpochEndReason, Instrument, LeaseAsset, Needs, OfferBody, Order,
    Ownership, Pay, Price, SaleAsset, Skill, StandingPlan,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Who caused a command; `System` for the engine, the operator, and the sim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    System,
    Citizen(CitizenId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    // --- society and epochs -------------------------------------------------
    SocietyCreated {
        society_id: u64,
        seed: u64,
        preset: Box<Preset>,
    },
    EpochStarted {
        epoch: Epoch,
    },
    /// Stock that did not come from production (ADR-0004). Only `start_epoch` emits it.
    Seeded {
        holder: Holder,
        asset: Asset,
    },
    EpochEnded {
        reason: EpochEndReason,
        cycle: Cycle,
    },

    // --- citizens -----------------------------------------------------------
    CitizenJoined {
        citizen: CitizenId,
        handle: String,
        kind: CitizenKind,
        endowment: Money,
        dwelling: Option<DwellingId>,
        explain: Option<Explain>,
    },
    CitizenSeen {
        citizen: CitizenId,
        tick: Tick,
        client_kind: ClientKind,
    },
    CitizenDormant {
        citizen: CitizenId,
    },
    CitizenReturned {
        citizen: CitizenId,
    },
    HouseholderJoined {
        citizen: CitizenId,
        handle: String,
        endowment: Money,
        dwelling: Option<DwellingId>,
        explain: Option<Explain>,
    },
    HouseholderEmigrated {
        citizen: CitizenId,
        burned_money: Money,
        burned_goods: BTreeMap<Good, u32>,
        explain: Explain,
    },
    PlanChanged {
        citizen: CitizenId,
        plan: Box<StandingPlan>,
    },
    LaborSet {
        citizen: CitizenId,
        allocations: Vec<Allocation>,
    },
    Transferred {
        from: Party,
        to: Party,
        asset: Asset,
        memo: String,
    },

    // --- markets ------------------------------------------------------------
    OrderPlaced {
        order: Order,
        escrow: Asset,
    },
    OrderCancelled {
        order: OrderId,
        released: Asset,
    },
    OrderExpired {
        order: OrderId,
        released: Asset,
    },
    Trade {
        instrument: Instrument,
        buyer: Party,
        seller: Party,
        buy_order: OrderId,
        sell_order: OrderId,
        qty: u32,
        price: Money,
        tick: Tick,
    },

    // --- direct sales and ads ----------------------------------------------
    SaleOffered {
        offer: OfferId,
        by: Party,
        asset: SaleAsset,
        price: Price,
        to: Option<Party>,
    },
    SaleAccepted {
        offer: OfferId,
        buyer: Party,
        seller: Party,
        asset: SaleAsset,
        price: Price,
    },
    SaleCancelled {
        offer: OfferId,
    },
    WantedPosted {
        offer: OfferId,
        by: Party,
        good: Good,
        qty: u32,
        max_price: Money,
    },
    WantedRemoved {
        offer: OfferId,
    },

    // --- orgs ---------------------------------------------------------------
    OrgFounded {
        org: OrgId,
        kind: OrgKind,
        name: String,
        founder: Option<CitizenId>,
        ownership: Ownership,
        manager: Option<CitizenId>,
        fee_burned: Money,
    },
    WorkplaceAdded {
        workplace: WorkplaceId,
        org: OrgId,
        kind: WorkplaceKind,
        slot: Option<crate::ids::SlotId>,
        materials_consumed: u32,
    },
    /// A citizen holds a position at a workplace (by contract, or by assignment
    /// in assigned-labor systems). `SetLabor` requires it (Q16).
    Assigned {
        workplace: WorkplaceId,
        citizen: CitizenId,
        contract: Option<ContractId>,
    },
    Unassigned {
        workplace: WorkplaceId,
        citizen: CitizenId,
    },
    ManagerAppointed {
        org: OrgId,
        citizen: Option<CitizenId>,
    },
    MemberAdmitted {
        org: OrgId,
        citizen: CitizenId,
    },
    MemberLeft {
        org: OrgId,
        citizen: CitizenId,
    },
    SharesIssued {
        org: OrgId,
        qty: u64,
    },
    SharesTransferred {
        org: OrgId,
        from: crate::world::ShareHolder,
        to: crate::world::ShareHolder,
        qty: u64,
    },
    DividendDeclared {
        org: OrgId,
        per_share: Money,
        cycle: Cycle,
    },
    DividendPaid {
        org: OrgId,
        citizen: CitizenId,
        amount: Money,
        explain: Explain,
    },
    MachinesInstalled {
        workplace: WorkplaceId,
        qty: u32,
    },
    MachinesUninstalled {
        workplace: WorkplaceId,
        qty: u32,
    },
    MachinesDepreciated {
        workplace: WorkplaceId,
        qty: u32,
        explain: Explain,
    },
    DwellingBuilt {
        dwelling: DwellingId,
        org: OrgId,
        workplace: Option<WorkplaceId>,
    },
    DwellingTransferred {
        dwelling: DwellingId,
        to: crate::world::Owner,
    },
    DwellingOccupied {
        dwelling: DwellingId,
        citizen: Option<CitizenId>,
    },

    // --- contracts ----------------------------------------------------------
    EmploymentOffered {
        offer: OfferId,
        body: OfferBody,
    },
    EmploymentAccepted {
        contract: ContractId,
        offer: OfferId,
        org: OrgId,
        workplace: WorkplaceId,
        citizen: CitizenId,
        pay: Pay,
        max_hours: u8,
        term_cycles: Option<u32>,
        notice_cycles: u32,
    },
    EmploymentTerminated {
        contract: ContractId,
        by: Party,
        notice_pay: Money,
        forfeited: Money,
    },
    CreditOffered {
        offer: OfferId,
        body: OfferBody,
    },
    CreditAccepted {
        contract: ContractId,
        lender: Party,
        borrower: Party,
        principal: Money,
        installment: Money,
        installments: u32,
        collateral: Option<Collateral>,
    },
    CreditInstallment {
        contract: ContractId,
        amount: Money,
        remaining: u32,
        explain: Explain,
    },
    CreditRepaid {
        contract: ContractId,
    },
    CreditDefaulted {
        contract: ContractId,
        collateral_seized: Option<Collateral>,
    },
    LeaseOffered {
        offer: OfferId,
        body: OfferBody,
    },
    LeaseAccepted {
        contract: ContractId,
        owner: Party,
        tenant: Party,
        asset: LeaseAsset,
        rent_per_cycle: Money,
        term_cycles: Option<u32>,
    },
    RentPaid {
        contract: ContractId,
        amount: Money,
        explain: Explain,
    },
    RentMissed {
        contract: ContractId,
        owed: Money,
    },
    LeaseEnded {
        contract: ContractId,
        evicted: bool,
    },
    Paid {
        citizen: CitizenId,
        org: OrgId,
        contract: Option<ContractId>,
        amount: Money,
        explain: Explain,
    },
    PaymentMissed {
        citizen: CitizenId,
        org: OrgId,
        contract: ContractId,
        owed: Money,
        paid: Money,
    },

    // --- production and needs -----------------------------------------------
    Produced {
        workplace: WorkplaceId,
        tick: Tick,
        output: Good,
        units: u32,
        inputs_consumed: BTreeMap<Good, u32>,
        per_worker: Vec<WorkerOutput>,
    },
    Drew {
        citizen: CitizenId,
        goods: BTreeMap<Good, u32>,
        explain: Explain,
    },
    HardshipBegan {
        citizen: CitizenId,
        cycle: Cycle,
    },
    HardshipEnded {
        citizen: CitizenId,
        cycle: Cycle,
    },
    DestitutionBegan {
        citizen: CitizenId,
        cycle: Cycle,
    },
    DestitutionEnded {
        citizen: CitizenId,
        cycle: Cycle,
    },

    // --- authority ----------------------------------------------------------
    PolicyChanged {
        policy: Box<Policy>,
        by: Actor,
    },

    // --- the tick -----------------------------------------------------------
    TickResolved {
        tick: Tick,
        cycle: Cycle,
        price_index: Option<f64>,
        citizen_deltas: Vec<CitizenDelta>,
        workplace_deltas: Vec<WorkplaceDelta>,
    },
    CycleClosed {
        cycle: Cycle,
        aggregates: CycleAggregates,
        /// The collapse counter after this cycle (phase 8m).
        low_population_cycles: u32,
    },
}

/// Per-worker attribution inside `Produced` (TDD §5.4). `true_output` is only
/// ever shown to the worker; the API filters it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerOutput {
    pub citizen: CitizenId,
    pub tick_hours: u32,
    pub true_output: f64,
    pub attributed_output: f64,
    pub explain: Explain,
}

/// Continuous per-citizen bookkeeping carried by `TickResolved`. Values are the
/// citizen's new state after the tick; goods eaten or consumed leave the pantry
/// for the `consumed` sink, never for another holder (D4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CitizenDelta {
    pub citizen: CitizenId,
    pub needs: Needs,
    pub food_eaten: u32,
    pub wares_consumed: u32,
    pub output_mult: f64,
    pub budget: u8,
    pub fatigue_debt: u8,
    pub consecutive_high_effort_cycles: u8,
    pub skill: BTreeMap<JobFamily, Skill>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkplaceDelta {
    pub workplace: WorkplaceId,
    pub machine_wear: f64,
    pub output_remainder: f64,
    pub cycle_output: f64,
    /// This cycle's accumulators per assigned worker.
    pub workers: BTreeMap<CitizenId, WorkerCycle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerCycle {
    pub tick_hours: u32,
    pub attributed: f64,
}

/// Per-cycle metrics snapshot (TDD §13). Filled in by S0.13.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CycleAggregates {
    pub population: u32,
    pub active_humans: u32,
    pub householders: u32,
}

impl Event {
    /// The variant name, for registries and logs.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Event::SocietyCreated { .. } => "SocietyCreated",
            Event::EpochStarted { .. } => "EpochStarted",
            Event::Seeded { .. } => "Seeded",
            Event::EpochEnded { .. } => "EpochEnded",
            Event::CitizenJoined { .. } => "CitizenJoined",
            Event::CitizenSeen { .. } => "CitizenSeen",
            Event::CitizenDormant { .. } => "CitizenDormant",
            Event::CitizenReturned { .. } => "CitizenReturned",
            Event::HouseholderJoined { .. } => "HouseholderJoined",
            Event::HouseholderEmigrated { .. } => "HouseholderEmigrated",
            Event::PlanChanged { .. } => "PlanChanged",
            Event::LaborSet { .. } => "LaborSet",
            Event::Transferred { .. } => "Transferred",
            Event::OrderPlaced { .. } => "OrderPlaced",
            Event::OrderCancelled { .. } => "OrderCancelled",
            Event::OrderExpired { .. } => "OrderExpired",
            Event::Trade { .. } => "Trade",
            Event::SaleOffered { .. } => "SaleOffered",
            Event::SaleAccepted { .. } => "SaleAccepted",
            Event::SaleCancelled { .. } => "SaleCancelled",
            Event::WantedPosted { .. } => "WantedPosted",
            Event::WantedRemoved { .. } => "WantedRemoved",
            Event::OrgFounded { .. } => "OrgFounded",
            Event::WorkplaceAdded { .. } => "WorkplaceAdded",
            Event::Assigned { .. } => "Assigned",
            Event::Unassigned { .. } => "Unassigned",
            Event::ManagerAppointed { .. } => "ManagerAppointed",
            Event::MemberAdmitted { .. } => "MemberAdmitted",
            Event::MemberLeft { .. } => "MemberLeft",
            Event::SharesIssued { .. } => "SharesIssued",
            Event::SharesTransferred { .. } => "SharesTransferred",
            Event::DividendDeclared { .. } => "DividendDeclared",
            Event::DividendPaid { .. } => "DividendPaid",
            Event::MachinesInstalled { .. } => "MachinesInstalled",
            Event::MachinesUninstalled { .. } => "MachinesUninstalled",
            Event::MachinesDepreciated { .. } => "MachinesDepreciated",
            Event::DwellingBuilt { .. } => "DwellingBuilt",
            Event::DwellingTransferred { .. } => "DwellingTransferred",
            Event::DwellingOccupied { .. } => "DwellingOccupied",
            Event::EmploymentOffered { .. } => "EmploymentOffered",
            Event::EmploymentAccepted { .. } => "EmploymentAccepted",
            Event::EmploymentTerminated { .. } => "EmploymentTerminated",
            Event::CreditOffered { .. } => "CreditOffered",
            Event::CreditAccepted { .. } => "CreditAccepted",
            Event::CreditInstallment { .. } => "CreditInstallment",
            Event::CreditRepaid { .. } => "CreditRepaid",
            Event::CreditDefaulted { .. } => "CreditDefaulted",
            Event::LeaseOffered { .. } => "LeaseOffered",
            Event::LeaseAccepted { .. } => "LeaseAccepted",
            Event::RentPaid { .. } => "RentPaid",
            Event::RentMissed { .. } => "RentMissed",
            Event::LeaseEnded { .. } => "LeaseEnded",
            Event::Paid { .. } => "Paid",
            Event::PaymentMissed { .. } => "PaymentMissed",
            Event::Produced { .. } => "Produced",
            Event::Drew { .. } => "Drew",
            Event::HardshipBegan { .. } => "HardshipBegan",
            Event::HardshipEnded { .. } => "HardshipEnded",
            Event::DestitutionBegan { .. } => "DestitutionBegan",
            Event::DestitutionEnded { .. } => "DestitutionEnded",
            Event::PolicyChanged { .. } => "PolicyChanged",
            Event::TickResolved { .. } => "TickResolved",
            Event::CycleClosed { .. } => "CycleClosed",
        }
    }

    /// Every variant name, in declaration order. Kept in sync by a test.
    pub const ALL_KINDS: &'static [&'static str] = &[
        "SocietyCreated",
        "EpochStarted",
        "Seeded",
        "EpochEnded",
        "CitizenJoined",
        "CitizenSeen",
        "CitizenDormant",
        "CitizenReturned",
        "HouseholderJoined",
        "HouseholderEmigrated",
        "PlanChanged",
        "LaborSet",
        "Transferred",
        "OrderPlaced",
        "OrderCancelled",
        "OrderExpired",
        "Trade",
        "SaleOffered",
        "SaleAccepted",
        "SaleCancelled",
        "WantedPosted",
        "WantedRemoved",
        "OrgFounded",
        "WorkplaceAdded",
        "Assigned",
        "Unassigned",
        "ManagerAppointed",
        "MemberAdmitted",
        "MemberLeft",
        "SharesIssued",
        "SharesTransferred",
        "DividendDeclared",
        "DividendPaid",
        "MachinesInstalled",
        "MachinesUninstalled",
        "MachinesDepreciated",
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
        "Produced",
        "Drew",
        "HardshipBegan",
        "HardshipEnded",
        "DestitutionBegan",
        "DestitutionEnded",
        "PolicyChanged",
        "TickResolved",
        "CycleClosed",
    ];
}
