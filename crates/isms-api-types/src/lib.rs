//! `isms-api-types`: the wire types of the public API (TDD 10, D9), shared by
//! the server, the CLI, and the `OpenAPI` document. Engine enums are re-exported
//! where the wire shape is the engine's own serde form.

use chrono::{DateTime, Utc};
use isms_core::Capabilities;
use isms_core::constitution::{
    CapitalMode, Compensation, Governance, LaborMode, Monitoring, OfficeSpec, Redistribution,
};
use isms_core::kinds::{ContractKind, OrgKind, WorkplaceKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use utoipa::ToSchema;

pub use isms_core::command::RejectCode;
pub use isms_core::kinds::ClientKind;

/// Where the society's clock stands. Ticks, cycles and epochs are 1-based
/// for display (TDD 5.1); `engine_tick` is the engine's own 0-based count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Clock {
    pub epoch: u32,
    pub cycle: u32,
    /// From 1 to `ticks_per_cycle`.
    pub tick: u32,
    pub ticks_per_cycle: u32,
    pub engine_tick: u32,
    pub epoch_ended: bool,
}

/// RFC 9457 problem details; `code` carries the engine's `RejectCode` when
/// the engine refused a command.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Problem {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub code: Option<RejectCode>,
}

// -- account -------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MagicLinkRequest {
    pub email: String,
    /// Required the first time an email signs in (Phase 1: invite only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MagicLinkSent {
    pub sent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Account {
    pub id: i64,
    pub email: String,
    pub consent_version: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Citizenship {
    pub society_id: i64,
    pub citizen_id: u32,
    pub handle: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeySummary {
    pub id: i64,
    pub label: String,
    pub prefix: String,
    pub society_id: i64,
    pub citizen_id: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Me {
    pub account: Account,
    pub citizenships: Vec<Citizenship>,
    pub api_keys: Vec<ApiKeySummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NewApiKey {
    pub society_id: i64,
    pub label: String,
}

/// The key is shown once; only its hash is stored.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyCreated {
    pub id: i64,
    pub label: String,
    pub prefix: String,
    pub key: String,
}

// -- societies -----------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SocietySummary {
    pub id: i64,
    pub name: String,
    pub preset: String,
    pub display: String,
    pub status: String,
    pub clock: Clock,
    pub population: u32,
    pub active_humans: u32,
    pub householders: u32,
    /// Wall-clock seconds per tick (0 = as fast as possible).
    pub tick_seconds: u32,
    pub next_tick_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SocietyList {
    pub societies: Vec<SocietySummary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RateLimitView {
    pub per_second: u32,
    pub burst: u32,
}

/// The engine's `Capabilities` (TDD 5.2) on the wire: what exists in this society.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct CapabilitiesView {
    pub clock: Clock,
    pub money: bool,
    pub order_books: bool,
    pub administered_prices: bool,
    pub common_store: bool,
    #[schema(value_type = Vec<String>)]
    pub contracts: BTreeSet<ContractKind>,
    #[schema(value_type = Vec<String>)]
    pub org_kinds: BTreeSet<OrgKind>,
    #[schema(value_type = String)]
    pub labor: LaborMode,
    #[schema(value_type = String)]
    pub pay: Compensation,
    #[schema(value_type = String)]
    pub capital: CapitalMode,
    #[schema(value_type = String)]
    pub redistribution: Redistribution,
    #[schema(value_type = String)]
    pub monitoring: Monitoring,
    pub monitoring_sigma: f64,
    #[schema(value_type = String)]
    pub governance: Governance,
    #[schema(value_type = Vec<Object>)]
    pub offices: Vec<OfficeSpec>,
    /// Slots per workplace kind; `null` = unlimited.
    #[schema(value_type = Object)]
    pub land_slots: BTreeMap<WorkplaceKind, Option<u32>>,
    pub rate_limit: RateLimitView,
}

impl CapabilitiesView {
    #[must_use]
    pub fn from_capabilities(clock: Clock, c: &Capabilities) -> Self {
        CapabilitiesView {
            clock,
            money: c.money,
            order_books: c.order_books,
            administered_prices: c.administered_prices,
            common_store: c.common_store,
            contracts: c.contracts.clone(),
            org_kinds: c.org_kinds.clone(),
            labor: c.labor,
            pay: c.pay,
            capital: c.capital,
            redistribution: c.redistribution,
            monitoring: c.monitoring,
            monitoring_sigma: c.monitoring_sigma,
            governance: c.governance,
            offices: c.offices.clone(),
            land_slots: c.land_slots.clone(),
            rate_limit: RateLimitView {
                per_second: c.rate_limit.per_second,
                burst: c.rate_limit.burst,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Lexicon {
    pub clock: Clock,
    pub preset: String,
    pub entries: BTreeMap<String, String>,
}

/// The Welcome Brief (GDD 9.4), Markdown in the society's own voice.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Welcome {
    pub clock: Clock,
    pub markdown: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct JoinRequest {
    pub handle: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Joined {
    pub clock: Clock,
    pub citizen_id: u32,
    pub handle: String,
    /// `false` when the account was already a citizen (join is idempotent).
    pub created: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Health {
    pub ok: bool,
    pub societies: usize,
}
