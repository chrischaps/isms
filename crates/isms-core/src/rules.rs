//! `Rules`: the derived `Capabilities` plus convenient access to the constitution,
//! policy and params (TDD §5.1). Derived once per epoch by the owner of a `World`;
//! `handle` and `tick` take it by reference.

use crate::capabilities::Capabilities;
use crate::world::World;

#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    pub capabilities: Capabilities,
}

impl Rules {
    #[must_use]
    pub fn from_world(world: &World) -> Self {
        Rules {
            capabilities: Capabilities::derive(&world.constitution, &world.policy, &world.params),
        }
    }
}
