//! The event catalog (TDD §5.4). Discrete events record things that happened;
//! `TickResolved` carries the continuous bookkeeping and never a movement of
//! money or goods between holders (D4). Externally tagged so postcard and JSON
//! both work.

use crate::config::Preset;
use crate::constitution::OfficeKind;
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
    Ownership, Pay, Price, SaleAsset, Skill, StandingPlan, VacancyReason,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
        /// The frozen summary the archive keeps (S1.15, GDD 11.5), computed
        /// in the same tick so replay reproduces it byte for byte.
        summary: EpochSummary,
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
    /// A standing request to join a member org (the notice-board "membership" ad).
    MembershipRequested {
        offer: OfferId,
        org: OrgId,
        citizen: CitizenId,
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
        materials_consumed: u32,
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
        by: Party,
        body: OfferBody,
    },
    CreditAccepted {
        contract: ContractId,
        lender: Party,
        borrower: Party,
        principal: Money,
        rate_per_cycle_bp: u32,
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
    /// A due installment the borrower could not pay; resolves as a default in phase 7.
    CreditMissed {
        contract: ContractId,
        owed: Money,
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
        /// The proposal whose close made the change (S2.1); `None` for `SetPolicy`.
        proposal: Option<crate::ids::ProposalId>,
    },

    // --- the tick -----------------------------------------------------------
    TickResolved {
        tick: Tick,
        cycle: Cycle,
        price_index: Option<f64>,
        /// Volume-weighted average price per instrument over the trades since the last tick.
        vwap: Vec<(Instrument, Money)>,
        citizen_deltas: Vec<CitizenDelta>,
        workplace_deltas: Vec<WorkplaceDelta>,
    },
    CycleClosed {
        cycle: Cycle,
        aggregates: CycleAggregates,
        /// The collapse counter after this cycle (phase 8m).
        low_population_cycles: u32,
        /// Whether the floor has been reached this epoch, after this cycle (ADR-0006).
        #[serde(default)]
        reached_floor: bool,
    },

    // --- Phase 0b (appended: postcard tags variants by index, Q50) ----------
    /// A Common Store draw request filed for this tick (S0.15).
    StoreDrawRequested {
        citizen: CitizenId,
        good: Good,
        qty: u32,
        tick: Tick,
    },
    /// A departing citizen's pantry and balance return to the society's stock
    /// instead of being burned (Q47).
    StoreReturned {
        citizen: CitizenId,
        holder: Holder,
        goods: BTreeMap<Good, u32>,
        money: Money,
    },
    /// A public promise of hours per cycle and/or goods (GDD §7.2; S0.15b).
    Pledged {
        contract: ContractId,
        citizen: CitizenId,
        /// `None` pledges to the assembly.
        to: Option<OrgId>,
        hours: Option<u8>,
        goods: Option<(Good, u32)>,
        term_cycles: u32,
    },
    /// A pledge's term ran out; `met` is whether the hours half was kept
    /// (`None` for a goods-only pledge).
    PledgeClosed {
        contract: ContractId,
        met: Option<bool>,
    },
    /// The Ledger of Contribution for one cycle (norm systems, step 8i).
    NormsLedgerClosed {
        cycle: Cycle,
        entries: Vec<ContributionEntry>,
    },
    /// A state-store purchase request filed for this tick (S0.16a).
    StateStoreRequested {
        citizen: CitizenId,
        good: Good,
        qty: u32,
        tick: Tick,
    },
    /// The state store sold at the list price: money to the till, goods to the pantry.
    StateStoreSold {
        citizen: CitizenId,
        good: Good,
        qty: u32,
        unit_price: Money,
        total: Money,
        explain: Explain,
    },
    /// Requests the state store could not serve this tick.
    StateStoreShortage {
        tick: Tick,
        unfilled: BTreeMap<Good, u32>,
    },
    /// Provision: goods issued at zero price from the state stock (step 8b).
    RationIssued {
        citizen: CitizenId,
        good: Good,
        qty: u32,
        explain: Explain,
    },
    /// The Committee published output targets (S0.16b).
    PlanPublished {
        cycle: Cycle,
        targets: BTreeMap<WorkplaceId, f64>,
        by: Actor,
    },
    /// One workplace's target changed (the ratchet, or a later edit).
    TargetSet {
        workplace: WorkplaceId,
        target: f64,
        by: Actor,
    },
    TransferRequested {
        citizen: CitizenId,
        to_workplace: WorkplaceId,
    },
    TransferDecided {
        citizen: CitizenId,
        to_workplace: WorkplaceId,
        approved: bool,
    },
    /// Income tax on the cycle's earnings, to the treasury (S0.17a).
    TaxAssessed {
        citizen: CitizenId,
        income: Money,
        tax: Money,
        explain: Explain,
    },
    /// The need floor: a top-up from a public purse (the treasury, or the bank).
    NeedFloorPaid {
        citizen: CitizenId,
        amount: Money,
        from: Holder,
        explain: Explain,
    },
    /// A cooperative's cycle surplus, about to be shared (S0.17b).
    SurplusDeclared {
        org: OrgId,
        cycle: Cycle,
        surplus: Money,
        rule: crate::world::ShareRule,
        members: u32,
    },
    /// One member's share of the surplus.
    ShareOutPaid {
        org: OrgId,
        citizen: CitizenId,
        amount: Money,
        explain: Explain,
    },
    ShareRuleSet {
        org: OrgId,
        rule: crate::world::ShareRule,
    },
    /// The capital levy, coop to bank (S0.17c).
    LevyPaid {
        org: OrgId,
        bank: OrgId,
        amount: Money,
        explain: Explain,
    },
    BankLoanRequested {
        offer: OfferId,
        org: OrgId,
        principal: Money,
        term_cycles: u32,
    },
    BankLoanDecided {
        application: OfferId,
        org: OrgId,
        granted: bool,
    },
    AdmissionProposed {
        proposal: crate::ids::ProposalId,
        org: OrgId,
        citizen: CitizenId,
        by: CitizenId,
    },
    AdmissionVoted {
        proposal: crate::ids::ProposalId,
        citizen: CitizenId,
        approve: bool,
    },
    ProposalClosed {
        proposal: crate::ids::ProposalId,
        passed: bool,
        /// The count at close (S2.1). An admission vote counts its members.
        tally: crate::world::Tally,
    },
    /// The org just founded is the union of `firm` (S0.17d).
    UnionFormed {
        org: OrgId,
        firm: OrgId,
    },
    DuesPaid {
        union: OrgId,
        citizen: CitizenId,
        amount: Money,
    },
    CollectiveAgreementOffered {
        offer: OfferId,
        body: OfferBody,
    },
    CollectiveAgreementAccepted {
        contract: ContractId,
        offer: OfferId,
        union: OrgId,
        firm: OrgId,
        wage_floor: Money,
        hours: u8,
        term_cycles: u32,
    },
    CollectiveAgreementEnded {
        contract: ContractId,
    },
    StrikeCalled {
        union: OrgId,
        firm: OrgId,
        until_cycle: Cycle,
    },
    StrikePaid {
        union: OrgId,
        citizen: CitizenId,
        amount: Money,
        explain: Explain,
    },
    StrikeEnded {
        union: OrgId,
    },
    /// An open employment, credit or lease offer taken back by its poster
    /// (Q109, S1.15). Carries the body so a ledger can say what it was.
    OfferWithdrawn {
        offer: OfferId,
        by: Party,
        body: OfferBody,
    },
    /// Two cycles remain (S1.15, GDD 11.5): emitted at the close of the cycle
    /// two before the last. Collapse and an operator end give no warning.
    EpochEnding {
        final_cycle: Cycle,
    },
    // --- Phase 2 (appended) -------------------------------------------------
    /// A proposal opened before the assembly (S2.1).
    Proposed {
        proposal: crate::ids::ProposalId,
        by: CitizenId,
        title: String,
        text: String,
        kind: crate::world::ProposalKind,
        closes_cycle: Cycle,
    },
    /// A ballot cast or replaced; `by_default` when 8j cast it from the
    /// standing plan's `vote_default` (S2.1).
    Voted {
        proposal: crate::ids::ProposalId,
        citizen: CitizenId,
        ballot: crate::world::Ballot,
        by_default: bool,
    },
    // --- offices (S2.2, appended) ------------------------------------------
    /// An election opened for `seats` seats of `office`, closing at the end
    /// of `closes_cycle`: at epoch start, and at 8j for any vacancy or a term
    /// that ends next cycle (S2.2).
    ElectionOpened {
        office: OfficeKind,
        seats: u32,
        closes_cycle: Cycle,
    },
    /// A citizen stands in the open election for `office` (S2.2).
    CandidacyDeclared {
        office: OfficeKind,
        citizen: CitizenId,
    },
    CandidacyWithdrawn {
        office: OfficeKind,
        citizen: CitizenId,
    },
    /// An approval ballot cast or replaced in the open election (S2.2, Q118).
    Approved {
        office: OfficeKind,
        citizen: CitizenId,
        candidates: BTreeSet<CitizenId>,
    },
    /// An election closed with candidates: every candidate's approvals, in
    /// rank order (S2.2). The `OfficeTaken` events that follow seat the winners.
    ElectionClosed {
        office: OfficeKind,
        approvals: Vec<(CitizenId, u32)>,
    },
    /// A seat taken; the term runs through `term_ends_cycle` (S2.2).
    OfficeTaken {
        office: OfficeKind,
        citizen: CitizenId,
        term_ends_cycle: Cycle,
        approvals: u32,
    },
    /// A seat emptied (S2.2; GDD 8.2, 8.3).
    OfficeVacated {
        office: OfficeKind,
        citizen: CitizenId,
        reason: VacancyReason,
    },
    /// An election with no candidate re-runs for one more cycle, for the
    /// seats short by then (S2.2, GDD 8.3).
    ElectionRerun {
        office: OfficeKind,
        seats: u32,
        closes_cycle: Cycle,
    },
    /// An office short of holders for `cycles` cycles, once per cycle from
    /// `governance.unfilled_office_headline_cycles` on (S2.2, GDD 8.3).
    OfficeUnfilled {
        office: OfficeKind,
        cycles: u32,
    },
    // --- the coordinator's powers and honors (S2.3, appended) --------------
    /// A coordinator opened a workplace of the collective on a land slot; the
    /// founding Materials came out of the Common Store (S2.3, Q120).
    WorkplaceOpened {
        workplace: WorkplaceId,
        org: OrgId,
        kind: WorkplaceKind,
        slot: Option<crate::ids::SlotId>,
        materials_consumed: u32,
        by: CitizenId,
    },
    /// A coordinator closed a workplace of the collective: its workers were
    /// unassigned by the `Unassigned` events just before this one, its
    /// machines went back to the collective's stock, its slot is free
    /// (S2.3, Q120).
    WorkplaceClosed {
        workplace: WorkplaceId,
        org: OrgId,
        slot: Option<crate::ids::SlotId>,
        machines_returned: u32,
        by: CitizenId,
    },
    /// The assembly honored a citizen: one line on their record, never
    /// revoked (S2.3, Q119).
    Honored {
        citizen: CitizenId,
        proposal: crate::ids::ProposalId,
        cycle: Cycle,
    },
    /// A member-owned org's disbursement vote carried: `asset` left the org's
    /// treasury or pantry for `to` (S2.4, Q122). Folds like `Transferred`.
    Disbursed {
        proposal: crate::ids::ProposalId,
        org: OrgId,
        to: Party,
        asset: Asset,
        explain: Explain,
    },
    /// A market org's shelves at cycle close (E-1, phase 8m): the stock of
    /// each good it produces and the markup step its householder manager
    /// will ask with. Emitted only when a shelf changed.
    ShelfClosed {
        org: OrgId,
        cycle: Cycle,
        shelf: BTreeMap<Good, crate::world::Shelf>,
    },
    /// A market workplace's labour board at cycle close (E-7, phase 8m): the
    /// step its householder manager's next wage offer will take, up one when
    /// an offer stood unfilled at the close and down one when the org's shelf
    /// of its output grew. Emitted only when the step changed.
    WageStepped {
        workplace: WorkplaceId,
        cycle: Cycle,
        step: i32,
    },
}

/// One citizen's line in the cycle's Ledger of Contribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContributionEntry {
    pub citizen: CitizenId,
    pub tick_hours: u32,
    pub attributed: f64,
    pub met_norm: bool,
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
    /// This cycle's running totals (reset at 8m).
    pub cycle: crate::metrics::CitizenCycle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkplaceDelta {
    pub workplace: WorkplaceId,
    pub machine_wear: f64,
    pub output_remainder: f64,
    pub cycle_output: f64,
    /// This cycle's accumulators per assigned worker.
    pub workers: BTreeMap<CitizenId, WorkerCycle>,
    /// Set at cycle end (8m) before the accumulators reset (S0.16b).
    pub last_cycle_output: f64,
    pub last_fulfillment: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerCycle {
    pub tick_hours: u32,
    pub attributed: f64,
}

/// What an epoch leaves behind (S1.15, GDD 11.5): the last cycle's aggregates
/// and every citizen's standing, computed in the tick that ends the epoch so
/// the archive is a fact of the log, never a query over it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EpochSummary {
    pub aggregates: CycleAggregates,
    /// Every citizen, ranked by net worth, highest first, ties by id; where
    /// labor is by norm, by the contribution record, then honors (Q157).
    pub standings: Vec<Standing>,
}

/// The Ledger of Contribution in three figures (GDD §6.2 Scoreboard), carried
/// on a standing where labor is by norm so the archive can rank a Commune's
/// epoch by its own scoreboard (Q157).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContributionTotals {
    pub hours_total: f64,
    pub days: u32,
    pub norm_met_days: u32,
}

/// One citizen's line in the epoch summary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Standing {
    pub citizen: CitizenId,
    pub handle: String,
    pub kind: CitizenKind,
    pub dormant: bool,
    pub net_worth: Money,
    pub self_made: Money,
    /// Assembly honors on the record (S2.3; the Commune's scoreboard).
    #[serde(default)]
    pub honors: u32,
    /// The contribution record where labor is by norm (Q157); `None` elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution: Option<ContributionTotals>,
}

/// Per-cycle metrics snapshot (TDD §13), computed in the engine at step 8m.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CycleAggregates {
    pub population: u32,
    pub active_humans: u32,
    pub householders: u32,
    /// Units produced weighted by the reference basket, dwellings by unit.
    pub real_output: f64,
    /// Median over citizens of the mean need satisfaction over the cycle (0..=100).
    pub median_wellbeing: f64,
    /// Share of citizen-cycles in which Food and Shelter never fell below the threshold.
    pub need_fulfillment_rate: f64,
    /// Gini of the consumption score (Food + Wares + housed ticks), never of wealth.
    pub consumption_gini: f64,
    /// Materials consumed by Machine Shops over Materials produced.
    pub investment_share: f64,
    pub price_index: Option<f64>,
    /// Mean wages paid this cycle to those paid anything, in credits.
    pub mean_cycle_wage: f64,
    pub unemployed: u32,
    pub firm_count: u32,
    pub credit_outstanding: Money,
    pub hardship_count: u32,
    pub store_stock: BTreeMap<Good, u32>,
    pub low_population_cycles: u32,
    // --- Phase 0b (S0.16b) --------------------------------------------------
    /// Mean output / target over workplaces with a target (administered systems).
    pub plan_fulfillment: Option<f64>,
    /// Units requested from the state store this cycle and not served.
    pub store_unfilled: u64,
    /// Units issued at zero price by provision this cycle.
    pub rations_issued: u64,
    pub state_stock: BTreeMap<Good, u32>,
    /// The state's till (administered systems).
    pub till: Money,
    /// Gini of last cycle's hours on the Ledger of Contribution (norm systems).
    pub contribution_gini: f64,
    // --- S0.17a --------------------------------------------------------------
    /// The tax treasury at cycle end (tax-transfer systems).
    pub treasury: Money,
    pub tax_collected: Money,
    pub floor_paid: Money,
    // --- S0.17b --------------------------------------------------------------
    /// Mean over cooperatives of last cycle's surplus per member, in credits.
    pub coop_surplus_per_member: f64,
    /// Mean membership tenure in cycles over coop members.
    pub mean_tenure_cycles: f64,
    // --- S0.17c --------------------------------------------------------------
    /// The Public Investment Bank's pool (its treasury) at cycle end.
    pub levy_pool: Money,
    /// Principal and interest still owed to the bank.
    pub bank_loans_outstanding: Money,
    /// The counts behind `investment_share`, so an epoch's share can be the
    /// ratio of sums rather than a mean of per-cycle ratios (Q100).
    pub materials_produced: u64,
    pub materials_to_machines: u64,
}

impl Event {
    /// The variant name, for registries and logs.
    #[must_use]
    #[allow(clippy::too_many_lines)]
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
            Event::MembershipRequested { .. } => "MembershipRequested",
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
            Event::CreditMissed { .. } => "CreditMissed",
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
            Event::StoreDrawRequested { .. } => "StoreDrawRequested",
            Event::StoreReturned { .. } => "StoreReturned",
            Event::Pledged { .. } => "Pledged",
            Event::PledgeClosed { .. } => "PledgeClosed",
            Event::NormsLedgerClosed { .. } => "NormsLedgerClosed",
            Event::StateStoreRequested { .. } => "StateStoreRequested",
            Event::StateStoreSold { .. } => "StateStoreSold",
            Event::StateStoreShortage { .. } => "StateStoreShortage",
            Event::RationIssued { .. } => "RationIssued",
            Event::PlanPublished { .. } => "PlanPublished",
            Event::TargetSet { .. } => "TargetSet",
            Event::TransferRequested { .. } => "TransferRequested",
            Event::TransferDecided { .. } => "TransferDecided",
            Event::TaxAssessed { .. } => "TaxAssessed",
            Event::NeedFloorPaid { .. } => "NeedFloorPaid",
            Event::SurplusDeclared { .. } => "SurplusDeclared",
            Event::ShareOutPaid { .. } => "ShareOutPaid",
            Event::ShareRuleSet { .. } => "ShareRuleSet",
            Event::LevyPaid { .. } => "LevyPaid",
            Event::BankLoanRequested { .. } => "BankLoanRequested",
            Event::BankLoanDecided { .. } => "BankLoanDecided",
            Event::AdmissionProposed { .. } => "AdmissionProposed",
            Event::AdmissionVoted { .. } => "AdmissionVoted",
            Event::ProposalClosed { .. } => "ProposalClosed",
            Event::UnionFormed { .. } => "UnionFormed",
            Event::DuesPaid { .. } => "DuesPaid",
            Event::CollectiveAgreementOffered { .. } => "CollectiveAgreementOffered",
            Event::CollectiveAgreementAccepted { .. } => "CollectiveAgreementAccepted",
            Event::CollectiveAgreementEnded { .. } => "CollectiveAgreementEnded",
            Event::StrikeCalled { .. } => "StrikeCalled",
            Event::StrikePaid { .. } => "StrikePaid",
            Event::StrikeEnded { .. } => "StrikeEnded",
            Event::OfferWithdrawn { .. } => "OfferWithdrawn",
            Event::EpochEnding { .. } => "EpochEnding",
            Event::Proposed { .. } => "Proposed",
            Event::Voted { .. } => "Voted",
            Event::ElectionOpened { .. } => "ElectionOpened",
            Event::CandidacyDeclared { .. } => "CandidacyDeclared",
            Event::CandidacyWithdrawn { .. } => "CandidacyWithdrawn",
            Event::Approved { .. } => "Approved",
            Event::ElectionClosed { .. } => "ElectionClosed",
            Event::OfficeTaken { .. } => "OfficeTaken",
            Event::OfficeVacated { .. } => "OfficeVacated",
            Event::ElectionRerun { .. } => "ElectionRerun",
            Event::OfficeUnfilled { .. } => "OfficeUnfilled",
            Event::WorkplaceOpened { .. } => "WorkplaceOpened",
            Event::WorkplaceClosed { .. } => "WorkplaceClosed",
            Event::Honored { .. } => "Honored",
            Event::Disbursed { .. } => "Disbursed",
            Event::ShelfClosed { .. } => "ShelfClosed",
            Event::WageStepped { .. } => "WageStepped",
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
        "MembershipRequested",
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
        "CreditMissed",
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
        "StoreDrawRequested",
        "StoreReturned",
        "Pledged",
        "PledgeClosed",
        "NormsLedgerClosed",
        "StateStoreRequested",
        "StateStoreSold",
        "StateStoreShortage",
        "RationIssued",
        "PlanPublished",
        "TargetSet",
        "TransferRequested",
        "TransferDecided",
        "TaxAssessed",
        "NeedFloorPaid",
        "SurplusDeclared",
        "ShareOutPaid",
        "ShareRuleSet",
        "LevyPaid",
        "BankLoanRequested",
        "BankLoanDecided",
        "AdmissionProposed",
        "AdmissionVoted",
        "ProposalClosed",
        "UnionFormed",
        "DuesPaid",
        "CollectiveAgreementOffered",
        "CollectiveAgreementAccepted",
        "CollectiveAgreementEnded",
        "StrikeCalled",
        "StrikePaid",
        "StrikeEnded",
        "OfferWithdrawn",
        "EpochEnding",
        "Proposed",
        "Voted",
        "ElectionOpened",
        "CandidacyDeclared",
        "CandidacyWithdrawn",
        "Approved",
        "ElectionClosed",
        "OfficeTaken",
        "OfficeVacated",
        "ElectionRerun",
        "OfficeUnfilled",
        "WorkplaceOpened",
        "WorkplaceClosed",
        "Honored",
        "Disbursed",
        "ShelfClosed",
        "WageStepped",
    ];
}
