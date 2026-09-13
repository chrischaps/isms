//! The scripted System planner for the headless simulator (TDD T10, Appendix A
//! "Sim planner (Directorate) target growth"; S0.16c). A pure decision
//! function, like the householder script: on the first tick of each cycle it
//! publishes targets of last cycle's output times `sim_planner_target_growth`
//! for every workplace that produced anything, and it approves every pending
//! transfer request. It never touches prices, grades, the split or ration
//! cards: those stay at the preset's policy until Phase 4's Committee exists.

use crate::command::Command;
use crate::constitution::LaborMode;
use crate::ids::WorkplaceId;
use crate::world::World;
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
