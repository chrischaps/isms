//! S0.18: the engine-level invariants hold in every preset from the same seed
//! (GDD §17 item 4; TDD §15 engine properties): money and every good are
//! conserved at every cycle close, no meter leaves 0..=100, no balance and no
//! treasury goes negative, and two runs from one seed produce one world.
//! S2.4 adds the governance invariants where a preset seeds an assembly: at
//! least one `PolicyChanged` from a proposal per epoch, every office seat
//! filled from cycle 1 on, and no proposal open past its `closes_cycle`.
//! The one-epoch check runs in `make check`; the five-epoch, five-seed check
//! is `cargo test -p isms-sim --release -- --ignored invariants`.

use isms_core::event::Event;
use isms_core::ids::{Cycle, Epoch, Tick};
use isms_core::ledger::conservation_check;
use isms_core::money::Money;
use isms_core::needs::FULL;
use isms_core::world::World;
use isms_sim::{Observer, PRESETS, Row, RunSpec, run, run_with};
use std::path::Path;

fn presets() -> &'static Path {
    Path::new(isms_core::WORKSPACE_PRESETS_DIR)
}

/// Checks the world after every cycle close; a failure names the preset,
/// epoch and cycle.
struct Invariants {
    preset: String,
    cycles_checked: u32,
    /// `PolicyChanged { proposal: Some(_) }` seen in the current epoch.
    voted_changes: u32,
}

impl Invariants {
    fn check(&self, world: &World, epoch: Epoch, cycle: Cycle) {
        let at = format!("{} epoch {epoch} cycle {cycle}", self.preset);
        conservation_check(world).unwrap_or_else(|e| panic!("{at}: {e}"));
        // No proposal outlives its closing cycle (S2.4).
        for p in world.proposals.values() {
            assert!(
                p.closes_cycle >= cycle,
                "{at}: proposal {} should have closed at the end of cycle {}",
                p.id,
                p.closes_cycle
            );
        }
        // With an assembly seeded, every seat is filled from cycle 1 (S2.4).
        if world.params.sim.assembly_size > 0 && cycle >= 1 {
            for spec in &world.constitution.offices {
                assert_eq!(
                    world.offices.filled(spec.kind),
                    spec.seats,
                    "{at}: {:?} has empty seats",
                    spec.kind
                );
            }
        }
        for c in world.citizens.values() {
            let n = &c.needs;
            assert!(
                n.food <= FULL && n.shelter <= FULL && n.comfort <= FULL,
                "{at}: {} has a meter above full",
                c.handle
            );
            assert!(
                c.household.balance >= Money::ZERO,
                "{at}: {} is overdrawn",
                c.handle
            );
        }
        for o in world.orgs.values() {
            assert!(o.treasury >= Money::ZERO, "{at}: {} is overdrawn", o.name);
        }
        assert!(
            world.treasury >= Money::ZERO,
            "{at}: the treasury is overdrawn"
        );
        if let Some(s) = &world.state_stock {
            assert!(s.till >= Money::ZERO, "{at}: the till is overdrawn");
        }
    }
}

impl Invariants {
    /// At least one policy change carried by a proposal in the epoch that
    /// just ended, where the preset seeds an assembly (S2.4).
    fn check_epoch_end(&mut self, world: &World, epoch: Epoch) {
        if world.params.sim.assembly_size > 0 {
            assert!(
                self.voted_changes > 0,
                "{} epoch {epoch}: no PolicyChanged from a proposal",
                self.preset
            );
        }
        self.voted_changes = 0;
    }
}

impl Observer for Invariants {
    fn cycle_closed(&mut self, world: &World, epoch: Epoch, cycle: Cycle, _row: &Row) {
        self.check(world, epoch, cycle);
        self.cycles_checked += 1;
    }

    fn tick_done(
        &mut self,
        _world: &World,
        _epoch: Epoch,
        _tick: Tick,
        _round: &[Event],
        tick_events: &[Event],
    ) {
        let n = tick_events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::PolicyChanged {
                        proposal: Some(_),
                        ..
                    }
                )
            })
            .count();
        self.voted_changes += u32::try_from(n).unwrap_or(u32::MAX);
    }

    fn epoch_started(&mut self, world: &World, epoch: Epoch, _events: &[Event]) {
        if epoch > 0 {
            self.check_epoch_end(world, epoch - 1);
        }
        self.check(world, epoch, 0);
    }

    fn finished(&mut self, world: &World) {
        self.check_epoch_end(world, world.meta.epoch);
    }
}

fn check_preset(preset: &str, epochs: u32, seed: u64) {
    let spec = RunSpec::new(preset, epochs, seed);
    let mut obs = Invariants {
        preset: preset.to_owned(),
        cycles_checked: 0,
        voted_changes: 0,
    };
    let a = run_with(presets(), &spec, &mut obs).unwrap_or_else(|e| panic!("{preset}: {e}"));
    assert_eq!(
        obs.cycles_checked,
        epochs * 42,
        "{preset}: one check per cycle"
    );
    assert_eq!(
        a.rejected, 0,
        "{preset}: a rejected scripted command is a bug"
    );
    // Determinism: the same seed gives the same world and the same event count.
    let b = run(presets(), &spec).unwrap();
    assert_eq!(a.world.hash(), b.world.hash(), "{preset}: two runs differ");
    assert_eq!(a.events, b.events);
}

/// Every preset, one epoch, seed 1: conservation, bounds and determinism.
#[test]
fn all_five_presets_hold_engine_invariants() {
    for preset in PRESETS {
        check_preset(preset, 1, 1);
    }
}

/// Every preset, five epochs, seeds 1..=5. Ignored by default: `make sim-all`
/// covers the targets; this covers the invariants at length.
#[test]
#[ignore = "long; cargo test -p isms-sim --release -- --ignored invariants"]
fn all_five_presets_five_epochs_invariants() {
    for preset in PRESETS {
        for seed in 1..=5 {
            check_preset(preset, 5, seed);
        }
    }
}
