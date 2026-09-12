//! Money is an integer count of cents (TDD §5.7). One credit = 100 cents.
//! The engine, API and CLI carry cents; config files and the UI speak credits.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// An amount of a society's money, in cents. May be negative only in deltas.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(pub i64);

/// Cents per credit.
pub const CENTS: i64 = 100;

impl Money {
    pub const ZERO: Money = Money(0);

    #[must_use]
    pub const fn cents(cents: i64) -> Self {
        Money(cents)
    }

    /// Whole credits, no fractional part.
    #[must_use]
    pub const fn credits(credits: i64) -> Self {
        Money(credits * CENTS)
    }

    /// Convert a credits figure from config into cents, requiring it to be exact to the cent.
    pub fn from_credits_f64(credits: f64) -> Result<Self, MoneyError> {
        if !credits.is_finite() {
            return Err(MoneyError::NotFinite);
        }
        #[allow(clippy::cast_precision_loss)]
        let scaled = credits * CENTS as f64;
        let rounded = scaled.round();
        if (scaled - rounded).abs() > 1e-6 {
            return Err(MoneyError::NotWholeCents(credits));
        }
        #[allow(clippy::cast_possible_truncation)]
        Ok(Money(rounded as i64))
    }

    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Non-negative floor for safe displays and tests.
    #[must_use]
    pub const fn max_zero(self) -> Self {
        if self.0 < 0 { Money(0) } else { self }
    }
}

/// Errors converting credits to cents.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MoneyError {
    #[error("money amount is not finite")]
    NotFinite,
    #[error("{0} credits is not a whole number of cents")]
    NotWholeCents(f64),
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}
impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}
impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        self.0 -= rhs.0;
    }
}
impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, Add::add)
    }
}

impl fmt::Debug for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self} cr")
    }
}

impl fmt::Display for Money {
    /// Renders as credits with two decimals, e.g. `1041.19`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.abs();
        write!(f, "{sign}{}.{:02}", abs / CENTS, abs % CENTS)
    }
}

/// Serde helpers for config fields written in credits (`8.00`) but stored in cents.
pub mod credits {
    use super::Money;
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Int(i64),
        Float(f64),
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Money, D::Error> {
        match Raw::deserialize(d)? {
            Raw::Int(i) => Ok(Money::credits(i)),
            Raw::Float(x) => Money::from_credits_f64(x).map_err(D::Error::custom),
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref, clippy::cast_precision_loss)]
    pub fn serialize<S: Serializer>(m: &Money, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(m.0 as f64 / super::CENTS as f64)
    }
}

/// Serde helpers for maps whose values are credits.
pub mod credits_map {
    use super::Money;
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Int(i64),
        Float(f64),
    }

    pub fn deserialize<'de, D, K>(d: D) -> Result<BTreeMap<K, Money>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Ord,
    {
        let raw: BTreeMap<K, Raw> = BTreeMap::deserialize(d)?;
        raw.into_iter()
            .map(|(k, v)| {
                let m = match v {
                    Raw::Int(i) => Money::credits(i),
                    Raw::Float(x) => Money::from_credits_f64(x).map_err(D::Error::custom)?,
                };
                Ok((k, m))
            })
            .collect()
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn serialize<S, K>(map: &BTreeMap<K, Money>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + Ord,
    {
        let as_credits: BTreeMap<&K, f64> = map
            .iter()
            .map(|(k, m)| (k, m.0 as f64 / super::CENTS as f64))
            .collect();
        as_credits.serialize(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_convert_exactly() {
        assert_eq!(Money::from_credits_f64(8.0), Ok(Money(800)));
        assert_eq!(Money::from_credits_f64(1.30), Ok(Money(130)));
        assert_eq!(Money::from_credits_f64(0.6), Ok(Money(60)));
        assert_eq!(
            Money::from_credits_f64(1.005),
            Err(MoneyError::NotWholeCents(1.005))
        );
    }

    #[test]
    fn display_is_credits_with_two_decimals() {
        assert_eq!(Money(104_119).to_string(), "1041.19");
        assert_eq!(Money(-5).to_string(), "-0.05");
        assert_eq!(Money::ZERO.to_string(), "0.00");
    }
}
