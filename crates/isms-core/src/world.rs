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
    /// Consecutive cycles with active humans below the population floor (GDD Q7).
    pub low_population_cycles: u32,
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
    pub ledger_meta: LedgerMeta,
    pub next: NextIds,
    /// Basket price index as of the last tick (market systems).
    pub price_index: Option<f64>,
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
                low_population_cycles: 0,
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
            ledger_meta: LedgerMeta::default(),
            next: NextIds::default(),
            price_index: None,
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
}

/// Who holds shares (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareHolder {
    Citizen(CitizenId),
    OrgSelf,
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
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateStock {
    pub stock: BTreeMap<Good, u32>,
    /// Money the state store has taken in (a holder for conservation).
    pub till: Money,
    pub ration_caps: BTreeMap<Good, u32>,
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
}

// ---------------------------------------------------------------------------
// Governance (Phase 2 fills these in)

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offices {
    pub holders: BTreeMap<OfficeKind, Vec<OfficeHolder>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeHolder {
    pub citizen: CitizenId,
    pub term_ends_cycle: Cycle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub by: CitizenId,
    pub opened_tick: Tick,
    pub title: String,
}
