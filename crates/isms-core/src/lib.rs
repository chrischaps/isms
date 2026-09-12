//! `isms-core` — the pure, deterministic, event-sourced simulation engine.
//!
//! No I/O beyond reading preset files in [`config`], no async, no clock, no
//! unseeded randomness. State changes only by applying events.
//! See `docs/tdd.md` §0 and §5.

pub mod capabilities;
pub mod config;
pub mod constitution;
pub mod ids;
pub mod kinds;
pub mod lexicon;
pub mod money;
pub mod params;
pub mod policy;

pub use capabilities::Capabilities;
pub use config::{ConfigError, Preset, load_preset};
pub use constitution::Constitution;
pub use money::Money;
pub use params::Params;
pub use policy::Policy;

/// Path to the workspace `presets/` directory, resolved at compile time.
/// Used by tests and the headless simulator; the server embeds presets instead.
pub const WORKSPACE_PRESETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../presets");
