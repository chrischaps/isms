//! `WorldBuilder`: fixtures built purely from events, so `fold == live` holds
//! from the first step. Later cards add methods as their events become real.

use super::harness::Harness;
use crate::config::Preset;
use crate::event::Event;
use crate::ids::CitizenId;
use crate::kinds::{CitizenKind, Good};
use crate::ledger::{Asset, Holder};
use crate::money::Money;

#[derive(Debug)]
pub struct WorldBuilder {
    preset: Preset,
    society_id: u64,
    seed: u64,
    citizens: Vec<(String, CitizenKind)>,
    pantry: Vec<(usize, Good, u32)>,
    balance_extra: Vec<(usize, Money)>,
}

impl WorldBuilder {
    #[must_use]
    pub fn new(preset_name: &str) -> Self {
        WorldBuilder {
            preset: super::preset(preset_name),
            society_id: 1,
            seed: 1,
            citizens: Vec::new(),
            pantry: Vec::new(),
            balance_extra: Vec::new(),
        }
    }

    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Edit the loaded preset before the society is created (e.g. tweak a param).
    #[must_use]
    pub fn with_preset(mut self, edit: impl FnOnce(&mut Preset)) -> Self {
        edit(&mut self.preset);
        self
    }

    /// Add `n` human citizens named `h0`, `h1`, ...
    #[must_use]
    pub fn humans(mut self, n: u32) -> Self {
        for i in 0..n {
            self.citizens.push((format!("h{i}"), CitizenKind::Human));
        }
        self
    }

    /// Add `n` householders named `hh0`, `hh1`, ...
    #[must_use]
    pub fn householders(mut self, n: u32) -> Self {
        for i in 0..n {
            self.citizens
                .push((format!("hh{i}"), CitizenKind::Householder));
        }
        self
    }

    /// Seed `qty` of `good` into the pantry of the `citizen`-th citizen added.
    #[must_use]
    pub fn pantry(mut self, citizen: usize, good: Good, qty: u32) -> Self {
        self.pantry.push((citizen, good, qty));
        self
    }

    /// Seed extra money (beyond the endowment) into a citizen's balance.
    #[must_use]
    pub fn balance_extra(mut self, citizen: usize, amount: Money) -> Self {
        self.balance_extra.push((citizen, amount));
        self
    }

    /// The event log this builder produces.
    #[must_use]
    pub fn events(&self) -> Vec<Event> {
        let has_money = self.preset.constitution.has_money();
        let endowment = if has_money {
            self.preset.params.money.endowment
        } else {
            Money::ZERO
        };
        let mut log = vec![
            Event::SocietyCreated {
                society_id: self.society_id,
                seed: self.seed,
                preset: Box::new(self.preset.clone()),
            },
            Event::EpochStarted { epoch: 0 },
        ];
        for (i, (handle, kind)) in self.citizens.iter().enumerate() {
            let citizen = CitizenId(u32::try_from(i).unwrap());
            log.push(match kind {
                CitizenKind::Human => Event::CitizenJoined {
                    citizen,
                    handle: handle.clone(),
                    kind: *kind,
                    endowment,
                    dwelling: None,
                    explain: None,
                },
                CitizenKind::Householder => Event::HouseholderJoined {
                    citizen,
                    handle: handle.clone(),
                    endowment,
                    dwelling: None,
                    explain: None,
                },
            });
        }
        for (i, good, qty) in &self.pantry {
            log.push(Event::Seeded {
                holder: Holder::Citizen(CitizenId(u32::try_from(*i).unwrap())),
                asset: Asset::Good(*good, *qty),
            });
        }
        for (i, amount) in &self.balance_extra {
            log.push(Event::Seeded {
                holder: Holder::Citizen(CitizenId(u32::try_from(*i).unwrap())),
                asset: Asset::Money(*amount),
            });
        }
        log
    }

    #[must_use]
    pub fn build(self) -> Harness {
        Harness::from_log(self.events())
    }
}
