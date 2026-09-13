//! `Policy`: the parameters a society's own governance may move within its
//! constitution (GDD Q10). Each preset sets the fields its system uses; the
//! rest stay `None`. Validation against the constitution is in `config`.

// serde `with` modules must take `&Option<T>`; the lint does not apply.
#![allow(clippy::ref_option)]

use crate::kinds::Good;
use crate::money::{Money, credits_map};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Monitoring as a policy override (TDD T14): every preset carries this so the
/// Commune's vote is an ordinary policy patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitoringPolicy {
    Inherit,
    High,
    Medium,
    Low,
}

/// Common Store rationing rule (GDD §6.2, TDD T8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rationing {
    NeedFirst,
    EqualShortfall,
    Lottery,
}

/// How the Public Investment Bank decides (GDD §6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LendingRule {
    Formula,
    Vote,
}

/// The Materials split (Wares / Machines / Dwellings) as fractions summing to 1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialsSplit {
    pub wares: f64,
    pub machines: f64,
    pub dwellings: f64,
}

/// A tax bracket: income above `above` (cents per cycle) is taxed at `rate`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxBracket {
    pub above: Money,
    pub rate: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub monitoring: MonitoringPolicy,

    // Commune (GDD §6.2)
    #[serde(default)]
    pub work_norm_hours: Option<u8>,
    #[serde(default)]
    pub rationing: Option<Rationing>,
    // Commune and Directorate
    #[serde(default)]
    pub materials_split: Option<MaterialsSplit>,

    // Directorate (GDD §6.3)
    #[serde(default)]
    pub plan_bonus_fraction: Option<f64>,
    #[serde(default)]
    pub ratchet: Option<bool>,
    #[serde(default, rename = "price_list_credits", with = "opt_credits_map")]
    pub price_list: Option<BTreeMap<Good, Money>>,
    #[serde(default, rename = "wage_grades_credits", with = "opt_credits_vec")]
    pub wage_grades: Option<Vec<Money>>,
    /// Ration cards: an equal cap per citizen per cycle on a good (GDD §6.3, Q66).
    #[serde(default)]
    pub ration_caps: Option<BTreeMap<Good, u32>>,
    // Directorate and Commonwealth
    #[serde(default)]
    pub minimum_food_ration: Option<u32>,

    // Republic (GDD §6.4)
    #[serde(default)]
    pub tax_rate: Option<f64>,
    #[serde(default)]
    pub tax_brackets: Option<Vec<TaxBracket>>,
    #[serde(default)]
    pub need_floor_food: Option<u32>,
    #[serde(default, rename = "minimum_wage_credits", with = "opt_credits")]
    pub minimum_wage: Option<Money>,
    #[serde(default)]
    pub public_dwellings: Option<u32>,
    #[serde(default)]
    pub public_bank: Option<bool>,

    // Commonwealth (GDD §6.5)
    #[serde(default)]
    pub capital_levy: Option<f64>,
    #[serde(default)]
    pub lending_rule: Option<LendingRule>,
}

impl Policy {
    /// Whether every field set is one the constitution's system uses, and the
    /// values are well-formed (S0.15b). Config loading and `SetPolicy` both
    /// call this, so a society can never hold a policy its axes do not have.
    pub fn validate_against(&self, c: &crate::constitution::Constitution) -> Result<(), String> {
        use crate::constitution::{CapitalMode, LaborMode, Ownership, Pricing, Redistribution};
        let only = |set: bool, ok: bool, what: &str| {
            if set && !ok {
                Err(format!("{what} is not a policy of this constitution"))
            } else {
                Ok(())
            }
        };
        only(
            self.rationing.is_some(),
            c.pricing == Pricing::None,
            "rationing",
        )?;
        only(
            self.work_norm_hours.is_some(),
            c.labor == LaborMode::Norm,
            "work_norm_hours",
        )?;
        only(
            self.materials_split.is_some(),
            c.ownership == Ownership::Collective,
            "materials_split",
        )?;
        let administered = c.pricing == Pricing::Administered;
        only(self.price_list.is_some(), administered, "price_list")?;
        only(self.wage_grades.is_some(), administered, "wage_grades")?;
        only(self.ration_caps.is_some(), administered, "ration_caps")?;
        only(
            self.plan_bonus_fraction.is_some(),
            administered,
            "plan_bonus_fraction",
        )?;
        only(self.ratchet.is_some(), administered, "ratchet")?;
        let tax = c.redistribution == Redistribution::TaxTransfer;
        only(self.tax_rate.is_some(), tax, "tax_rate")?;
        only(self.tax_brackets.is_some(), tax, "tax_brackets")?;
        only(self.need_floor_food.is_some(), tax, "need_floor_food")?;
        only(self.minimum_wage.is_some(), tax, "minimum_wage")?;
        only(self.public_dwellings.is_some(), tax, "public_dwellings")?;
        only(self.public_bank.is_some(), tax, "public_bank")?;
        let bank = c.capital == CapitalMode::PublicBank;
        only(self.capital_levy.is_some(), bank, "capital_levy")?;
        only(self.lending_rule.is_some(), bank, "lending_rule")?;
        only(
            self.minimum_food_ration.is_some(),
            c.redistribution == Redistribution::Provision,
            "minimum_food_ration",
        )?;
        if let Some(split) = self.materials_split
            && (split.wares + split.machines + split.dwellings - 1.0).abs() > 1e-9
        {
            return Err("materials_split must sum to 1".into());
        }
        if let Some(g) = &self.wage_grades
            && (g.is_empty() || g.windows(2).any(|w| w[0] > w[1]))
        {
            return Err("wage_grades must be non-empty and ascending".into());
        }
        if self.tax_rate.is_some_and(|r| !(0.0..=1.0).contains(&r))
            || self
                .tax_brackets
                .as_ref()
                .is_some_and(|b| b.iter().any(|x| !(0.0..=1.0).contains(&x.rate)))
            || self.capital_levy.is_some_and(|r| !(0.0..=1.0).contains(&r))
            || self.plan_bonus_fraction.is_some_and(|r| r < 0.0)
        {
            return Err("rates must lie in 0..=1".into());
        }
        Ok(())
    }
}

mod opt_credits {
    use crate::money::{Money, credits};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Money>, D::Error> {
        #[derive(Deserialize)]
        struct W(#[serde(with = "credits")] Money);
        Option::<W>::deserialize(d).map(|o| o.map(|w| w.0))
    }

    pub fn serialize<S: Serializer>(v: &Option<Money>, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct W<'a>(#[serde(with = "credits")] &'a Money);
        match v {
            Some(m) => s.serialize_some(&W(m)),
            None => s.serialize_none(),
        }
    }
}

mod opt_credits_vec {
    use crate::money::{Money, credits};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct W(#[serde(with = "credits")] Money);

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<Money>>, D::Error> {
        Option::<Vec<W>>::deserialize(d).map(|o| o.map(|v| v.into_iter().map(|w| w.0).collect()))
    }

    pub fn serialize<S: Serializer>(v: &Option<Vec<Money>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(v) => s.serialize_some(&v.iter().map(|m| W(*m)).collect::<Vec<_>>()),
            None => s.serialize_none(),
        }
    }
}

mod opt_credits_map {
    use super::credits_map;
    use crate::kinds::Good;
    use crate::money::Money;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<BTreeMap<Good, Money>>, D::Error> {
        #[derive(Deserialize)]
        struct W(#[serde(with = "credits_map")] BTreeMap<Good, Money>);
        Option::<W>::deserialize(d).map(|o| o.map(|w| w.0))
    }

    pub fn serialize<S: Serializer>(
        v: &Option<BTreeMap<Good, Money>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct W<'a>(#[serde(with = "credits_map")] &'a BTreeMap<Good, Money>);
        match v {
            Some(m) => s.serialize_some(&W(m)),
            None => s.serialize_none(),
        }
    }
}
