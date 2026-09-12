//! The eight axes (GDD §5) and the immutable per-epoch `Constitution`.
//! Every setting is a concrete engine rule, never a slider on a vibe.

use crate::kinds::{Channel, ContractKind, OrgKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A1 — ownership of means of production.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Private,
    Cooperative,
    Collective,
}

/// A2 — price formation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pricing {
    Market,
    Administered,
    None,
}

/// A3 — compensation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compensation {
    Contract,
    Scale,
    Share,
    Need,
}

/// A4 — labor allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaborMode {
    Free,
    Assigned,
    Norm,
}

/// A5 — capital markets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapitalMode {
    Open,
    PublicBank,
    None,
}

/// A6 — redistribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Redistribution {
    None,
    TaxTransfer,
    Provision,
    Total,
}

/// A7 — monitoring intensity; the σ per level lives in `Params`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Monitoring {
    High,
    Medium,
    Low,
}

/// A8 — economic governance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Governance {
    None,
    Direct,
    Representative,
    Committee,
}

/// Office kinds (GDD §8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficeKind {
    Coordinator,
    PlanningCommittee,
    Legislator,
    UnionSteward,
    BankBoard,
}

/// Recall threshold for an office.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecallRule {
    Majority,
    TwoThirds,
}

/// An elected office as the constitution defines it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeSpec {
    pub kind: OfficeKind,
    pub seats: u32,
    pub term_cycles: u32,
    /// Whether a holder may serve consecutive terms.
    pub consecutive: bool,
    pub recall: RecallRule,
}

/// The society's constitution: immutable for an epoch (GDD Q10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constitution {
    pub ownership: Ownership,
    pub pricing: Pricing,
    pub compensation: Compensation,
    pub labor: LaborMode,
    pub capital: CapitalMode,
    pub redistribution: Redistribution,
    pub monitoring: Monitoring,
    pub governance: Governance,
    pub contracts: BTreeSet<ContractKind>,
    pub org_kinds: BTreeSet<OrgKind>,
    pub communication: BTreeSet<Channel>,
    #[serde(default)]
    pub offices: Vec<OfficeSpec>,
}

impl Constitution {
    /// Money exists unless pricing is `none` (GDD §5).
    #[must_use]
    pub const fn has_money(&self) -> bool {
        !matches!(self.pricing, Pricing::None)
    }
}
