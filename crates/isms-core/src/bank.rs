//! The Public Investment Bank (GDD §6.5; S0.17c). In S0.17b only the levy
//! hook exists so the coop's obligations can reserve it; the levy itself, the
//! bank org and its lending arrive with S0.17c.

use crate::ids::OrgId;
use crate::money::Money;
use crate::world::World;

/// The capital levy due from `org` at this cycle end (S0.17c); zero until then.
#[must_use]
pub fn levy_due(_world: &World, _org: OrgId) -> Money {
    Money::ZERO
}
