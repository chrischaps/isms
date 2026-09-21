//! In-society wire types (TDD 10.3, S1.4): the Situation view, plan, labor,
//! market, orgs, contracts, society pages, and command results. Money is
//! always cents on the wire (TDD 10.4). Where a payload is the engine's own
//! serde shape (events, offers, contracts, the standing plan) it travels as a
//! JSON object and the schema says `Object`.

use crate::Clock;
use chrono::{DateTime, Utc};
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
    /// The engine's 0-based tick within the epoch.
    pub tick: u32,
    pub cycle: u32,
    /// 0-based, like `tick` and `cycle` (the `Clock` view is 1-based). Ticks and cycles
    /// restart with every epoch, so a list that spans epochs needs this to place an event.
    pub epoch: u32,
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
    /// The open sale or lease offer on it, if any (S1.15, D6).
    pub offer: Option<u32>,
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
    /// Every position the caller holds, contract or not (S2.7): the norm
    /// systems' positions have no contract and would otherwise not show.
    #[serde(default)]
    pub positions: Vec<PositionView>,
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

/// What moved the treasury (S1.11c): the org's trades, sales, transfers,
/// payroll, dividends and escrows, oldest first, for managers and owners.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrgLedgerView {
    pub clock: Clock,
    pub entries: Vec<EventRef>,
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
    /// The Plan's target for it, units a day (S2.8), and yesterday's output:
    /// the advisory target a Commune worker reads their hours against.
    #[serde(default)]
    pub target: Option<f64>,
    #[serde(default)]
    pub last_cycle_output: f64,
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
    /// Money held in the org's resting bids; the treasury is net of it (S1.15, D5).
    pub escrow: Cents,
    /// Per-share dividend declared this cycle, paid at the day's end (S1.15).
    pub declared_dividend: Option<Cents>,
    pub payment_missed: bool,
    /// The last payday the org could not cover, for its manager and owners on
    /// the single-org view (S1.15, D7); `None` on the list.
    pub last_payment_missed: Option<PaymentMissedView>,
    pub employees: u32,
    pub members: Vec<u32>,
    pub workplaces: Vec<WorkplaceView>,
    /// Dwellings the org owns (a Builder's output), in id order (S1.15, D6).
    pub dwellings: Vec<DwellingView>,
    pub my_shares: u64,
    pub i_manage: bool,
}

/// A payday an org could not cover: who was owed what, and when.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PaymentMissedView {
    pub seq: i64,
    pub epoch: u32,
    pub cycle: u32,
    pub citizen: u32,
    pub handle: String,
    pub owed: Cents,
    pub paid: Cents,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrgsView {
    pub clock: Clock,
    pub orgs: Vec<OrgView>,
    /// Orgs the log remembers and the world no longer holds (an earlier epoch's, or dissolved):
    /// enough to name them where an old payslip or trade still points at one.
    pub former: Vec<FormerOrg>,
    /// What each workplace kind makes, and from what (the preset's recipes): where a good comes from.
    pub recipes: Vec<RecipeView>,
    /// What founding costs here (the client previews it before the command).
    pub founding: FoundingCosts,
    /// Slot scarcity per slot-limited workplace kind; an absent kind is unlimited.
    #[schema(value_type = Object)]
    pub slots: BTreeMap<WorkplaceKind, SlotSummary>,
}

/// One workplace kind's recipe. Names are the wire's `snake_case` kinds and goods;
/// `produces` may be `dwelling`, which is an asset and not a good.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecipeView {
    pub workplace_kind: String,
    pub produces: String,
    /// True when each unit made is one dwelling in the org's `dwellings`,
    /// not a good in its inventory; a workplace's `cycle_output` then counts
    /// finished dwellings (S1.15, D6).
    pub produces_asset: bool,
    /// Inputs used up per unit made.
    pub consumes: BTreeMap<String, u32>,
    /// Units per worker-hour before skill, effort, needs and machines.
    pub base_rate: f64,
}

/// An org that exists only in the log. `epoch` is 0-based, like `EventRef`'s.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FormerOrg {
    pub id: u32,
    pub name: String,
    pub epoch: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FoundingCosts {
    /// The founding fee (zero where money does not exist).
    pub money: Cents,
    /// Materials per workplace, from the founder's pantry (or the org's inventory later).
    pub materials: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SlotSummary {
    pub total: u32,
    pub free: u32,
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
    /// Honors the assembly has conferred (S2.3); 0 where there is no assembly.
    #[serde(default)]
    pub honors: u32,
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
    /// Honors the assembly has conferred (S2.3); the Commune's scoreboard reads it.
    #[serde(default)]
    pub honors: u32,
    /// The contribution record (S2.7), where labor is by norm: the Commune's
    /// scoreboard ranks on it; `None` elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution: Option<ContributionScore>,
}

/// The Ledger of Contribution in three figures (GDD 6.2 Scoreboard).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContributionScore {
    pub hours_total: f64,
    pub days: u32,
    pub norm_met_days: u32,
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

// -- the assembly: proposals, ballots, offices (S2.5) ---------------------------

/// The count on a proposal (engine `Tally`): `cast` includes abstentions and
/// the ballots `vote_default` cast at the close; `quorum` is the ballots the
/// close requires against `eligible` voters (Q117). On an open proposal it is
/// the count so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TallyView {
    pub yes: u32,
    pub no: u32,
    pub abstain: u32,
    pub cast: u32,
    pub quorum: u32,
    pub eligible: u32,
}

/// One ballot on the roll (Q135: the assembly votes openly).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BallotView {
    pub citizen: u32,
    pub handle: String,
    /// `yes`, `no` or `abstain`.
    pub ballot: String,
}

/// How a proposal closed (S2.1) and what it did.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposalOutcome {
    pub passed: bool,
    /// Log position of the `ProposalClosed`.
    pub closed_seq: i64,
    /// The engine's 0-based cycle the close fell in.
    pub closed_cycle: u32,
    pub tally: TallyView,
    /// What the carry did: the `PolicyChanged`, `Honored` or `Disbursed` it
    /// produced, as the viewer may see them.
    pub effects: Vec<EventRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposalView {
    pub id: u32,
    pub by: u32,
    pub by_handle: String,
    pub title: String,
    pub text: String,
    /// The engine's `ProposalKind`: `"resolution"`, `{"policy_change": {"patch": {...}}}`,
    /// `{"honor": {"citizen": n}}`, `{"recall": {"office": "coordinator", "citizen": n}}`,
    /// `{"disbursement": {"org": n, "to": {...}, "asset": {...}}}`, `{"admission": {...}}`.
    #[schema(value_type = Object)]
    pub kind: serde_json::Value,
    /// The kind's tag: `policy_change`, `resolution`, `election`, `recall`,
    /// `honor`, `admission`, `disbursement`.
    pub kind_tag: String,
    /// The org whose members vote, for an admission or a disbursement; absent
    /// for the assembly's own proposals.
    pub org: Option<u32>,
    pub opened_tick: u32,
    /// The engine's 0-based cycle whose end (8j) closes the vote.
    pub closes_cycle: u32,
    pub open: bool,
    pub tally: TallyView,
    /// The roll so far; empty once closed (the tally stands for it).
    pub ballots: Vec<BallotView>,
    /// `yes`, `no`, `abstain` or absent.
    pub my_ballot: Option<String>,
    /// Whether the floor (`assembly:<id>`) takes posts now: while open and for
    /// one cycle after the close (TDD 12).
    pub floor_open: bool,
    /// Present once closed.
    pub outcome: Option<ProposalOutcome>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposalsView {
    pub clock: Clock,
    /// Open proposals the caller may see: the assembly's, and the members'
    /// votes of the orgs they belong to. Oldest first.
    pub open: Vec<ProposalView>,
    /// Closed this epoch, oldest first, with outcomes.
    pub closed: Vec<ProposalView>,
    /// Active humans: the voters a quorum is counted against right now (Q117).
    pub electorate: u32,
    /// `population.quorum_fraction`.
    pub quorum_fraction: f64,
    /// `governance.open_proposals_per_citizen`.
    pub open_per_citizen: u32,
    /// How the caller's standing plan votes for them at the close (`vote_default`).
    #[schema(value_type = Object)]
    pub my_vote_default: serde_json::Value,
}

/// Open a proposal before the assembly (S2.1). `text` may be empty except for
/// a resolution, which is its text.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposeRequest {
    pub title: String,
    #[serde(default)]
    pub text: String,
    /// The engine's `ProposalKind` (see `ProposalView.kind`). An election
    /// cannot be moved: stand for the office instead (Q125). A disbursement
    /// goes through `POST /s/{id}/orgs/{oid}/disbursements`.
    #[schema(value_type = Object)]
    pub kind: isms_core::world::ProposalKind,
}

/// Cast or replace a ballot; the last one before the close counts.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BallotRequest {
    /// `yes`, `no` or `abstain`.
    #[schema(value_type = String)]
    pub ballot: isms_core::world::Ballot,
}

/// A member moves the org's money or goods to a citizen or an org (S2.4,
/// Q122); the members vote through the ballot route.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DisbursementRequest {
    /// `{"citizen": id}` or `{"org": id}`.
    #[schema(value_type = Object)]
    pub to: Party,
    /// `{"money": cents}` or `{"good": ["food", 3]}`.
    #[schema(value_type = Object)]
    pub asset: Asset,
    /// The mover's case, optional.
    #[serde(default)]
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HolderView {
    pub citizen: u32,
    pub handle: String,
    /// The engine's 0-based cycle the term runs through; the seat empties at its end.
    pub term_ends_cycle: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CandidateView {
    pub citizen: u32,
    pub handle: String,
    /// Approval ballots naming this candidate so far.
    pub approvals: u32,
}

/// The election open for an office (S2.2, Q118): approval ballots, the top
/// `seats` win at the close.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ElectionView {
    pub seats: u32,
    /// Engine 0-based cycles.
    pub opened_cycle: u32,
    pub closes_cycle: u32,
    /// In rank order: most approvals, then fewer past terms, then the lower id.
    pub candidates: Vec<CandidateView>,
    /// Approval ballots cast so far (each names any subset of the candidates).
    pub ballots_cast: u32,
    pub my_approvals: Vec<u32>,
    pub i_stand: bool,
    /// Why the caller could not stand right now, in the engine's words with
    /// its ids named; absent when they could.
    pub stand_refusal: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OfficeView {
    /// `coordinator`, `planning_committee`, `legislator`, `union_steward`, `bank_board`.
    pub kind: String,
    pub seats: u32,
    pub term_cycles: u32,
    pub consecutive: bool,
    /// `majority` or `two_thirds`.
    pub recall: String,
    pub holders: Vec<HolderView>,
    pub i_hold: bool,
    pub election: Option<ElectionView>,
    /// The engine's 0-based cycle since which the office has had fewer holders than seats.
    pub short_since: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OfficesView {
    pub clock: Clock,
    pub offices: Vec<OfficeView>,
}

// -- the Coordinator workspace (S2.8) -----------------------------------------------

/// One workplace as the Plan sees it: its target beside what it made.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PlanTargetView {
    pub workplace: u32,
    pub org: u32,
    pub org_name: String,
    /// True for the collective's workplaces, the ones a coordinator may close.
    pub collective: bool,
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub slot: Option<u32>,
    pub workers: u32,
    pub machines: u32,
    /// The published target, units a day; `None` where none was set.
    pub target: Option<f64>,
    /// Units made so far today.
    pub cycle_output: f64,
    /// Yesterday's output and its fulfilment against the target then in force.
    pub last_cycle_output: f64,
    pub last_fulfillment: Option<f64>,
}

/// One land slot: its kind and the workplace on it, if any.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SlotView {
    pub id: u32,
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub workplace: Option<u32>,
}

/// The Plan as published, and the land it is published over (S2.8). The
/// coordinator's workspace is this view plus the offices; every citizen may
/// read it, since the Plan is public.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublishedPlanView {
    pub clock: Clock,
    /// True when the caller sits as Coordinator (the workspace mounts on it).
    pub i_coordinate: bool,
    /// Advisory where governance is direct (GDD 6.2): no bonus, no ratchet.
    pub advisory: bool,
    /// The engine's 0-based cycle the Plan was last published in, and by whom.
    pub published_cycle: Option<u32>,
    pub published_tick: Option<u32>,
    pub published_by: Option<u32>,
    pub targets: Vec<PlanTargetView>,
    /// Every land slot, in id order; kinds absent from the land are unlimited.
    pub slots: Vec<SlotView>,
    /// Workplace kinds with no slot limit.
    #[schema(value_type = Vec<String>)]
    pub unlimited_kinds: Vec<WorkplaceKind>,
    /// The collective a coordinator opens workplaces for; `None` where there is none.
    pub collective: Option<u32>,
    /// Materials a new workplace costs, and what the Common Store holds.
    pub founding_materials: u32,
    pub store_materials: u32,
    pub max_workplaces: u32,
}

/// The targets to publish, per workplace id, in units a day.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublishPlanRequest {
    pub targets: BTreeMap<u32, f64>,
}

/// Open a workplace of the collective on a free slot of its kind (any, when `slot` is absent).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OpenWorkplaceRequest {
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    pub slot: Option<u32>,
}
/// An approval ballot: any subset of the candidates, the empty set included.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApproveRequest {
    pub candidates: Vec<u32>,
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

// -- epoch archives (S1.15) ----------------------------------------------------

/// A closing statement is at most this many characters.
pub const CLOSING_STATEMENT_MAX_CHARS: usize = 2000;

/// What an ended epoch left behind (GDD §11.5): the engine's frozen summary
/// and the citizens' closing statements, readable by anyone once written.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ArchiveView {
    /// 1-based, as the clock shows it.
    pub epoch: u32,
    /// `scheduled`, `collapse` or `operator`.
    pub reason: String,
    /// The epoch's last day, 1-based.
    pub final_cycle: u32,
    pub ended_at: DateTime<Utc>,
    /// The `EpochEnded` event.
    pub ended_seq: i64,
    /// Closing statements are accepted until this moment; the next epoch starts then.
    pub closes_at: DateTime<Utc>,
    /// Whether a statement written now would be accepted.
    pub open: bool,
    /// The engine's `EpochSummary`: `aggregates` (the last day's `CycleAggregates`)
    /// and `standings` (every citizen ranked by net worth: `citizen`, `handle`,
    /// `kind`, `dormant`, `net_worth`, `self_made`, in cents).
    #[schema(value_type = Object)]
    pub summary: serde_json::Value,
    /// In the order citizens first spoke.
    pub closing_statements: Vec<ClosingStatementView>,
    /// The caller's own statement, on the citizen routes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mine: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClosingStatementView {
    pub citizen: u32,
    pub handle: String,
    pub text: String,
    pub written_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ArchivesView {
    pub clock: Clock,
    /// Oldest first.
    pub archives: Vec<ArchiveView>,
}

/// One per citizen, replaced on every write, until the window closes.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClosingStatementRequest {
    pub text: String,
}

// -- the Common Store and the Ledger of Contribution (S2.7) -----------------------

/// One good on the Store's shelves this tick (GDD 6.2), with the caller's
/// own standing toward it.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StockView {
    #[schema(value_type = String)]
    pub good: Good,
    /// Units on the shelf now.
    pub stock: u32,
    /// Units requested this tick and not yet resolved (phase 6 resolves them).
    pub requested: u32,
    /// Citizens with a request in this tick.
    pub requesters: u32,
    /// What the caller may still draw this tick (`store::entitlement`): the
    /// units that bring the meter to full, less pantry and pending, capped by
    /// pantry room. 0 for a good the Store does not ration by need.
    pub my_entitlement: u32,
    /// The caller's request this tick, not yet served.
    pub my_pending: u32,
    /// Meter tenths one unit restores, for the goods drawn by need; `None` otherwise.
    pub meter_per_unit: Option<u32>,
    /// What each active citizen would get if the day ended now:
    /// `floor(stock / active citizens)` (GDD 6.2, Q54).
    pub share_if_shared_now: u32,
}

/// How one good fared over a whole day: what was asked, what was served.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StoreDayView {
    #[schema(value_type = String)]
    pub good: Good,
    pub requested: u32,
    pub served: u32,
    /// `requested - served`: the units the Store could not find.
    pub short: u32,
    /// Hours in which the rationing rule had to decide (served < requested).
    pub rationed_ticks: u32,
    /// Units shared out equally at the day's end (surplus shares).
    pub shared: u32,
    /// Citizens who received a surplus share.
    pub shared_with: u32,
}

/// The Common Store (GDD 6.2; S2.7): the shelves, the rule in force, the
/// caller's entitlement and pending draw, yesterday's service, and the
/// caller's draw record. Answers 422 `NoStore` where there is no store.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StoreView {
    pub clock: Clock,
    /// The rationing rule in force when the stock runs short:
    /// `need_first`, `equal_shortfall` or `lottery` (`policy.rationing`).
    pub rule: String,
    /// Per good, in the engine's order; the goods drawn by need come first.
    pub stock: Vec<StockView>,
    /// Active citizens: the denominator of a surplus share.
    pub active_citizens: u32,
    /// The engine's 0-based cycle `last_cycle` reports; `None` before a day has closed.
    pub last_cycle: Option<u32>,
    /// Yesterday, per good that anyone asked for or that was shared.
    pub yesterday: Vec<StoreDayView>,
    /// Today so far, per good that anyone has asked for.
    pub today: Vec<StoreDayView>,
    /// The caller's `Drew` events, oldest first (the draw record; GDD 9.1 step 3).
    pub my_draws: Vec<EventRef>,
    #[schema(value_type = Object)]
    pub my_pantry: BTreeMap<Good, u32>,
    #[schema(value_type = Object)]
    pub pantry_capacity: BTreeMap<Good, u32>,
}

/// One citizen's line on the Ledger of Contribution (GDD 6.2): hours exact,
/// output as attributed under the society's monitoring, the norm met or not.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContributionRow {
    pub citizen: u32,
    pub handle: String,
    pub kind: String,
    pub dormant: bool,
    pub is_me: bool,
    /// Hours worked so far today (tick-hours over the day's ticks; exact, a decision).
    pub hours_today: f64,
    /// Output attributed so far today under the monitoring in force: with
    /// σ > 0 this is a noised figure, never the true one (Q55).
    pub attributed_today: f64,
    /// Whether today's hours already reach the norm.
    pub norm_met_today: bool,
    /// The last closed day.
    pub hours_yesterday: f64,
    pub attributed_yesterday: f64,
    /// Every closed day on the record.
    pub days: u32,
    pub hours_total: f64,
    pub attributed_total: f64,
    /// Closed days on which the norm was met.
    pub norm_met_days: u32,
    /// Honors the assembly has conferred (S2.3).
    pub honors: u32,
    /// Where they hold a position today, by workplace id.
    pub workplaces: Vec<u32>,
}

/// The Ledger of Contribution (GDD 6.2; S2.7): every citizen's public record,
/// the norm and the monitoring stated. Answers 422 `NotInThisSociety` where
/// labor is not by norm.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContributionView {
    pub clock: Clock,
    /// The published work norm, hours per day (`policy.work_norm_hours`).
    pub norm_hours: Option<u32>,
    /// The monitoring level in force: `high`, `medium`, `low`.
    pub monitoring: String,
    /// The σ of the attribution noise: 0 means the figures are exact.
    pub sigma: f64,
    /// Active citizens first, by hours today, then by handle; dormant citizens after.
    pub rows: Vec<ContributionRow>,
    /// The workplace with room where labor is scarcest by the balance weights
    /// (`orgs::least_staffed`, Q62): where the norm would send you.
    pub least_staffed: Option<u32>,
    /// Positions per workplace and the cap, for the position picker.
    pub max_workers_per_workplace: u32,
    pub max_workplaces: u32,
}

/// A position the caller holds at a workplace, with or without a contract (S2.7).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PositionView {
    pub workplace: u32,
    pub org: u32,
    pub org_name: String,
    #[schema(value_type = String)]
    pub kind: WorkplaceKind,
    /// The employment contract, where positions come by contract; `None`
    /// for a norm position (`JoinWorkplace`, Q62).
    pub contract: Option<u32>,
}
