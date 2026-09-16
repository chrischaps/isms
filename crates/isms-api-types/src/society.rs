//! In-society wire types (TDD 10.3, S1.4): the Situation view, plan, labor,
//! market, orgs, contracts, society pages, and command results. Money is
//! always cents on the wire (TDD 10.4). Where a payload is the engine's own
//! serde shape (events, offers, contracts, the standing plan) it travels as a
//! JSON object and the schema says `Object`.

use crate::Clock;
use isms_core::ids::{CitizenId, DwellingId, OrgId, SlotId, WorkplaceId};
use isms_core::kinds::{Effort, Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Party};
use isms_core::money::Money;
use isms_core::world::{Collateral, LeaseAsset, Pay, Price, SaleAsset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

/// Money on the wire: integer cents.
pub type Cents = i64;

/// One stored event, as the viewer may see it.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct EventRef {
    pub seq: i64,
    pub tick: u32,
    pub cycle: u32,
    pub kind: String,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
}

/// What a command did: the events it produced, in log order.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Committed {
    pub clock: Clock,
    /// Log position of the first event; `None` when the command produced none.
    pub first_seq: Option<i64>,
    pub events: Vec<EventRef>,
}

// -- home ----------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NeedsView {
    /// 0..=100 with one decimal.
    pub food: f64,
    pub shelter: f64,
    pub comfort: f64,
    pub low_food_ticks_this_cycle: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DwellingView {
    pub id: u32,
    #[schema(value_type = Object)]
    pub owner: serde_json::Value,
    pub occupant: Option<u32>,
    pub lease: Option<u32>,
    pub rent_per_cycle: Option<Cents>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HouseholdView {
    pub balance: Cents,
    #[schema(value_type = Object)]
    pub pantry: BTreeMap<Good, u32>,
    /// Per-good pantry caps from the preset; a good absent here is uncapped.
    #[schema(value_type = Object)]
    pub pantry_capacity: BTreeMap<Good, u32>,
    pub dwelling: Option<DwellingView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AllocationView {
    pub workplace: u32,
    pub org: u32,
    pub org_name: String,
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub hours: u8,
    #[schema(value_type = String)]
    pub effort: Effort,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SkillView {
    pub family: String,
    pub level: f64,
    /// Real hours worked in this family, ever.
    pub hours: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContractView {
    pub id: u32,
    #[schema(value_type = Object)]
    pub parties: serde_json::Value,
    pub created_tick: u32,
    pub term_cycles: Option<u32>,
    pub status: String,
    #[schema(value_type = Object)]
    pub body: serde_json::Value,
    /// `party` when you are one, `manager` when your org is.
    pub role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct LaborView {
    pub budget: u8,
    pub fatigue_debt: u8,
    pub output_mult: f64,
    pub allocations: Vec<AllocationView>,
    pub skills: Vec<SkillView>,
    pub employment: Vec<ContractView>,
    /// What each effort level costs and yields (GDD 4.3), from the preset.
    pub effort: EffortCosts,
}

/// The effort table (GDD 4.3) as the client shows it in the editor's tooltips.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct EffortCosts {
    /// Output multiplier per level: low, normal, high.
    pub output_mult: [f64; 3],
    /// Food decay multiplier per level: low, normal, high.
    pub food_decay_mult: [f64; 3],
    /// High effort for more than this many consecutive cycles accrues fatigue debt.
    pub high_effort_debt_after_cycles: u8,
    /// Hours of budget lost per cycle of debt.
    pub high_effort_debt_hours: u8,
    pub max_workplaces: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CitizenSelfView {
    pub id: u32,
    pub handle: String,
    pub dormant: bool,
    pub joined_tick: u32,
    pub last_seen_tick: u32,
    #[schema(value_type = Object)]
    pub flags: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DigestView {
    pub since_tick: u32,
    pub events: Vec<EventRef>,
}

/// A Chronicle headline (S1.5 fills these; empty until then).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Headline {
    pub seq: i64,
    pub tick: u32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SocietyPulse {
    pub population: u32,
    pub active_humans: u32,
    pub price_index: Option<f64>,
    pub food_last_price: Option<Cents>,
    pub unemployed: u32,
}

/// The Situation view (GDD 9.1 step 1).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HomeView {
    pub clock: Clock,
    pub citizen: CitizenSelfView,
    pub household: HouseholdView,
    pub needs: NeedsView,
    pub labor: LaborView,
    #[schema(value_type = Object)]
    pub plan: serde_json::Value,
    pub since_last_seen: DigestView,
    pub headlines: Vec<Headline>,
    pub society: SocietyPulse,
}

// -- plan and labor -----------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PlanView {
    pub clock: Clock,
    #[schema(value_type = Object)]
    pub plan: serde_json::Value,
}

/// The whole standing plan (engine `StandingPlan` shape).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SetPlanRequest {
    #[schema(value_type = Object)]
    pub plan: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AllocationInput {
    pub workplace: u32,
    pub hours: u8,
    #[schema(value_type = String)]
    pub effort: Effort,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SetLaborRequest {
    pub allocations: Vec<AllocationInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PayslipsView {
    pub clock: Clock,
    pub payslips: Vec<EventRef>,
}

// -- market --------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BookSummary {
    /// `food`, `wares`, ... or `share:<org id>`.
    pub instrument: String,
    pub last_price: Option<Cents>,
    pub best_bid: Option<Cents>,
    pub best_ask: Option<Cents>,
    pub bid_depth: u32,
    pub ask_depth: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BooksView {
    pub clock: Clock,
    pub price_index: Option<f64>,
    pub books: Vec<BookSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Level {
    pub price: Cents,
    pub qty: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderView {
    pub id: u32,
    pub side: String,
    pub qty: u32,
    pub remaining: u32,
    pub limit_price: Cents,
    pub placed_tick: u32,
    pub expires_tick: u32,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BookView {
    pub clock: Clock,
    pub instrument: String,
    pub last_price: Option<Cents>,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
    pub my_orders: Vec<OrderView>,
    /// Recent trades, oldest first.
    pub tape: Vec<EventRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PlaceOrderRequest {
    pub instrument: String,
    /// `bid` or `ask`.
    pub side: String,
    pub qty: u32,
    pub limit_price: Cents,
    pub expires_tick: Option<u32>,
    /// A manager placing for an org.
    pub on_behalf_of: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PricePoint {
    pub tick: u32,
    pub instrument: String,
    pub vwap: Cents,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PricesView {
    pub clock: Clock,
    pub window: u32,
    pub points: Vec<PricePoint>,
}

// -- orgs ----------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkerView {
    pub citizen: u32,
    pub handle: String,
    pub hours: u8,
    /// What the org can count so far this cycle (attribution noise applied);
    /// shown to the worker and the manager. True per-tick figures are in the
    /// `Produced` events (`/explain/{seq}`), where the viewer rules apply.
    pub attributed_this_cycle: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkplaceView {
    pub id: u32,
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub slot: Option<u32>,
    pub machines: u32,
    pub cycle_output: f64,
    pub workers: Vec<WorkerView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrgView {
    pub id: u32,
    #[schema(value_type = String)]
    pub kind: OrgKind,
    pub name: String,
    pub manager: Option<u32>,
    pub treasury: Cents,
    #[schema(value_type = Object)]
    pub inventory: BTreeMap<Good, u32>,
    #[schema(value_type = Object)]
    pub ownership: serde_json::Value,
    pub book_value: Cents,
    pub payment_missed: bool,
    pub employees: u32,
    pub members: Vec<u32>,
    pub workplaces: Vec<WorkplaceView>,
    pub my_shares: u64,
    pub i_manage: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrgsView {
    pub clock: Clock,
    pub orgs: Vec<OrgView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FirstWorkplace {
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub slot: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FoundOrgRequest {
    #[schema(value_type = String)]
    pub kind: OrgKind,
    pub name: String,
    pub first_workplace: Option<FirstWorkplace>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct EmploymentOfferRequest {
    pub workplace: u32,
    /// `{"hourly": cents}` or `{"piece_rate": cents}`.
    #[schema(value_type = Object)]
    pub pay: Pay,
    pub max_hours: u8,
    pub term_cycles: Option<u32>,
    pub notice_cycles: u32,
    pub places: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AddWorkplaceRequest {
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub slot: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MachinesRequest {
    pub workplace: u32,
    pub qty: u32,
    /// `install` or `uninstall`.
    pub action: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DividendRequest {
    pub per_share: Cents,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct IssueSharesRequest {
    pub qty: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AppointRequest {
    pub citizen: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MemberRequest {
    pub citizen: u32,
}

// -- offers, contracts, transfers ----------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OfferView {
    pub id: u32,
    #[schema(value_type = Object)]
    pub by: serde_json::Value,
    pub created_tick: u32,
    pub kind: String,
    #[schema(value_type = Object)]
    pub body: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NoticeBoardView {
    pub clock: Clock,
    pub offers: Vec<OfferView>,
}

/// `POST /s/{id}/offers/sale`. Asset and price are engine shapes:
/// `{"good": ["food", 10]}`, `{"shares": [3, 20]}`, `{"dwelling": 7}`;
/// `{"money": cents}` or `{"good": ["grain", 4]}`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SaleOfferRequest {
    #[schema(value_type = Object)]
    pub asset: SaleAsset,
    #[schema(value_type = Object)]
    pub price: Price,
    #[schema(value_type = Option<Object>)]
    pub to: Option<Party>,
    pub on_behalf_of: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WantedRequest {
    #[schema(value_type = String)]
    pub good: Good,
    pub qty: u32,
    pub max_price: Cents,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreditOfferRequest {
    #[schema(value_type = Option<Object>)]
    pub to: Option<Party>,
    pub principal: Cents,
    pub rate_per_cycle_bp: u32,
    pub term_cycles: u32,
    #[schema(value_type = Option<Object>)]
    pub collateral: Option<Collateral>,
    pub on_behalf_of: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct LeaseOfferRequest {
    #[schema(value_type = Object)]
    pub asset: LeaseAsset,
    pub rent_per_cycle: Cents,
    pub term_cycles: Option<u32>,
    pub on_behalf_of: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AcceptRequest {
    /// A manager accepting for an org (e.g. a firm taking credit).
    pub on_behalf_of: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContractsView {
    pub clock: Clock,
    pub contracts: Vec<ContractView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransferRequest {
    /// `{"citizen": id}` or `{"org": id}`.
    #[schema(value_type = Object)]
    pub to: Party,
    /// `{"money": cents}` or `{"good": ["food", 3]}`.
    #[schema(value_type = Object)]
    pub asset: Asset,
    pub memo: String,
    pub on_behalf_of: Option<u32>,
}

// -- society -------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StatsView {
    pub clock: Clock,
    /// The last `CycleClosed.aggregates` (engine shape), if a cycle has closed.
    #[schema(value_type = Option<Object>)]
    pub last_cycle: Option<serde_json::Value>,
    pub live: SocietyPulse,
    pub firm_count: u32,
    pub credit_outstanding: Cents,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CitizenPublic {
    pub id: u32,
    pub handle: String,
    pub kind: String,
    pub dormant: bool,
    pub joined_tick: u32,
    #[schema(value_type = Object)]
    pub flags: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CitizensView {
    pub clock: Clock,
    pub citizens: Vec<CitizenPublic>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FirmValuation {
    pub org: u32,
    pub name: String,
    pub book_value: Cents,
    pub shares_held: u64,
    pub shares_issued: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScoreRow {
    pub citizen: u32,
    pub handle: String,
    pub net_worth: Cents,
    pub self_made: Cents,
    pub firms: Vec<FirmValuation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScoreboardView {
    pub clock: Clock,
    pub rows: Vec<ScoreRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HouseholdersView {
    pub clock: Clock,
    /// `docs/SCRIPT.md`: the published householder script.
    pub markdown: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ExplainView {
    pub clock: Clock,
    pub event: EventRef,
    /// Every `Explain` payload inside the event, as the viewer may see it.
    #[schema(value_type = Vec<Object>)]
    pub explains: Vec<serde_json::Value>,
}

/// One WebSocket frame on `/s/{id}/stream`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StreamFrame {
    pub clock: Clock,
    pub events: Vec<EventRef>,
    /// Set when the subscriber fell behind and frames were dropped.
    pub lagged: Option<u64>,
}

// -- helpers -------------------------------------------------------------------

/// `food` or `share:3` -> engine instrument.
#[must_use]
pub fn parse_instrument(s: &str) -> Option<isms_core::world::Instrument> {
    if let Some(org) = s.strip_prefix("share:") {
        return org
            .parse::<u32>()
            .ok()
            .map(|n| isms_core::world::Instrument::Share(OrgId(n)));
    }
    serde_json::from_value::<Good>(serde_json::Value::String(s.to_owned()))
        .ok()
        .map(isms_core::world::Instrument::Good)
}

#[must_use]
pub fn instrument_name(i: isms_core::world::Instrument) -> String {
    match i {
        isms_core::world::Instrument::Good(g) => serde_json::to_value(g)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        isms_core::world::Instrument::Share(org) => format!("share:{}", org.0),
    }
}

#[must_use]
pub fn cents(m: Money) -> Cents {
    m.0
}

#[must_use]
pub fn citizen_id(n: u32) -> CitizenId {
    CitizenId(n)
}
#[must_use]
pub fn org_id(n: u32) -> OrgId {
    OrgId(n)
}
#[must_use]
pub fn workplace_id(n: u32) -> WorkplaceId {
    WorkplaceId(n)
}
#[must_use]
pub fn slot_id(n: u32) -> SlotId {
    SlotId(n)
}
#[must_use]
pub fn dwelling_id(n: u32) -> DwellingId {
    DwellingId(n)
}
