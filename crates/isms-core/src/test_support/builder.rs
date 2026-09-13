//! `WorldBuilder`: fixtures built purely from events, so `fold == live` holds
//! from the first step. Later cards add methods as their events become real.

use super::harness::Harness;
use crate::config::Preset;
use crate::event::Event;
use crate::ids::CitizenId;
use crate::ids::{OrgId, WorkplaceId};
use crate::kinds::{CitizenKind, Effort, Good, OrgKind, WorkplaceKind};
use crate::ledger::{Asset, Holder};
use crate::money::Money;
use crate::world::{Allocation, LandRegistry, Ownership, ShareHolder};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct WorldBuilder {
    preset: Preset,
    society_id: u64,
    seed: u64,
    citizens: Vec<(String, CitizenKind)>,
    pantry: Vec<(usize, Good, u32)>,
    balance_extra: Vec<(usize, Money)>,
    orgs: Vec<(OrgKind, String)>,
    workplaces: Vec<(WorkplaceKind, usize, u32)>,
    org_inventory: Vec<(usize, Good, u32)>,
    assignments: Vec<(usize, usize, u8, Effort)>,
    dwellings: Vec<usize>,
    seed_epoch: bool,
    /// The preset's population floor, restored by `seed_epoch`; fixtures
    /// otherwise run with a floor of 0 so the fill rule never adds citizens.
    preset_floor: u32,
}

impl WorldBuilder {
    #[must_use]
    pub fn new(preset_name: &str) -> Self {
        let mut preset = super::preset(preset_name);
        let preset_floor = preset.params.population.floor;
        preset.params.population.floor = 0;
        WorldBuilder {
            preset,
            society_id: 1,
            seed: 1,
            citizens: Vec::new(),
            pantry: Vec::new(),
            balance_extra: Vec::new(),
            orgs: Vec::new(),
            workplaces: Vec::new(),
            org_inventory: Vec::new(),
            assignments: Vec::new(),
            dwellings: Vec::new(),
            seed_epoch: false,
            preset_floor,
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

    /// Add an org (seeded like a legacy org: no founder, no fee; firms are 100% `OrgSelf`).
    #[must_use]
    pub fn org(mut self, kind: OrgKind, name: &str) -> Self {
        self.orgs.push((kind, name.to_owned()));
        self
    }

    /// Add a workplace of `kind` to the `org`-th org with `machines` installed.
    #[must_use]
    pub fn workplace(mut self, kind: WorkplaceKind, org: usize, machines: u32) -> Self {
        self.workplaces.push((kind, org, machines));
        self
    }

    /// Seed goods into an org's inventory.
    #[must_use]
    pub fn org_inventory(mut self, org: usize, good: Good, qty: u32) -> Self {
        self.org_inventory.push((org, good, qty));
        self
    }

    /// Assign the `citizen`-th citizen to the `workplace`-th workplace with an allocation.
    #[must_use]
    pub fn assign(mut self, citizen: usize, workplace: usize, hours: u8, effort: Effort) -> Self {
        self.assignments.push((citizen, workplace, hours, effort));
        self
    }

    /// Seed a dwelling owned by the `org`-th org (no Materials consumed).
    #[must_use]
    pub fn dwelling(mut self, org: usize) -> Self {
        self.dwellings.push(org);
        self
    }

    /// Seed the society like `start_epoch` does (legacy orgs, dwellings,
    /// householders, managers) after the explicit fixtures.
    #[must_use]
    pub fn seed_epoch(mut self) -> Self {
        self.seed_epoch = true;
        self.preset.params.population.floor = self.preset_floor;
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
        for (i, (kind, name)) in self.orgs.iter().enumerate() {
            let ownership = match kind {
                OrgKind::Firm => Ownership::Shares {
                    issued: 100,
                    holdings: BTreeMap::from([(ShareHolder::OrgSelf, 100)]),
                },
                OrgKind::Cooperative | OrgKind::Association | OrgKind::Union => Ownership::Members,
                OrgKind::Collective | OrgKind::StateEnterprise => Ownership::Society,
            };
            log.push(Event::OrgFounded {
                org: OrgId(u32::try_from(i).unwrap()),
                kind: *kind,
                name: name.clone(),
                founder: None,
                ownership,
                manager: None,
                fee_burned: Money::ZERO,
            });
        }
        let mut land = LandRegistry::from_params(&self.preset.params);
        for (i, (kind, org, machines)) in self.workplaces.iter().enumerate() {
            let id = WorkplaceId(u32::try_from(i).unwrap());
            let slot = land.free_slot(*kind);
            assert!(
                slot.is_some() || !land.is_limited(*kind),
                "no free {kind:?} slot for fixture workplace {i}"
            );
            if let Some(s) = slot {
                land.slots.get_mut(&s).unwrap().workplace = Some(id);
            }
            log.push(Event::WorkplaceAdded {
                workplace: id,
                org: OrgId(u32::try_from(*org).unwrap()),
                kind: *kind,
                slot,
                materials_consumed: 0,
            });
            if *machines > 0 {
                log.push(Event::Seeded {
                    holder: Holder::Workplace(id),
                    asset: Asset::Good(Good::Machines, *machines),
                });
            }
        }
        for (org, good, qty) in &self.org_inventory {
            log.push(Event::Seeded {
                holder: Holder::Org(OrgId(u32::try_from(*org).unwrap())),
                asset: Asset::Good(*good, *qty),
            });
        }
        for (i, org) in self.dwellings.iter().enumerate() {
            log.push(Event::DwellingBuilt {
                dwelling: crate::ids::DwellingId(u32::try_from(i).unwrap()),
                org: OrgId(u32::try_from(*org).unwrap()),
                workplace: None,
                materials_consumed: 0,
            });
        }
        for (citizen, workplace, hours, effort) in &self.assignments {
            let citizen = CitizenId(u32::try_from(*citizen).unwrap());
            let workplace = WorkplaceId(u32::try_from(*workplace).unwrap());
            log.push(Event::Assigned {
                workplace,
                citizen,
                contract: None,
            });
            log.push(Event::LaborSet {
                citizen,
                allocations: vec![Allocation {
                    workplace,
                    hours: *hours,
                    effort: *effort,
                }],
            });
        }
        if self.seed_epoch {
            // EpochStarted is already the second event; append the rest of the seeding.
            let world = super::fold::fold(&log);
            let rules = crate::rules::Rules::from_world(&world);
            let mut seeded = crate::seeding::start_epoch(&world, &rules, 0);
            seeded.retain(|e| !matches!(e, Event::EpochStarted { .. }));
            log.extend(seeded);
        }
        log
    }

    #[must_use]
    pub fn build(self) -> Harness {
        Harness::from_log(self.events())
    }
}
