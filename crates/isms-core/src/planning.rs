//! The Plan (GDD §6.3; TDD §5.5 step 8a, S0.16b): wage grades, plan targets
//! with the bonus and the ratchet, `SetPlan`, and transfer requests. In an
//! administered society the state pays every position from the till by the
//! published wage-grade table (grade by skill band of the workplace's job
//! family, Q73); a workplace that reaches its target earns its workers a bonus
//! on top; overfulfilment raises next cycle's target when the ratchet is on
//! (Q70). Targets, the price list, the grades, the Materials split and ration
//! cards are published together by `SetPlan` (the Committee; `System` until
//! Phase 2 governance lands, Q56).

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::constitution::{Compensation, LaborMode};
use crate::event::{Actor, Event};
use crate::explain::{Explain, RuleId};
use crate::ids::{CitizenId, WorkplaceId};
use crate::kinds::Good;
use crate::money::Money;
use crate::policy::{MaterialsSplit, Policy};
use crate::tick::TickBuilder;
use crate::world::World;
use std::collections::BTreeMap;

/// The wage grade a citizen earns at a workplace: the skill band of the
/// workplace's job family, capped at the top grade.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn grade_of(
    world: &World,
    citizen: CitizenId,
    workplace: WorkplaceId,
) -> Option<(usize, Money)> {
    let grades = world.policy.wage_grades.as_ref()?;
    if grades.is_empty() {
        return None;
    }
    let wp = world.workplaces.get(&workplace)?;
    let level = world
        .citizens
        .get(&citizen)?
        .labor
        .skill
        .get(&wp.kind.job_family())
        .map_or(0.0, |s| s.level);
    let band = (level / f64::from(world.params.labor.skill_band)).floor() as usize;
    let band = band.min(grades.len() - 1);
    Some((band, grades[band]))
}

/// Step 8a for wage-scale systems: every position is paid from the till at its
/// grade for the cycle's hours, plus the plan bonus where the workplace met
/// its target; a short till pays everyone pro rata (Q67). Then the ratchet.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
pub fn cycle_end_8a_scale_payroll(b: &mut TickBuilder) {
    if b.rules.capabilities.pay != Compensation::Scale {
        return;
    }
    let tpc = b.world.params.time.ticks_per_cycle;
    let bonus_fraction = b.world.policy.plan_bonus_fraction.unwrap_or(0.0);
    // Owed: (citizen, org, workplace, grade index, hours, pay, bonus)
    let mut owed: Vec<(
        CitizenId,
        crate::ids::OrgId,
        WorkplaceId,
        usize,
        u32,
        Money,
        Money,
    )> = Vec::new();
    for wp in b.world.workplaces.values() {
        let met = wp.target.is_some_and(|t| wp.cycle_output >= t);
        for (citizen, a) in &wp.workers {
            if a.cycle_tick_hours == 0 {
                continue;
            }
            let Some((band, grade)) = grade_of(&b.world, *citizen, wp.id) else {
                continue;
            };
            let pay = Money(grade.0 * i64::from(a.cycle_tick_hours) / i64::from(tpc));
            let bonus = if met {
                Money((pay.0 as f64 * bonus_fraction).floor() as i64)
            } else {
                Money::ZERO
            };
            owed.push((
                *citizen,
                wp.org,
                wp.id,
                band,
                a.cycle_tick_hours,
                pay,
                bonus,
            ));
        }
    }
    let total: Money = owed.iter().map(|o| o.5 + o.6).sum();
    let till = b.world.state_stock.as_ref().map_or(Money::ZERO, |s| s.till);
    let short = total > till;
    for (citizen, org, workplace, band, hours, pay, bonus) in owed {
        let scale = |m: Money| {
            if short {
                Money(m.0 * till.0 / total.0)
            } else {
                m
            }
        };
        let paid = scale(pay);
        if paid > Money::ZERO {
            let explain = Explain::new(RuleId::PayScale, "hours x grade", paid)
                .input("hours", f64::from(hours) / f64::from(tpc))
                .input("grade", i64::try_from(band).unwrap_or(0))
                .input("rate", pay.0 / i64::from(hours.max(1)) * i64::from(tpc))
                .input("till", till);
            b.emit(Event::Paid {
                citizen,
                org,
                contract: None,
                amount: paid,
                explain,
            });
        }
        let paid_bonus = scale(bonus);
        if paid_bonus > Money::ZERO {
            let target = b.world.workplaces[&workplace].target.unwrap_or(0.0);
            let explain = Explain::new(RuleId::PlanBonus, "pay x plan_bonus_fraction", paid_bonus)
                .input("pay", pay)
                .input("plan_bonus_fraction", bonus_fraction)
                .input("target", target)
                .input("output", b.world.workplaces[&workplace].cycle_output);
            b.emit(Event::Paid {
                citizen,
                org,
                contract: None,
                amount: paid_bonus,
                explain,
            });
        }
    }
    // The ratchet: overfulfilment raises next cycle's target (Q70).
    if b.world.policy.ratchet == Some(true) {
        let mult = b.world.params.governance.ratchet_mult;
        let raised: Vec<(WorkplaceId, f64)> = b
            .world
            .workplaces
            .values()
            .filter_map(|w| {
                let t = w.target?;
                (w.cycle_output > t).then(|| (w.id, t.max(w.cycle_output * mult)))
            })
            .collect();
        for (workplace, target) in raised {
            b.emit(Event::TargetSet {
                workplace,
                target,
                by: Actor::System,
            });
        }
    }
}

/// `SetPlan`: the Committee publishes targets and, optionally, a new price
/// list, wage grades, Materials split and ration cards (a typed `SetPolicy`).
#[allow(clippy::too_many_arguments)]
pub fn set_plan(
    world: &World,
    envelope: &Envelope<Command>,
    targets: &BTreeMap<WorkplaceId, f64>,
    materials_split: Option<MaterialsSplit>,
    price_list: Option<&BTreeMap<Good, Money>>,
    wage_grades: Option<&[Money]>,
    ration_caps: Option<&BTreeMap<Good, u32>>,
) -> Result<Vec<Event>, Reject> {
    if envelope.actor != Actor::System {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "the Plan is published by the Committee (System in Phase 0)",
        ));
    }
    for (wp, t) in targets {
        if !world.workplaces.contains_key(wp) {
            return Err(Reject::new(
                RejectCode::UnknownWorkplace,
                format!("no workplace {wp}"),
            ));
        }
        if !t.is_finite() || *t < 0.0 {
            return Err(Reject::new(
                RejectCode::InvalidQuantity,
                "a target is a non-negative number of units",
            ));
        }
    }
    let mut events = vec![Event::PlanPublished {
        cycle: world.cycle_of(envelope.received_at_tick),
        targets: targets.clone(),
        by: envelope.actor,
    }];
    if materials_split.is_some()
        || price_list.is_some()
        || wage_grades.is_some()
        || ration_caps.is_some()
    {
        let mut policy: Policy = world.policy.clone();
        if let Some(s) = materials_split {
            policy.materials_split = Some(s);
        }
        if let Some(p) = price_list {
            policy.price_list = Some(p.clone());
        }
        if let Some(g) = wage_grades {
            policy.wage_grades = Some(g.to_vec());
        }
        if let Some(c) = ration_caps {
            policy.ration_caps = Some(c.clone());
        }
        policy
            .validate_against(&world.constitution)
            .map_err(|reason| Reject::new(RejectCode::NotInThisSociety, reason))?;
        events.push(Event::PolicyChanged {
            policy: Box::new(policy),
            by: envelope.actor,
            proposal: None,
        });
    }
    Ok(events)
}

/// `RequestTransfer`: ask the Committee for a different workplace.
pub fn request_transfer(
    world: &World,
    envelope: &Envelope<Command>,
    to_workplace: WorkplaceId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let wp = crate::orgs::check_room(world, to_workplace)?;
    if wp.workers.contains_key(&citizen.id) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{} already works at {to_workplace}", citizen.id),
        ));
    }
    Ok(vec![Event::TransferRequested {
        citizen: citizen.id,
        to_workplace,
    }])
}

/// `DecideTransfer`: the Committee approves or denies a pending request; an
/// approval moves the citizen's position (their allocation resets).
pub fn decide_transfer(
    world: &World,
    envelope: &Envelope<Command>,
    citizen: CitizenId,
    approve: bool,
) -> Result<Vec<Event>, Reject> {
    if envelope.actor != Actor::System {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "transfers are decided by the Committee (System in Phase 0)",
        ));
    }
    if world.constitution.labor != LaborMode::Assigned {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            "no assignments here",
        ));
    }
    let Some(to_workplace) = world.transfer_requests.get(&citizen).copied() else {
        return Err(Reject::new(
            RejectCode::NoRequestPending,
            format!("{citizen} has no transfer request pending"),
        ));
    };
    let mut events = vec![Event::TransferDecided {
        citizen,
        to_workplace,
        approved: approve,
    }];
    if approve && crate::orgs::check_room(world, to_workplace).is_ok() {
        for w in world.workplaces.values() {
            if w.workers.contains_key(&citizen) {
                events.push(Event::Unassigned {
                    workplace: w.id,
                    citizen,
                });
            }
        }
        events.push(Event::Assigned {
            workplace: to_workplace,
            citizen,
            contract: None,
        });
    }
    Ok(events)
}
