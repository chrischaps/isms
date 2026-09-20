//! The headless simulator's scripts (TDD T10, Appendix A; S0.16c, S2.4):
//! pure decision functions over `World`, like the householder script.
//!
//! The System planner: on the first tick of each cycle it publishes targets
//! of last cycle's output times `sim_planner_target_growth` for every
//! workplace that produced anything, and it approves every pending transfer
//! request. It never touches prices, grades, the split or ration cards: those
//! stay at the preset's policy until Phase 4's Committee exists.
//!
//! The assembly (S2.4): the `sim.assembly_size` humans `isms-sim` seeds are
//! present each cycle, live by the householder script, and govern by this
//! one: the first of them moves the Materials split nudged toward the
//! scarcest sink each cycle (back toward even shares when nothing is short)
//! and honors the top contributor every
//! `sim.honor_every_cycles`; every one of them stands whenever a seat is
//! open to them, approves every candidate, and votes yes on a split and on
//! an honor (Q134). It is the reference for S2.10's scripted personas.

use crate::command::{Command, Envelope, Reject};
use crate::constitution::{Governance, LaborMode, ProposalKindTag};
use crate::event::{Actor, Event};
use crate::ids::{CitizenId, OrgId, Tick, WorkplaceId};
use crate::kinds::{CitizenKind, ClientKind, Good};
use crate::policy::{MaterialsSplit, PolicyPatch};
use crate::world::{Ballot, ProposalKind, World};
use std::collections::BTreeMap;

/// The System's commands this round (administered societies only).
#[must_use]
pub fn decide_system(world: &World) -> Vec<Command> {
    if world.constitution.pricing != crate::constitution::Pricing::Administered {
        return Vec::new();
    }
    let mut cmds = Vec::new();
    let tpc = world.params.time.ticks_per_cycle;
    if world.meta.tick.is_multiple_of(tpc) && world.meta.tick > 0 {
        let growth = world.params.governance.sim_planner_target_growth;
        let targets: BTreeMap<WorkplaceId, f64> = world
            .workplaces
            .values()
            .filter(|w| w.last_cycle_output > 0.0)
            .map(|w| (w.id, w.last_cycle_output * growth))
            .collect();
        if !targets.is_empty() {
            cmds.push(Command::SetPlan {
                targets,
                materials_split: None,
                price_list: None,
                wage_grades: None,
                ration_caps: None,
            });
        }
    }
    if world.constitution.labor == LaborMode::Assigned {
        for citizen in world.transfer_requests.keys() {
            cmds.push(Command::DecideTransfer {
                citizen: *citizen,
                approve: true,
            });
        }
    }
    cmds
}

/// Run the System's script once, applying its events to `world`. A rejection
/// is a script bug, like a householder's, and is returned in full.
pub fn run_system_round(
    world: &mut World,
    rules: &crate::rules::Rules,
    tick: crate::ids::Tick,
) -> (Vec<crate::event::Event>, Vec<crate::command::Reject>) {
    let mut events = Vec::new();
    let mut rejected = Vec::new();
    for command in decide_system(world) {
        let env = crate::command::Envelope::system(command, tick);
        match crate::command::handle(world, rules, &env) {
            Ok(evs) => {
                for e in evs {
                    crate::apply::apply(world, &e);
                    events.push(e);
                }
            }
            Err(reject) => rejected.push(reject),
        }
    }
    (events, rejected)
}

// ---------------------------------------------------------------------------
// The scripted assembly (S2.4).

/// The scripted humans: every human on the rolls, in id order. The
/// simulator seeds them and nobody else joins a headless society.
#[must_use]
pub fn assembly(world: &World) -> Vec<CitizenId> {
    world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Human)
        .map(|c| c.id)
        .collect()
}

fn active(world: &World) -> impl Iterator<Item = CitizenId> + '_ {
    world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Human && !c.dormant)
        .map(|c| c.id)
}

/// Pass one of a round: the humans who send `Seen` this tick. Every human
/// on the first tick of a cycle (Q127: presence is a session's, and the
/// simulator's humans hold one each cycle), and any dormant human at once
/// (the epoch rollover puts every human to sleep, Q41).
#[must_use]
pub fn decide_presence(world: &World, tick: Tick) -> Vec<CitizenId> {
    let first_tick = tick.is_multiple_of(world.params.time.ticks_per_cycle);
    assembly(world)
        .into_iter()
        .filter(|id| first_tick || world.citizens[id].dormant)
        .collect()
}

/// The three Materials sinks the split feeds (`labor.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sink {
    Wares,
    Machines,
    Dwellings,
}

/// The sink the assembly nudges toward, each read against a need the engine
/// already states, so no sink is scarce forever (Q134): Dwellings while an
/// active citizen is unhoused and the society has none free; else Wares
/// while the Common Store holds fewer than the active citizens' Wares
/// entitlements (tomorrow's draws would go short); else Machines while the
/// staffed workplaces hold fewer machines than workers (below one per
/// worker the capital curve's marginal gain is still above half its start);
/// `None` when nothing is short.
#[must_use]
pub fn scarcest_sink(world: &World) -> Option<Sink> {
    let unhoused = world
        .citizens
        .values()
        .any(|c| !c.dormant && c.household.dwelling.is_none());
    if unhoused && crate::housing::free_society_dwelling(world).is_none() {
        return Some(Sink::Dwellings);
    }
    let stock = world
        .store
        .as_ref()
        .and_then(|s| s.stock.get(&Good::Wares).copied())
        .unwrap_or(0);
    let demand: u32 = world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| crate::store::entitlement(world, c, Good::Wares))
        .sum();
    if stock < demand {
        return Some(Sink::Wares);
    }
    let (machines, workers) = world
        .workplaces
        .values()
        .filter(|w| !w.workers.is_empty())
        .fold((0u32, 0u32), |(m, n), w| {
            (
                m + w.machines,
                n + u32::try_from(w.workers.len()).unwrap_or(u32::MAX),
            )
        });
    (workers > 0 && machines < workers).then_some(Sink::Machines)
}

/// `split` moved `nudge` toward `sink`, the other two sinks giving up half
/// each (never below zero); with no sink short, every share moved `nudge` of
/// the way back toward even thirds. Renormalised so the three sum to one.
#[must_use]
pub fn nudged_split(split: MaterialsSplit, sink: Option<Sink>, nudge: f64) -> MaterialsSplit {
    let mut v = [split.wares, split.machines, split.dwellings];
    match sink {
        Some(sink) => {
            let t = match sink {
                Sink::Wares => 0,
                Sink::Machines => 1,
                Sink::Dwellings => 2,
            };
            for (i, x) in v.iter_mut().enumerate() {
                *x = if i == t {
                    (*x + nudge).min(1.0)
                } else {
                    (*x - nudge / 2.0).max(0.0)
                };
            }
        }
        None => {
            for x in &mut v {
                *x += nudge * (1.0 / 3.0 - *x);
            }
        }
    }
    let sum = v[0] + v[1] + v[2];
    let wares = v[0] / sum;
    let machines = v[1] / sum;
    MaterialsSplit {
        wares,
        machines,
        dwellings: 1.0 - wares - machines,
    }
}

/// The citizen with the most hours on the Ledger of Contribution, the lower
/// id on a tie; `None` before anyone has worked.
#[must_use]
pub fn top_contributor(world: &World) -> Option<CitizenId> {
    world
        .citizens
        .values()
        .filter(|c| c.contribution.tick_hours_total > 0)
        .map(|c| (std::cmp::Reverse(c.contribution.tick_hours_total), c.id))
        .min()
        .map(|(_, id)| id)
}

fn open_split_proposal(world: &World) -> bool {
    world.proposals.values().any(|p| {
        matches!(
            p.kind,
            ProposalKind::PolicyChange { patch } if patch.materials_split.is_some()
        )
    })
}

/// The motions the assembly's speaker (its first active member) opens on the
/// first tick of a cycle.
fn decide_motions(world: &World, tick: Tick) -> Vec<Command> {
    let mut cmds = Vec::new();
    let kinds = &world.constitution.proposal_kinds;
    let cycle = world.cycle_of(tick);
    if kinds.contains(&ProposalKindTag::PolicyChange)
        && let Some(split) = world.policy.materials_split
        && !open_split_proposal(world)
    {
        let sink = scarcest_sink(world);
        let (title, text) = match sink {
            Some(s) => (
                format!("Cycle {cycle}: the Materials split, toward {s:?}"),
                format!("The society is shortest of {s:?}; move the split that way."),
            ),
            None => (
                format!("Cycle {cycle}: the Materials split, toward even shares"),
                "Nothing is short; let the split drift back toward even thirds.".to_owned(),
            ),
        };
        cmds.push(Command::Propose {
            title,
            text,
            kind: ProposalKind::PolicyChange {
                patch: PolicyPatch {
                    materials_split: Some(nudged_split(split, sink, world.params.sim.split_nudge)),
                    ..PolicyPatch::default()
                },
            },
        });
    }
    let every = world.params.sim.honor_every_cycles;
    if kinds.contains(&ProposalKindTag::Honor)
        && every > 0
        && cycle > 0
        && cycle.is_multiple_of(every)
        && let Some(citizen) = top_contributor(world)
        && !world
            .proposals
            .values()
            .any(|p| p.kind == ProposalKind::Honor { citizen })
    {
        cmds.push(Command::Propose {
            title: format!("Cycle {cycle}: honor the top contributor"),
            text: "For the most hours on the Ledger.".to_owned(),
            kind: ProposalKind::Honor { citizen },
        });
    }
    cmds
}

/// Pass two: one active human's governance this tick. Stand in any open
/// election the office rules admit them to (`offices::stand` is the oracle,
/// so the script never guesses at eligibility), approve every candidate,
/// vote yes on a split and on an honor, and, as the speaker on a cycle's
/// first tick, open the cycle's motions.
#[must_use]
pub fn decide_assembly(world: &World, id: CitizenId, tick: Tick) -> Vec<Command> {
    let Some(c) = world.citizens.get(&id) else {
        return Vec::new();
    };
    if c.dormant || c.kind != CitizenKind::Human {
        return Vec::new();
    }
    let mut cmds = Vec::new();
    let first_tick = tick.is_multiple_of(world.params.time.ticks_per_cycle);
    if first_tick
        && world.constitution.governance == Governance::Direct
        && active(world).next() == Some(id)
    {
        cmds.extend(decide_motions(world, tick));
    }
    for (office, election) in &world.offices.elections {
        let stand = Command::Stand { office: *office };
        if !election.candidates.contains(&id)
            && crate::offices::stand(world, &Envelope::citizen(id, stand.clone(), tick), *office)
                .is_ok()
        {
            cmds.push(stand);
        }
        if !election.candidates.is_empty()
            && election.approvals.get(&id) != Some(&election.candidates)
        {
            cmds.push(Command::Approve {
                office: *office,
                candidates: election.candidates.clone(),
            });
        }
    }
    for p in world.proposals.values() {
        if p.ballots.contains_key(&id) {
            continue;
        }
        let yes = match p.kind {
            ProposalKind::PolicyChange { patch } => patch.materials_split.is_some(),
            ProposalKind::Honor { .. } => true,
            _ => false,
        };
        if yes {
            cmds.push(Command::Vote {
                proposal: p.id,
                ballot: Ballot::Yes,
            });
        }
    }
    cmds
}

/// Run the assembly once, applying its events to `world`: presence first,
/// then each active human's governance and, since the simulator's humans
/// live like householders, the householder script's commands. A rejection is
/// a script bug and is returned in full.
pub fn run_assembly_round(
    world: &mut World,
    rules: &crate::rules::Rules,
    tick: Tick,
) -> (Vec<Event>, Vec<Reject>) {
    let mut events = Vec::new();
    let mut rejected = Vec::new();
    let mut run = |world: &mut World, env: Envelope<Command>| match crate::command::handle(
        world, rules, &env,
    ) {
        Ok(evs) => {
            for e in evs {
                crate::apply::apply(world, &e);
                events.push(e);
            }
        }
        Err(reject) => rejected.push(reject),
    };
    for id in decide_presence(world, tick) {
        run(world, envelope(id, None, Command::Seen, tick));
    }
    let members: Vec<CitizenId> = active(world).collect();
    for id in members {
        let mut cmds: Vec<(Option<OrgId>, Command)> = decide_assembly(world, id, tick)
            .into_iter()
            .chain(crate::householder::decide(world, id))
            .map(|c| (None, c))
            .collect();
        let managed: Vec<OrgId> = world
            .orgs
            .values()
            .filter(|o| o.manager == Some(id))
            .map(|o| o.id)
            .collect();
        for org in managed {
            cmds.extend(
                crate::householder::decide_manager(world, org)
                    .into_iter()
                    .map(|c| (Some(org), c)),
            );
        }
        for (org, command) in cmds {
            run(world, envelope(id, org, command, tick));
        }
    }
    (events, rejected)
}

fn envelope(
    id: CitizenId,
    on_behalf_of: Option<OrgId>,
    command: Command,
    tick: Tick,
) -> Envelope<Command> {
    Envelope {
        actor: Actor::Citizen(id),
        on_behalf_of,
        client_kind: ClientKind::Sim,
        received_at_tick: tick,
        command,
    }
}
