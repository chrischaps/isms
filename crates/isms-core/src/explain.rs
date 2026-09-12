//! Every number a rule produces carries an `Explain` (TDD §5.6): a closed rule id,
//! named inputs, the formula, and the result. The UI renders these verbatim.

use crate::money::Money;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A display-typed number.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Num {
    Int(i64),
    Money(Money),
    Float(f64),
}

impl From<i64> for Num {
    fn from(v: i64) -> Self {
        Num::Int(v)
    }
}
impl From<u32> for Num {
    fn from(v: u32) -> Self {
        Num::Int(i64::from(v))
    }
}
impl From<Money> for Num {
    fn from(v: Money) -> Self {
        Num::Money(v)
    }
}
impl From<f64> for Num {
    fn from(v: f64) -> Self {
        Num::Float(v)
    }
}

/// The closed set of rules that produce player-visible numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleId {
    LaborOutput,
    LaborAttribution,
    OutputMultiplier,
    SkillMult,
    CapitalMult,
    PayHourly,
    PayPieceRate,
    PayScale,
    PayShare,
    PlanBonus,
    StoreDrawNeedFirst,
    StoreDrawEqualShortfall,
    StoreDrawLottery,
    TaxIncome,
    NeedFloorTransfer,
    DividendPerShare,
    CreditInstallment,
    Rent,
    FoundingFee,
    Endowment,
    Emigration,
    Depreciation,
}

impl RuleId {
    /// The GDD section and one plain sentence, for the Explain popover.
    #[must_use]
    pub const fn doc(self) -> (&'static str, &'static str) {
        match self {
            RuleId::LaborOutput => (
                "GDD §4.3",
                "Output per hour is the workplace base rate times skill, effort, and capital multipliers.",
            ),
            RuleId::LaborAttribution => (
                "GDD §4.3",
                "What the manager sees is your true output with monitoring noise applied.",
            ),
            RuleId::OutputMultiplier => (
                "GDD §4.2",
                "Low Food and being unhoused reduce output, down to a floor.",
            ),
            RuleId::SkillMult => (
                "GDD §4.3",
                "Skill grows with hours in a job family and multiplies output.",
            ),
            RuleId::CapitalMult => (
                "GDD §4.4",
                "Machines per worker raise output with diminishing returns.",
            ),
            RuleId::PayHourly => ("GDD §7.2", "Hourly wages pay for hours worked, not output."),
            RuleId::PayPieceRate => (
                "GDD §7.2",
                "Piece rates pay for attributed output, not hours.",
            ),
            RuleId::PayScale => (
                "GDD §5 A3",
                "The wage grade table pays by job family and skill band.",
            ),
            RuleId::PayShare => (
                "GDD §6.5",
                "The coop's surplus is divided among its members.",
            ),
            RuleId::PlanBonus => (
                "GDD §6.3",
                "A workplace that meets its target pays a bonus.",
            ),
            RuleId::StoreDrawNeedFirst => (
                "GDD §6.2",
                "When the Store is short, the largest shortfall is served first.",
            ),
            RuleId::StoreDrawEqualShortfall => (
                "GDD §6.2",
                "When the Store is short, everyone's shortfall is cut equally.",
            ),
            RuleId::StoreDrawLottery => (
                "GDD §6.2",
                "When the Store is short, draws are decided by lot.",
            ),
            RuleId::TaxIncome => (
                "GDD §6.4",
                "Income tax on the cycle's income funds the treasury.",
            ),
            RuleId::NeedFloorTransfer => (
                "GDD §6.4",
                "The treasury tops up citizens below the need floor.",
            ),
            RuleId::DividendPerShare => {
                ("GDD §6.1", "A declared dividend pays each share equally.")
            }
            RuleId::CreditInstallment => (
                "GDD §7.2",
                "A loan repays principal plus simple interest in equal installments.",
            ),
            RuleId::Rent => ("GDD §7.2", "A lease debits rent to the owner each cycle."),
            RuleId::FoundingFee => ("GDD §6.1", "Founding a firm costs money and Materials."),
            RuleId::Endowment => (
                "GDD §4.5",
                "Every citizen joins with the same money endowment.",
            ),
            RuleId::Emigration => (
                "GDD §11.3",
                "A departing householder's assets leave the economy.",
            ),
            RuleId::Depreciation => ("GDD §4.4", "Machines wear out at a fixed rate per cycle."),
        }
    }
}

/// The explanation attached to a rule-produced value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Explain {
    pub rule: RuleId,
    pub inputs: Vec<(Cow<'static, str>, Num)>,
    pub formula: Cow<'static, str>,
    pub result: Num,
}

impl Explain {
    #[must_use]
    pub fn new(rule: RuleId, formula: &'static str, result: impl Into<Num>) -> Self {
        Explain {
            rule,
            inputs: Vec::new(),
            formula: Cow::Borrowed(formula),
            result: result.into(),
        }
    }

    #[must_use]
    pub fn input(mut self, name: &'static str, value: impl Into<Num>) -> Self {
        self.inputs.push((Cow::Borrowed(name), value.into()));
        self
    }
}
