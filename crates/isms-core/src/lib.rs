//! `isms-core`: the pure, deterministic, event-sourced simulation engine.
//!
//! No I/O beyond reading preset files in [`config`], no async, no clock, no
//! unseeded randomness. State changes only by applying events.
//! See `docs/tdd.md` sections 0 and 5.

pub mod apply;
pub mod bank;
pub mod capabilities;
pub mod command;
pub mod config;
pub mod constitution;
pub mod coop;
pub mod coordinator;
pub mod credit;
pub mod employment;
pub mod event;
pub mod explain;
pub mod governance;
pub mod householder;
pub mod housing;
pub mod ids;
pub mod kinds;
pub mod labor;
pub mod ledger;
pub mod lexicon;
pub mod market;
pub mod metrics;
pub mod money;
pub mod needs;
pub mod norms;
pub mod offices;
pub mod orgs;
pub mod params;
pub mod plan;
pub mod planner;
pub mod planning;
pub mod policy;
pub mod rules;
pub mod seeding;
pub mod shares;
pub mod state_store;
pub mod store;
pub mod tax;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod tick;
pub mod transfers;
pub mod union;
pub mod world;

pub use apply::apply;
pub use capabilities::Capabilities;
pub use command::{Command, Envelope, Reject, RejectCode, handle};
pub use config::{ConfigError, Preset, load_preset, load_preset_with_overrides};
pub use constitution::Constitution;
pub use event::Event;
pub use explain::{Explain, RuleId};
pub use ledger::{Asset, Holder, Party, conservation_check};
pub use money::Money;
pub use params::Params;
pub use policy::Policy;
pub use rules::Rules;
pub use tick::{TickError, TickInput, start_epoch, tick};
pub use world::World;

/// Path to the workspace `presets/` directory, resolved at compile time.
/// Used by tests and the headless simulator; the server embeds presets instead.
pub const WORKSPACE_PRESETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../presets");
