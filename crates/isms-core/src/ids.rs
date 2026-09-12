//! Typed identifiers. All are `u32` newtypes with transparent serialization so the
//! event log stays small and canonical; `BTreeMap` keys iterate in id order.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id {
    ($(#[$m:meta])* $name:ident, $prefix:literal) => {
        $(#[$m])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u32);

        impl $name {
            /// The next id in sequence.
            #[must_use]
            pub const fn next(self) -> Self {
                Self(self.0 + 1)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }
    };
}

id!(/// A citizen (human or householder) within one society.
    CitizenId, "c");
id!(/// An organization: firm, cooperative, collective, state enterprise, union, association.
    OrgId, "o");
id!(/// A production unit owned by an org.
    WorkplaceId, "w");
id!(/// A dwelling asset with identity (not a fungible good).
    DwellingId, "d");
id!(/// A land slot for a slot-limited workplace kind.
    SlotId, "s");
id!(/// A contract of any kind (GDD §7.2).
    ContractId, "k");
id!(/// A notice-board offer that becomes a contract on acceptance.
    OfferId, "f");
id!(/// An order resting on an order book.
    OrderId, "r");
id!(/// A governance proposal (Phase 2).
    ProposalId, "p");

/// Tick counter, 0-based within an epoch. `cycle = tick / ticks_per_cycle`.
pub type Tick = u32;
/// Cycle index, 0-based within an epoch.
pub type Cycle = u32;
/// Epoch index, 0-based within a society.
pub type Epoch = u32;
