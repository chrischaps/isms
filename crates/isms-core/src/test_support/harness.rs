//! `Harness`: a live `World` plus its complete event log. After every step it
//! asserts conservation and that folding the log reproduces the live state.

use super::fold::assert_fold_equals_live;
use crate::apply::apply;
use crate::capabilities::Capabilities;
use crate::event::Event;
use crate::ids::CitizenId;
use crate::ledger::conservation_check;
use crate::world::World;

#[derive(Debug)]
pub struct Harness {
    pub world: World,
    pub log: Vec<Event>,
    /// When false, `check()` is skipped after each step (for long proptest runs
    /// that check once at the end).
    pub check_every_step: bool,
}

impl Harness {
    /// Start from a complete log (first event `SocietyCreated`).
    #[must_use]
    pub fn from_log(log: Vec<Event>) -> Self {
        let world = super::fold::fold(&log);
        let h = Harness {
            world,
            log,
            check_every_step: true,
        };
        h.check();
        h
    }

    /// Apply one event and record it.
    pub fn apply(&mut self, event: Event) {
        apply(&mut self.world, &event);
        self.log.push(event);
        if self.check_every_step {
            self.check();
        }
    }

    /// Apply a batch of events and record them.
    pub fn apply_all(&mut self, events: impl IntoIterator<Item = Event>) {
        let was = self.check_every_step;
        self.check_every_step = false;
        for e in events {
            self.apply(e);
        }
        self.check_every_step = was;
        if was {
            self.check();
        }
    }

    /// Conservation plus fold-equals-live.
    pub fn check(&self) {
        conservation_check(&self.world)
            .unwrap_or_else(|e| panic!("conservation after {} events: {e}", self.log.len()));
        assert_fold_equals_live(&self.log, &self.world);
    }

    #[must_use]
    pub fn capabilities(&self) -> Capabilities {
        Capabilities::derive(
            &self.world.constitution,
            &self.world.policy,
            &self.world.params,
        )
    }

    #[must_use]
    pub fn citizen(&self, id: CitizenId) -> &crate::world::Citizen {
        &self.world.citizens[&id]
    }

    /// Ids of every citizen, in id order.
    #[must_use]
    pub fn citizen_ids(&self) -> Vec<CitizenId> {
        self.world.citizens.keys().copied().collect()
    }
}
