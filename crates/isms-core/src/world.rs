//! The society's state (TDD §6). Arena-style `BTreeMap`s keyed by typed ids so
//! iteration is deterministic and serialization is canonical. Nothing here is
//! mutated except by `apply`.

use crate::config::Preset;
use crate::constitution::{Constitution, OfficeKind};
use crate::ids::{
    CitizenId, ContractId, Cycle, DwellingId, Epoch, OfferId, OrderId, OrgId, ProposalId, SlotId,
    Tick, WorkplaceId,
};
use crate::kinds::{CitizenKind, Effort, Good, JobFamily, OrgKind, WorkplaceKind};
use crate::ledger::{Asset, LedgerMeta, Party};
use crate::money::Money;
use crate::params::Params;
use crate::policy::Policy;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Society

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SocietyMeta {
    pub society_id: u64,
    pub preset: String,
    pub display: String,
    /// Seed for all randomness; per-tick RNG is derived from it (TDD §5.9).
    pub seed: u64,
    pub epoch: Epoch,
    /// The next tick to resolve (0 before any tick has run).
    pub tick: Tick,
    /// Set when the epoch has ended and no new epoch has started.
    pub epoch_ended: Option<EpochEndReason>,
    /// The last cycle of the epoch, once the end has been announced (S1.15).
    pub epoch_ending: Option<Cycle>,
    /// Consecutive cycles with active humans below the population floor (GDD Q7).
    pub low_population_cycles: u32,
    /// Whether active humans have reached the population floor at any cycle end
    /// of this epoch. Collapse counts only after that (ADR-0006): a society that
    /// never drew a crowd is an AI economy, not an abandoned one.
    pub reached_floor: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpochEndReason {
    Scheduled,
    Collapse,
    Operator,
}

/// The whole society. Built only by folding events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub meta: SocietyMeta,
    pub constitution: Constitution,
    pub policy: Policy,
    pub params: Params,
    pub citizens: BTreeMap<CitizenId, Citizen>,
    pub orgs: BTreeMap<OrgId, Org>,
    pub workplaces: BTreeMap<WorkplaceId, Workplace>,
    pub dwellings: BTreeMap<DwellingId, Dwelling>,
    pub land: LandRegistry,
    pub books: BTreeMap<Instrument, OrderBook>,
    pub store: Option<CommonStore>,
    pub state_stock: Option<StateStock>,
    /// The tax treasury (money systems with redistribution).
    pub treasury: Money,
    pub contracts: BTreeMap<ContractId, Contract>,
    pub offers: BTreeMap<OfferId, Offer>,
    /// Money or goods held against an order or an offer/contract.
    pub escrow: BTreeMap<EscrowKey, Asset>,
    /// Shares held against an order or an offer: (org, qty).
    pub share_escrow: BTreeMap<EscrowKey, (OrgId, u64)>,
    pub offices: Offices,
    pub proposals: BTreeMap<ProposalId, Proposal>,
    /// Pending transfer requests (assigned-labor systems): citizen -> wanted workplace.
    pub transfer_requests: BTreeMap<CitizenId, WorkplaceId>,
    pub ledger_meta: LedgerMeta,
    pub next: NextIds,
    /// Basket price index as of the last tick (market systems).
    pub price_index: Option<f64>,
    /// This cycle's production totals (metrics).
    pub cycle: crate::metrics::WorldCycle,
}

/// What an escrow entry is held against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscrowKey {
    Order(OrderId),
    Offer(OfferId),
    Contract(ContractId),
}

/// Id allocation counters; part of state so replay assigns the same ids.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextIds {
    pub citizen: CitizenId,
    pub org: OrgId,
    pub workplace: WorkplaceId,
    pub dwelling: DwellingId,
    pub contract: ContractId,
    pub offer: OfferId,
    pub order: OrderId,
    pub proposal: ProposalId,
}

impl World {
    /// An empty society from a loaded preset; `apply(SocietyCreated)` calls this.
    #[must_use]
    pub fn new(society_id: u64, seed: u64, preset: &Preset) -> Self {
        let land = LandRegistry::from_params(&preset.params);
        let store = (!preset.constitution.has_money()).then(CommonStore::default);
        let state_stock = matches!(
            preset.constitution.pricing,
            crate::constitution::Pricing::Administered
        )
        .then(StateStock::default);
        World {
            meta: SocietyMeta {
                society_id,
                preset: preset.name.clone(),
                display: preset.display.clone(),
                seed,
                epoch: 0,
                tick: 0,
                epoch_ended: None,
                epoch_ending: None,
                low_population_cycles: 0,
                reached_floor: false,
            },
            constitution: preset.constitution.clone(),
            policy: preset.policy.clone(),
            params: preset.params.clone(),
            citizens: BTreeMap::new(),
            orgs: BTreeMap::new(),
            workplaces: BTreeMap::new(),
            dwellings: BTreeMap::new(),
            land,
            books: BTreeMap::new(),
            store,
            state_stock,
            treasury: Money::ZERO,
            contracts: BTreeMap::new(),
            offers: BTreeMap::new(),
            escrow: BTreeMap::new(),
            share_escrow: BTreeMap::new(),
            offices: Offices::default(),
            proposals: BTreeMap::new(),
            transfer_requests: BTreeMap::new(),
            ledger_meta: LedgerMeta::default(),
            next: NextIds::default(),
            price_index: None,
            cycle: crate::metrics::WorldCycle::default(),
        }
    }

    #[must_use]
    pub const fn ticks_per_cycle(&self) -> u32 {
        self.params.time.ticks_per_cycle
    }

    /// The cycle a tick belongs to (0-based).
    #[must_use]
    pub const fn cycle_of(&self, tick: Tick) -> Cycle {
        tick / self.params.time.ticks_per_cycle
    }

    /// Whether `tick` is the last tick of its cycle.
    #[must_use]
    pub const fn is_cycle_end(&self, tick: Tick) -> bool {
        tick % self.params.time.ticks_per_cycle == self.params.time.ticks_per_cycle - 1
    }

    /// Canonical bytes: postcard over the whole state.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("World is always serializable")
    }

    /// A 32-byte hash of the canonical bytes.
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        *blake3::hash(&self.canonical_bytes()).as_bytes()
    }
}

// ---------------------------------------------------------------------------
// Citizens and households

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Citizen {
    pub id: CitizenId,
    pub handle: String,
    pub kind: CitizenKind,
    pub joined_tick: Tick,
    pub last_seen_tick: Tick,
    pub dormant: bool,
    pub household: Household,
    pub labor: LaborState,
    pub needs: Needs,
    pub plan: StandingPlan,
    pub flags: CitizenFlags,
    /// Commands by client kind, for the API-share telemetry.
    pub api_share: BTreeMap<crate::kinds::ClientKind, u32>,
    /// Wages and dividends received in total and in the last closed cycle (Q39).
    pub wages_total: Money,
    pub last_cycle_wages: Money,
    pub cycle_wages: Money,
    /// This cycle's consumption and wellbeing totals (metrics).
    pub cycle: crate::metrics::CitizenCycle,
    /// The Ledger of Contribution (norm systems, S0.15b).
    pub contribution: ContributionRecord,
    /// Income since the last tax assessment (tax-transfer systems, Q83).
    pub taxable_income: Money,
}

/// A citizen's public contribution record (GDD §6.2): hours exact, output as
/// attributed under the society's monitoring, and the norm cycles met.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContributionRecord {
    pub cycles: u32,
    pub tick_hours_total: u64,
    pub attributed_total: f64,
    pub norm_met_cycles: u32,
    pub last_cycle_tick_hours: u32,
    pub last_cycle_attributed: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Household {
    pub balance: Money,
    pub pantry: BTreeMap<Good, u32>,
    pub dwelling: Option<DwellingId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LaborState {
    /// At most `max_workplaces` allocations, each a rate in hours per cycle.
    pub allocations: Vec<Allocation>,
    /// This cycle's hour budget: `base_budget - fatigue_debt`.
    pub budget: u8,
    pub fatigue_debt: u8,
    pub consecutive_high_effort_cycles: u8,
    pub skill: BTreeMap<JobFamily, Skill>,
    /// Output multiplier for this tick, computed from needs at the end of the last one.
    pub output_mult: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    pub workplace: WorkplaceId,
    pub hours: u8,
    pub effort: Effort,
}

/// Skill in one job family (GDD §4.3, TDD §6). Hours are integer tick-hours (Q5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    /// 0..=100 as a float so decay of 1 per 10 idle cycles is exact in tenths.
    pub level: f64,
    /// Accumulated tick-hours in this family (24 per real hour).
    pub tick_hours: u64,
    /// Tick-hours worked this cycle; reset at step 8g.
    pub tick_hours_this_cycle: u32,
    pub idle_cycles: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Needs {
    /// Meters in tenths of a point, 0..=1000 (see `needs::TENTHS`).
    pub food: u16,
    pub shelter: u16,
    pub comfort: u16,
    /// Ticks so far this cycle with Food below the hardship threshold (Q13).
    pub low_food_ticks_this_cycle: u8,
    pub consecutive_hardship_cycles: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // they are the public flags (GDD Q3)
pub struct CitizenFlags {
    pub in_hardship: bool,
    pub destitute: bool,
    pub defaulted: bool,
    pub options_narrowed: bool,
}

// ---------------------------------------------------------------------------
// Standing plan (GDD §9.3, TDD §6)

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StandingPlan {
    pub labor: LaborPlan,
    pub keep_food_at_least: u32,
    /// `None` = last price x 1.25.
    pub max_food_price: Option<Money>,
    pub buy_wares_when: Option<BuyRule>,
    pub keep_balance_at_least: Money,
    pub standing_orders: Vec<StandingOrder>,
    pub vote_default: VoteDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaborPlan {
    /// Use `LaborState.allocations` as set by `SetLabor`.
    Explicit,
    AcceptAssignment,
    FollowNorm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuyRule {
    pub comfort_below: u8,
    pub balance_above: Money,
    pub max_price: Option<Money>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingOrder {
    pub instrument: Instrument,
    pub side: Side,
    pub qty: u32,
    pub limit_price: Money,
    pub refresh: Refresh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refresh {
    EachTick,
    EachCycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteDefault {
    Abstain,
    Follow(CitizenId),
    None,
}

// ---------------------------------------------------------------------------
// Orgs, workplaces, land, dwellings

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Org {
    pub id: OrgId,
    pub kind: OrgKind,
    pub name: String,
    pub ownership: Ownership,
    pub manager: Option<CitizenId>,
    pub treasury: Money,
    pub inventory: BTreeMap<Good, u32>,
    pub workplaces: BTreeSet<WorkplaceId>,
    pub employees: BTreeSet<ContractId>,
    pub members: BTreeSet<CitizenId>,
    pub founded_tick: Tick,
    /// Set when a payslip could not be covered (TDD §5.4 `PaymentMissed`).
    pub payment_missed: bool,
    /// Per-share dividend declared this cycle, paid at 8e and cleared at cycle close.
    pub declared_dividend: Option<Money>,
    // --- cooperatives (S0.17b) ----------------------------------------------
    /// How the surplus is split; `None` outside cooperatives.
    pub share_rule: Option<ShareRule>,
    /// The treasury at the last cycle close plus capital received since:
    /// what this cycle's surplus is measured against (Q85).
    pub surplus_base: Money,
    pub member_since: BTreeMap<CitizenId, Tick>,
    pub last_surplus: Money,
    pub last_share_out_members: u32,
    /// Set for a union org (S0.17d).
    pub union: Option<UnionState>,
}

/// A union's firm and its current strike, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionState {
    pub firm: OrgId,
    /// The strike runs while `cycle < strike_until`.
    pub strike_until: Option<Cycle>,
}

/// How a cooperative shares its surplus (GDD §6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareRule {
    Equal,
    HoursWeighted,
}

/// Who holds shares (ADR-0005). Serialized as a string (`citizen:7`, `org_self`)
/// because it keys the share registry, and JSON maps (the event log, S1.1)
/// can only be keyed by strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShareHolder {
    Citizen(CitizenId),
    OrgSelf,
}

impl std::fmt::Display for ShareHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShareHolder::Citizen(c) => write!(f, "citizen:{}", c.0),
            ShareHolder::OrgSelf => f.write_str("org_self"),
        }
    }
}

impl std::str::FromStr for ShareHolder {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "org_self" {
            return Ok(ShareHolder::OrgSelf);
        }
        s.strip_prefix("citizen:")
            .and_then(|n| n.parse::<u32>().ok())
            .map(|n| ShareHolder::Citizen(CitizenId(n)))
            .ok_or_else(|| format!("not a share holder: {s}"))
    }
}

impl Serialize for ShareHolder {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ShareHolder {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    /// Private firms: a share registry.
    Shares {
        issued: u64,
        holdings: BTreeMap<ShareHolder, u64>,
    },
    /// Cooperatives and associations: equal members.
    Members,
    /// Collectives and state enterprises.
    Society,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workplace {
    pub id: WorkplaceId,
    pub kind: WorkplaceKind,
    pub org: OrgId,
    pub slot: Option<SlotId>,
    pub machines: u32,
    /// Accumulated depreciation not yet realised as a lost machine.
    pub machine_wear: f64,
    pub output_remainder: f64,
    /// Workers with an assignment here; mirrors `Citizen.labor.allocations`.
    pub workers: BTreeMap<CitizenId, Assignment>,
    pub cycle_output: f64,
    /// Plan target (Directorate).
    pub target: Option<f64>,
    /// Last closed cycle's output and fulfilment (output / target), for the
    /// scoreboard and the planner (S0.16b).
    pub last_cycle_output: f64,
    pub last_fulfillment: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub hours: u8,
    pub effort: Effort,
    pub contract: Option<ContractId>,
    /// This cycle's tick-hours and attributed output, for payroll (S0.10).
    pub cycle_tick_hours: u32,
    pub cycle_attributed: f64,
}

impl Assignment {
    #[must_use]
    pub const fn new(contract: Option<ContractId>) -> Self {
        Assignment {
            hours: 0,
            effort: Effort::Normal,
            contract,
            cycle_tick_hours: 0,
            cycle_attributed: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandRegistry {
    pub slots: BTreeMap<SlotId, Slot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub kind: WorkplaceKind,
    pub workplace: Option<WorkplaceId>,
}

impl LandRegistry {
    #[must_use]
    pub fn from_params(params: &Params) -> Self {
        let mut slots = BTreeMap::new();
        let mut next = 0u32;
        for kind in WorkplaceKind::ALL {
            if let Some(n) = params.land.get(&kind) {
                for _ in 0..*n {
                    slots.insert(
                        SlotId(next),
                        Slot {
                            kind,
                            workplace: None,
                        },
                    );
                    next += 1;
                }
            }
        }
        LandRegistry { slots }
    }

    /// The first free slot of a kind, if the kind is slot-limited and one is free.
    #[must_use]
    pub fn free_slot(&self, kind: WorkplaceKind) -> Option<SlotId> {
        self.slots
            .iter()
            .find(|(_, s)| s.kind == kind && s.workplace.is_none())
            .map(|(id, _)| *id)
    }

    #[must_use]
    pub fn is_limited(&self, kind: WorkplaceKind) -> bool {
        self.slots.values().any(|s| s.kind == kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dwelling {
    pub id: DwellingId,
    pub owner: Owner,
    pub occupant: Option<CitizenId>,
    pub lease: Option<ContractId>,
    pub built_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    Citizen(CitizenId),
    Org(OrgId),
    Society,
}

// ---------------------------------------------------------------------------
// Markets and stores

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Instrument {
    Good(Good),
    Share(OrgId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSource {
    Manual,
    Standing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub owner: Party,
    pub instrument: Instrument,
    pub side: Side,
    pub qty: u32,
    pub remaining: u32,
    pub limit_price: Money,
    pub placed_tick: Tick,
    pub expires_tick: Tick,
    pub source: OrderSource,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBook {
    pub orders: BTreeMap<OrderId, Order>,
    pub last_price: Option<Money>,
    pub last_trade_tick: Option<Tick>,
    /// Trades since the last tick, for the tick's VWAP; reset by `TickResolved`.
    pub tick_volume: u32,
    pub tick_value: Money,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonStore {
    pub stock: BTreeMap<Good, u32>,
    /// This tick's draw requests in arrival order; resolved in phase 6 and
    /// cleared by `TickResolved` (S0.15).
    pub requests: Vec<StoreRequest>,
}

/// One citizen's request to draw from the Common Store this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreRequest {
    pub citizen: CitizenId,
    pub good: Good,
    pub qty: u32,
    pub tick: Tick,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateStock {
    pub stock: BTreeMap<Good, u32>,
    /// The state's one purse (Q67): store takings, the seeded budget, and the
    /// source of state wages. A holder for conservation.
    pub till: Money,
    /// This tick's purchase requests in arrival order; served in phase 6 and
    /// cleared by `TickResolved` (S0.16a).
    pub requests: Vec<StateRequest>,
    /// Units sold per citizen per good this cycle, for ration cards; cleared
    /// by `CycleClosed`.
    pub issued_this_cycle: BTreeMap<CitizenId, BTreeMap<Good, u32>>,
}

/// One citizen's request to buy from the state store this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRequest {
    pub citizen: CitizenId,
    pub good: Good,
    pub qty: u32,
    pub tick: Tick,
}

// ---------------------------------------------------------------------------
// Contracts and offers (GDD §7.2). Bodies are filled in by S0.10 and S0.11.

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contract {
    pub id: ContractId,
    pub parties: (Party, Party),
    pub created_tick: Tick,
    pub term_cycles: Option<u32>,
    pub status: ContractStatus,
    pub body: ContractBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    Active,
    Suspended,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pay {
    Hourly(Money),
    PieceRate(Money),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractBody {
    Employment {
        org: OrgId,
        workplace: WorkplaceId,
        pay: Pay,
        max_hours: u8,
        notice_cycles: u32,
    },
    Credit {
        principal: Money,
        rate_per_cycle_bp: u32,
        installment: Money,
        installments_left: u32,
        collateral: Option<Collateral>,
        /// The last due installment was missed; phase 7 resolves the default.
        missed: bool,
    },
    Lease {
        asset: LeaseAsset,
        rent_per_cycle: Money,
        missed_cycles: u32,
    },
    CollectiveAgreement {
        wage_floor: Money,
        hours: u8,
    },
    Pledge {
        hours: Option<u8>,
        goods: Option<(Good, u32)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Collateral {
    Dwelling(DwellingId),
    Shares(OrgId, u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseAsset {
    Dwelling(DwellingId),
    Workplace(WorkplaceId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: OfferId,
    pub by: Party,
    pub created_tick: Tick,
    pub body: OfferBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleAsset {
    Good(Good, u32),
    Shares(OrgId, u64),
    Dwelling(DwellingId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Price {
    Money(Money),
    Good(Good, u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferBody {
    Employment {
        org: OrgId,
        workplace: WorkplaceId,
        pay: Pay,
        max_hours: u8,
        term_cycles: Option<u32>,
        notice_cycles: u32,
        places: u32,
    },
    Sale {
        asset: SaleAsset,
        price: Price,
        to: Option<Party>,
    },
    Wanted {
        good: Good,
        qty: u32,
        max_price: Money,
    },
    Credit {
        to: Option<Party>,
        principal: Money,
        rate_per_cycle_bp: u32,
        term_cycles: u32,
        collateral: Option<Collateral>,
    },
    Lease {
        asset: LeaseAsset,
        rent_per_cycle: Money,
        term_cycles: Option<u32>,
    },
    Membership {
        org: OrgId,
        citizen: CitizenId,
    },
    /// A cooperative's application to the Public Investment Bank (S0.17c).
    BankLoan {
        org: OrgId,
        principal: Money,
        term_cycles: u32,
    },
    /// A union's proposed terms to its firm (S0.17d).
    CollectiveAgreement {
        union: OrgId,
        firm: OrgId,
        wage_floor: Money,
        hours: u8,
        term_cycles: u32,
    },
}

// ---------------------------------------------------------------------------
// Governance (Phase 2 fills these in)

/// The society's offices (GDD §8.2, §8.3; S2.2): who sits, which elections
/// are open, and what each citizen has served. Reset with the epoch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offices {
    pub holders: BTreeMap<OfficeKind, Vec<OfficeHolder>>,
    /// The election open for an office, at most one at a time (S2.2).
    #[serde(default)]
    pub elections: BTreeMap<OfficeKind, Election>,
    /// What each citizen has served in each office: the tie-break and the
    /// no-consecutive rule read it (S2.2, Q118).
    #[serde(default)]
    pub past_terms: BTreeMap<OfficeKind, BTreeMap<CitizenId, ServiceRecord>>,
    /// The cycle since which an office has had fewer holders than seats;
    /// absent while it is full (S2.2, GDD §8.3's five-cycle headline).
    #[serde(default)]
    pub short_since: BTreeMap<OfficeKind, Cycle>,
}

impl Offices {
    /// The seats of `kind` filled right now.
    #[must_use]
    pub fn filled(&self, kind: OfficeKind) -> u32 {
        self.holders
            .get(&kind)
            .map_or(0, |h| u32::try_from(h.len()).unwrap_or(u32::MAX))
    }

    /// Whether `citizen` sits in `kind` right now.
    #[must_use]
    pub fn holds(&self, citizen: CitizenId, kind: OfficeKind) -> bool {
        self.holders
            .get(&kind)
            .is_some_and(|seats| seats.iter().any(|h| h.citizen == citizen))
    }

    /// `citizen`'s record in `kind`, empty when they never sat.
    #[must_use]
    pub fn record(&self, citizen: CitizenId, kind: OfficeKind) -> ServiceRecord {
        self.past_terms
            .get(&kind)
            .and_then(|m| m.get(&citizen))
            .copied()
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeHolder {
    pub citizen: CitizenId,
    /// The last cycle of the term: the seat empties at that cycle's end (8j).
    pub term_ends_cycle: Cycle,
}

/// One citizen's service in one office (S2.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRecord {
    /// Terms served, full or partial: the election tie-break (Q118).
    pub terms: u32,
    /// The cycle a full term last ended; a partial term (recall, absence)
    /// leaves it alone, so it never bars the next election (Q118).
    pub last_full_term_ended: Option<Cycle>,
}

/// An election for the vacant seats of one office (S2.2, Q118): approval
/// ballots, the top `seats` by approvals win, ties by fewer past terms then
/// lower id. Opened at 8j (or at epoch start) and closed at the next 8j; with
/// no candidate it re-runs each cycle (GDD §8.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Election {
    pub office: OfficeKind,
    /// Seats this election fills.
    pub seats: u32,
    /// The cycle the election opened in; kept across re-runs.
    pub opened_cycle: Cycle,
    /// The cycle whose end (8j) closes it.
    pub closes_cycle: Cycle,
    pub candidates: BTreeSet<CitizenId>,
    /// Each voter's approved candidates, replaceable until close.
    pub approvals: BTreeMap<CitizenId, BTreeSet<CitizenId>>,
}

/// Why a seat emptied (S2.2; GDD §8.2 "Removal", §8.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VacancyReason {
    /// The term ran its course.
    TermEnded,
    /// No session for `population.office_vacancy_absent_cycles` cycles.
    Absence,
    /// A recall carried by the office's `RecallRule`.
    Recalled,
}

/// One citizen's ballot on a proposal (GDD 8.1: one citizen one vote).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ballot {
    Yes,
    No,
    Abstain,
}

/// The count a proposal closed on (S2.1). `cast` includes abstentions and the
/// ballots `vote_default` cast; `quorum` is what the close required (Q117).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    pub yes: u32,
    pub no: u32,
    pub abstain: u32,
    pub cast: u32,
    pub quorum: u32,
    pub eligible: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub by: CitizenId,
    pub opened_tick: Tick,
    pub title: String,
    /// The proposer's case, free text; a `Resolution` is nothing but this.
    pub text: String,
    pub kind: ProposalKind,
    /// The cycle whose end (8j) closes the vote.
    pub closes_cycle: Cycle,
    /// Ballots cast so far, replaceable until close.
    pub ballots: BTreeMap<CitizenId, Ballot>,
}

/// What a proposal decides. `Admission` is a cooperative's members' vote
/// (S0.17c); the rest are the assembly's (S2.1). `Election`, `Recall`, `Honor`
/// and `Disbursement` are declared here and given their effects by S2.2-S2.4.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    Admission {
        org: OrgId,
        citizen: CitizenId,
    },
    PolicyChange {
        patch: crate::policy::PolicyPatch,
    },
    Resolution,
    Election {
        office: OfficeKind,
    },
    Recall {
        office: OfficeKind,
        citizen: CitizenId,
    },
    Honor {
        citizen: CitizenId,
    },
    Disbursement {
        org: OrgId,
    },
}

impl ProposalKind {
    /// The constitution-level tag this kind is gated by.
    #[must_use]
    pub const fn tag(&self) -> crate::constitution::ProposalKindTag {
        use crate::constitution::ProposalKindTag as T;
        match self {
            ProposalKind::Admission { .. } => T::Admission,
            ProposalKind::PolicyChange { .. } => T::PolicyChange,
            ProposalKind::Resolution => T::Resolution,
            ProposalKind::Election { .. } => T::Election,
            ProposalKind::Recall { .. } => T::Recall,
            ProposalKind::Honor { .. } => T::Honor,
            ProposalKind::Disbursement { .. } => T::Disbursement,
        }
    }
}
