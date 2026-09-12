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
