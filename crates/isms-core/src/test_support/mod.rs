//! Shared test harness (TDD §15; plan S0.3b). Every engine card builds on this
//! instead of reinventing fixtures: worlds are built only by emitting events,
//! and every step re-asserts conservation and fold-equals-live.

// Test support favours readable fixtures over the pedantic style lints.
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

pub mod builder;
pub mod fold;
pub mod golden;
pub mod harness;
pub mod strategies;

pub use builder::WorldBuilder;
pub use fold::{assert_fold_equals_live, fold};
pub use golden::check_golden;
pub use harness::Harness;

use crate::config::{Preset, load_preset};
use std::path::Path;

/// Load a preset from the workspace `presets/` directory.
#[must_use]
pub fn preset(name: &str) -> Preset {
    load_preset(Path::new(crate::WORKSPACE_PRESETS_DIR), name)
        .unwrap_or_else(|e| panic!("preset {name}: {e}"))
}

/// Run `run(seed)` twice and assert the event logs are byte-identical.
pub fn assert_deterministic<F>(seed: u64, run: F)
where
    F: Fn(u64) -> Vec<crate::event::Event>,
{
    let a = postcard::to_allocvec(&run(seed)).unwrap();
    let b = postcard::to_allocvec(&run(seed)).unwrap();
    assert!(
        a == b,
        "two runs with seed {seed} produced different event bytes"
    );
}
