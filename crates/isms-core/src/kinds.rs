//! Closed enums of the domain. Adding a variant here is a design change, not a tweak.

use serde::{Deserialize, Serialize};

/// The six fungible goods (GDD §4.1). Dwellings are assets, not goods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Good {
    Grain,
    Ore,
    Materials,
    Food,
    Wares,
    Machines,
}

impl Good {
    pub const ALL: [Good; 6] = [
        Good::Grain,
        Good::Ore,
        Good::Materials,
        Good::Food,
        Good::Wares,
        Good::Machines,
    ];
}

/// Anything a workplace can produce: a good, or a dwelling asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Product {
    Grain,
    Ore,
    Materials,
    Food,
    Wares,
    Machines,
    Dwelling,
}

impl Product {
    /// The good this product is, if it is a fungible good.
    #[must_use]
    pub const fn as_good(self) -> Option<Good> {
        match self {
            Product::Grain => Some(Good::Grain),
            Product::Ore => Some(Good::Ore),
            Product::Materials => Some(Good::Materials),
            Product::Food => Some(Good::Food),
            Product::Wares => Some(Good::Wares),
            Product::Machines => Some(Good::Machines),
            Product::Dwelling => None,
        }
    }
}

impl From<Good> for Product {
    fn from(g: Good) -> Self {
        match g {
            Good::Grain => Product::Grain,
            Good::Ore => Product::Ore,
            Good::Materials => Product::Materials,
            Good::Food => Product::Food,
            Good::Wares => Product::Wares,
            Good::Machines => Product::Machines,
        }
    }
}

/// The seven workplace kinds (GDD §4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkplaceKind {
    Farm,
    Mine,
    Foundry,
    Mill,
    Workshop,
    MachineShop,
    Builder,
}

impl WorkplaceKind {
    pub const ALL: [WorkplaceKind; 7] = [
        WorkplaceKind::Farm,
        WorkplaceKind::Mine,
        WorkplaceKind::Foundry,
        WorkplaceKind::Mill,
        WorkplaceKind::Workshop,
        WorkplaceKind::MachineShop,
        WorkplaceKind::Builder,
    ];

    /// The skill family this workplace trains (GDD §4.3).
    #[must_use]
    pub const fn job_family(self) -> JobFamily {
        match self {
            WorkplaceKind::Farm => JobFamily::Farming,
            WorkplaceKind::Mine => JobFamily::Mining,
            WorkplaceKind::Foundry => JobFamily::Smelting,
            WorkplaceKind::Mill => JobFamily::Milling,
            WorkplaceKind::Workshop => JobFamily::Crafting,
            WorkplaceKind::MachineShop => JobFamily::Machining,
            WorkplaceKind::Builder => JobFamily::Building,
        }
    }
}

/// Skill families, one per workplace kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobFamily {
    Farming,
    Mining,
    Smelting,
    Milling,
    Crafting,
    Machining,
    Building,
}

/// The three needs (GDD §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Need {
    Food,
    Shelter,
    Comfort,
}

/// Effort level chosen per labor allocation (GDD Q1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Low,
    Normal,
    High,
}

/// A value per effort level, as written in `_base.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffortTable<T> {
    pub low: T,
    pub normal: T,
    pub high: T,
}

impl<T: Copy> EffortTable<T> {
    #[must_use]
    pub const fn get(&self, effort: Effort) -> T {
        match effort {
            Effort::Low => self.low,
            Effort::Normal => self.normal,
            Effort::High => self.high,
        }
    }
}

/// Organization kinds (GDD §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgKind {
    Firm,
    Cooperative,
    Collective,
    StateEnterprise,
    Union,
    Association,
}

/// Contract kinds (GDD §7.2). Each preset enables a subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    Employment,
    SaleBook,
    SaleDirect,
    Credit,
    PublicCredit,
    Lease,
    Share,
    CollectiveAgreement,
    Pledge,
}

/// Communication channels a society provides (GDD §12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Square,
    Assembly,
    Org,
    Dm,
    NoticeBoard,
    Chronicle,
}

/// Who sent a command (TDD §5.1, QUESTIONS Q6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    Web,
    ApiKey,
    Householder,
    Sim,
    /// Internal commands issued by a citizen's standing plan during the tick.
    Plan,
}

/// Human player or scripted householder (GDD §11.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitizenKind {
    Human,
    Householder,
}
