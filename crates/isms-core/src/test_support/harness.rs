//! `Harness`: a live `World` plus its complete event log. After every step it
//! asserts conservation and that folding the log reproduces the live state.

use super::fold::assert_fold_equals_live;
use crate::apply::apply;
use crate::capabilities::Capabilities;
use crate::command::{Command, Envelope, Reject, handle};
use crate::event::Event;
use crate::ids::CitizenId;
use crate::ledger::conservation_check;
use crate::rules::Rules;
use crate::tick::{TickError, TickInput, tick};
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

    /// Current rules (re-derived; cheap).
    #[must_use]
    pub fn rules(&self) -> Rules {
        Rules::from_world(&self.world)
    }

    /// Validate and execute a command, applying its events. Rejections are returned.
    #[allow(clippy::needless_pass_by_value)]
    pub fn cmd(&mut self, envelope: Envelope<Command>) -> Result<Vec<Event>, Reject> {
        let events = handle(&self.world, &self.rules(), &envelope)?;
        self.apply_all(events.iter().cloned());
        Ok(events)
    }

    /// Validate a command without applying anything.
    #[allow(clippy::needless_pass_by_value)]
    pub fn cmd_dry(&self, envelope: Envelope<Command>) -> Result<Vec<Event>, Reject> {
        handle(&self.world, &self.rules(), &envelope)
    }

    /// Resolve the next tick and apply its events.
    pub fn try_tick(&mut self) -> Result<Vec<Event>, TickError> {
        let input = TickInput::next_for(&self.world);
        let events = tick(&self.world, &self.rules(), input)?;
        self.apply_all(events.iter().cloned());
        Ok(events)
    }

    /// Resolve the next tick; panics if the epoch has ended.
    pub fn tick(&mut self) -> Vec<Event> {
        self.try_tick().expect("tick")
    }

    /// Run to the end of the current cycle (inclusive).
    pub fn run_cycle(&mut self) -> Vec<Event> {
        let mut all = Vec::new();
        loop {
            let at_end = self.world.is_cycle_end(self.world.meta.tick);
            all.extend(self.tick());
            if at_end {
                return all;
            }
        }
    }

    pub fn run_cycles(&mut self, n: u32) -> Vec<Event> {
        (0..n).flat_map(|_| self.run_cycle()).collect()
    }

    /// Test-only: set a citizen's continuous state through a synthetic
    /// `TickResolved` that does not advance the clock (fold == live still holds).
    pub fn set_needs(&mut self, id: CitizenId, needs: crate::world::Needs) {
        let c = &self.world.citizens[&id];
        let delta = crate::event::CitizenDelta {
            citizen: id,
            needs,
            food_eaten: 0,
            wares_consumed: 0,
            output_mult: c.labor.output_mult,
            budget: c.labor.budget,
            fatigue_debt: c.labor.fatigue_debt,
            consecutive_high_effort_cycles: c.labor.consecutive_high_effort_cycles,
            skill: c.labor.skill.clone(),
        };
        let tick = self.world.meta.tick.wrapping_sub(1);
        self.apply(Event::TickResolved {
            tick,
            cycle: self.world.cycle_of(self.world.meta.tick),
            price_index: None,
            citizen_deltas: vec![delta],
            workplace_deltas: Vec::new(),
        });
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
