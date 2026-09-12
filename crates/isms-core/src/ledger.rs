//! Holders, assets, and conservation (TDD §5.8, ADR-0004, ADR-0005).
//!
//! Every unit of money and every good is held by exactly one [`Holder`]. Money
//! enters through `minted` and leaves through `burned`; goods enter through
//! `produced` or `seeded` and leave through `consumed`, `depreciated`, or
//! `burned`. `conservation_check` asserts the books balance exactly.

use crate::ids::{CitizenId, ContractId, OrderId, OrgId, WorkplaceId};
use crate::kinds::Good;
use crate::money::Money;
use crate::world::{Ownership, ShareHolder, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A citizen or an org: the parties that can act and own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Party {
    Citizen(CitizenId),
    Org(OrgId),
}

/// Anywhere money or goods can sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Holder {
    Citizen(CitizenId),
    Org(OrgId),
    /// Machines installed at a workplace.
    Workplace(WorkplaceId),
    OrderEscrow(OrderId),
    ContractEscrow(ContractId),
    /// The Common Store (moneyless systems).
    Store,
    /// The state's stock (administered systems).
    StateStock,
    /// The tax treasury (tax-transfer systems).
    Treasury,
}

impl From<Party> for Holder {
    fn from(p: Party) -> Self {
        match p {
            Party::Citizen(c) => Holder::Citizen(c),
            Party::Org(o) => Holder::Org(o),
        }
    }
}

/// A quantity of money or of one good.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Asset {
    Money(Money),
    Good(Good, u32),
}

/// Running totals of what has entered and left the economy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerMeta {
    pub minted: Money,
    pub burned_money: Money,
    pub produced: BTreeMap<Good, u64>,
    pub seeded: BTreeMap<Good, u64>,
    pub consumed: BTreeMap<Good, u64>,
    pub depreciated: BTreeMap<Good, u64>,
    pub burned: BTreeMap<Good, u64>,
}

impl LedgerMeta {
    pub fn add(map: &mut BTreeMap<Good, u64>, good: Good, qty: u32) {
        *map.entry(good).or_insert(0) += u64::from(qty);
    }

    #[must_use]
    pub fn expected_goods(&self, good: Good) -> i128 {
        let g = |m: &BTreeMap<Good, u64>| i128::from(m.get(&good).copied().unwrap_or(0));
        g(&self.produced) + g(&self.seeded)
            - g(&self.consumed)
            - g(&self.depreciated)
            - g(&self.burned)
    }
}

/// A conservation violation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Imbalance {
    #[error("money: holders sum to {actual} but minted - burned is {expected}")]
    Money { expected: Money, actual: Money },
    #[error("{good:?}: holders sum to {actual} but ledger expects {expected}")]
    Good {
        good: Good,
        expected: i128,
        actual: i128,
    },
    #[error("negative balance at {holder:?}: {amount}")]
    NegativeMoney { holder: Holder, amount: Money },
    #[error("org {org}: shares held {held} + escrowed {escrowed} != issued {issued}")]
    Shares {
        org: OrgId,
        held: u64,
        escrowed: u64,
        issued: u64,
    },
}

fn add_goods(goods: &mut BTreeMap<Good, i128>, map: &BTreeMap<Good, u32>) {
    for (g, q) in map {
        *goods.entry(*g).or_insert(0) += i128::from(*q);
    }
}

/// Sum every holder of money and goods, per good.
#[must_use]
pub fn holdings(world: &World) -> (Money, BTreeMap<Good, i128>) {
    let mut money = Money::ZERO;
    let mut goods: BTreeMap<Good, i128> = BTreeMap::new();
    for c in world.citizens.values() {
        money += c.household.balance;
        add_goods(&mut goods, &c.household.pantry);
    }
    for o in world.orgs.values() {
        money += o.treasury;
        add_goods(&mut goods, &o.inventory);
    }
    for w in world.workplaces.values() {
        *goods.entry(Good::Machines).or_insert(0) += i128::from(w.machines);
    }
    for e in world.escrow.values() {
        match e {
            Asset::Money(m) => money += *m,
            Asset::Good(g, q) => *goods.entry(*g).or_insert(0) += i128::from(*q),
        }
    }
    if let Some(store) = &world.store {
        add_goods(&mut goods, &store.stock);
    }
    if let Some(stock) = &world.state_stock {
        add_goods(&mut goods, &stock.stock);
        money += stock.till;
    }
    money += world.treasury;
    (money, goods)
}

/// Assert the books balance (TDD §5.8). Runs after every event in tests.
pub fn conservation_check(world: &World) -> Result<(), Imbalance> {
    for c in world.citizens.values() {
        if c.household.balance.is_negative() {
            return Err(Imbalance::NegativeMoney {
                holder: Holder::Citizen(c.id),
                amount: c.household.balance,
            });
        }
    }
    for o in world.orgs.values() {
        if o.treasury.is_negative() {
            return Err(Imbalance::NegativeMoney {
                holder: Holder::Org(o.id),
                amount: o.treasury,
            });
        }
    }
    let (money, goods) = holdings(world);
    let expected = world.ledger_meta.minted - world.ledger_meta.burned_money;
    if money != expected {
        return Err(Imbalance::Money {
            expected,
            actual: money,
        });
    }
    for good in Good::ALL {
        let actual = goods.get(&good).copied().unwrap_or(0);
        let expected = world.ledger_meta.expected_goods(good);
        if actual != expected {
            return Err(Imbalance::Good {
                good,
                expected,
                actual,
            });
        }
    }
    for o in world.orgs.values() {
        if let Ownership::Shares { issued, holdings } = &o.ownership {
            let held: u64 = holdings.values().sum();
            let escrowed: u64 = world
                .share_escrow
                .values()
                .filter(|(org, _)| *org == o.id)
                .map(|(_, q)| *q)
                .sum();
            if held + escrowed != *issued {
                return Err(Imbalance::Shares {
                    org: o.id,
                    held,
                    escrowed,
                    issued: *issued,
                });
            }
        }
    }
    Ok(())
}

/// Whether a citizen or the org itself holds shares; used by control checks.
#[must_use]
pub fn shares_of(ownership: &Ownership, holder: ShareHolder) -> u64 {
    match ownership {
        Ownership::Shares { holdings, .. } => holdings.get(&holder).copied().unwrap_or(0),
        _ => 0,
    }
}
