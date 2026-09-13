//! A read-only hook into the simulator's loop (S0.14e).
//!
//! `run_with` calls an [`Observer`] after each step has been applied to the
//! world, handing it the events that were applied and the world as it now
//! stands. The default methods are empty, so a run with [`NoObserver`] does
//! exactly what `run` did before this hook existed: nothing is cloned or
//! buffered on its account.

use crate::Row;
use isms_core::event::Event;
use isms_core::ids::{Cycle, Epoch, Tick};
use isms_core::world::World;

/// Callbacks from the simulator's loop; every method has an empty default.
pub trait Observer {
    /// After `start_epoch`'s events are applied (material state was just reset).
    fn epoch_started(&mut self, _world: &World, _epoch: Epoch, _events: &[Event]) {}

    /// After both the householder round and the tick's own events are applied.
    /// `tick` is the tick just resolved; `world.meta.tick` is already `tick + 1`.
    /// Householder orders can fill inside the round, so `Trade` events appear
    /// in either slice.
    fn tick_done(
        &mut self,
        _world: &World,
        _epoch: Epoch,
        _tick: Tick,
        _round: &[Event],
        _tick_events: &[Event],
    ) {
    }

    /// After the aggregate row for `cycle` was pushed (`CycleClosed` applied).
    fn cycle_closed(&mut self, _world: &World, _epoch: Epoch, _cycle: Cycle, _row: &Row) {}

    /// After the last epoch.
    fn finished(&mut self, _world: &World) {}
}

/// The observer `run` uses: sees everything, does nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoObserver;

impl Observer for NoObserver {}
