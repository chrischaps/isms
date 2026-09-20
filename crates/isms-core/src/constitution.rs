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

/// The kinds of proposal a constitution enables (GDD §8.1; TDD §5.2). The tag
/// is the gate; the payload lives on `world::ProposalKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKindTag {
    /// A cooperative's members admit a candidate (S0.17c).
    Admission,
    /// A patch to the society's policy (S2.1).
    PolicyChange,
    /// Free text, recorded as the assembly's minutes (S2.1, Q119).
    Resolution,
    /// Declared in S2.1; the office effects arrive with S2.2.
    Election,
    Recall,
    /// Declared in S2.1; the honor's effect arrives with S2.3.
    Honor,
    /// Declared in S2.1; the member-owned org's vote arrives with S2.4.
    Disbursement,
}

/// Who may open a proposal (GDD §8.1: "anyone / office-holders only").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Proposers {
    #[default]
    Anyone,
    OfficeHolders,
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
    /// The proposal kinds the society's assembly, legislature or committee
    /// may open; empty where governance is `none` (S2.1).
    #[serde(default)]
    pub proposal_kinds: BTreeSet<ProposalKindTag>,
    #[serde(default)]
    pub proposers: Proposers,
}

impl Constitution {
    /// Money exists unless pricing is `none` (GDD §5).
    #[must_use]
    pub const fn has_money(&self) -> bool {
        !matches!(self.pricing, Pricing::None)
    }
}
