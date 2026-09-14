//! Employment contracts and payroll (GDD §6.1, §7.2; TDD §5.3 contract rows,
//! step 8a, T17, T18). Offers are the notice board's job ads; accepting one
//! creates a contract and an assignment; payday is cycle end.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{ContractId, OfferId, OrgId, WorkplaceId};
use crate::ledger::Party;
use crate::money::Money;
use crate::orgs::{check_room, managed_org};
use crate::tick::TickBuilder;
use crate::world::{Contract, ContractBody, ContractStatus, OfferBody, Pay, World};

/// `OfferEmployment`: the manager posts a job ad for one of the org's workplaces.
#[allow(clippy::too_many_arguments)]
pub fn offer_employment(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    workplace: WorkplaceId,
    pay: Pay,
    max_hours: u8,
    term_cycles: Option<u32>,
    notice_cycles: u32,
    places: u32,
) -> Result<Vec<Event>, Reject> {
    let o = managed_org(world, envelope, org)?;
    if o.kind == crate::kinds::OrgKind::Cooperative {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            "a cooperative admits members; it does not employ (Q86)",
        ));
    }
    let wp = world.workplaces.get(&workplace).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownWorkplace,
            format!("no workplace {workplace}"),
        )
    })?;
    if wp.org != org {
        return Err(Reject::new(
            RejectCode::NotAuthorized,
            format!("{workplace} is not {}'s", o.name),
        ));
    }
    let rate = match pay {
        Pay::Hourly(m) | Pay::PieceRate(m) => m,
    };
    if rate <= Money::ZERO {
        return Err(Reject::new(
            RejectCode::InvalidPrice,
            "pay must be positive",
        ));
    }
    // The society's wage floor (Q82): hourly as is, piece rates at the base rate.
    let floor = crate::tax::wage_floor(world, org);
    if floor > Money::ZERO {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let hourly = match pay {
            Pay::Hourly(w) => w,
            Pay::PieceRate(r) => {
                Money((r.0 as f64 * world.params.recipes[&wp.kind].base_rate).floor() as i64)
            }
        };
        if hourly < floor {
            return Err(Reject::new(
                RejectCode::BelowMinimumWage,
                format!("{hourly} an hour is below the minimum wage of {floor}"),
            ));
        }
    }
    if max_hours == 0 || max_hours > world.params.labor.base_budget_hours {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "max hours must be 1..=budget",
        ));
    }
    if places == 0 || term_cycles == Some(0) {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            "places and term must be positive",
        ));
    }
    let body = OfferBody::Employment {
        org,
        workplace,
        pay,
        max_hours,
        term_cycles,
        notice_cycles,
        places,
    };
    Ok(vec![Event::EmploymentOffered {
        offer: world.next.offer,
        body,
    }])
}

/// `AcceptEmployment`: a citizen takes a place on an open job ad.
pub fn accept_employment(
    world: &World,
    envelope: &Envelope<Command>,
    id: OfferId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let offer = world
        .offers
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOffer, format!("no offer {id}")))?;
    let OfferBody::Employment {
        org,
        workplace,
        pay,
        max_hours,
        term_cycles,
        notice_cycles,
        places,
    } = offer.body
    else {
        return Err(Reject::new(
            RejectCode::UnknownOffer,
            format!("offer {id} is not a job"),
        ));
    };
    if places == 0 {
        return Err(Reject::new(RejectCode::UnknownOffer, "no places left"));
    }
    // A contract's commitment is its notice period; open-ended with short notice is short (Q26).
    let threshold = world.params.contracts.long_contract_cycles;
    let long = term_cycles.is_some_and(|t| t > threshold) || notice_cycles > threshold;
    if citizen.flags.options_narrowed && long {
        return Err(Reject::new(
            RejectCode::OptionsNarrowed,
            "a destitute citizen cannot sign a long contract",
        ));
    }
    let wp = check_room(world, workplace)?;
    if wp.workers.contains_key(&citizen.id) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            format!("{} already works at {workplace}", citizen.id),
        ));
    }
    let contract = world.next.contract;
    Ok(vec![
        Event::EmploymentAccepted {
            contract,
            offer: id,
            org,
            workplace,
            citizen: citizen.id,
            pay,
            max_hours,
            term_cycles,
            notice_cycles,
        },
        Event::Assigned {
            workplace,
            citizen: citizen.id,
            contract: Some(contract),
        },
    ])
}

/// The employment contract, if it is one and still open.
fn employment(
    world: &World,
    id: ContractId,
) -> Result<(&Contract, OrgId, WorkplaceId, Pay, u8, u32), Reject> {
    let k = world
        .contracts
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownContract, format!("no contract {id}")))?;
    match k.body {
        ContractBody::Employment {
            org,
            workplace,
            pay,
            max_hours,
            notice_cycles,
        } if k.status != ContractStatus::Ended => {
            Ok((k, org, workplace, pay, max_hours, notice_cycles))
        }
        _ => Err(Reject::new(
            RejectCode::UnknownContract,
            format!("contract {id} is not an open employment"),
        )),
    }
}

/// Pay accrued this cycle under a contract, from the assignment's accumulators.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn accrued_pay(
    world: &World,
    workplace: WorkplaceId,
    citizen: crate::ids::CitizenId,
    pay: Pay,
) -> Money {
    let Some(a) = world
        .workplaces
        .get(&workplace)
        .and_then(|w| w.workers.get(&citizen))
    else {
        return Money::ZERO;
    };
    let tpc = i64::from(world.params.time.ticks_per_cycle);
    match pay {
        Pay::Hourly(w) => {
            // A union member's hourly rate is at least the agreed floor (S0.17d).
            let firm = world.workplaces[&workplace].org;
            let rate = match (
                crate::union::union_for(world, firm, citizen),
                crate::union::agreement_for(world, firm),
            ) {
                (Some(_), Some((floor, _))) => w.max(floor),
                _ => w,
            };
            Money(rate.0 * i64::from(a.cycle_tick_hours) / tpc)
        }
        Pay::PieceRate(r) => Money((r.0 as f64 * a.cycle_attributed).floor() as i64),
    }
}

/// `TerminateEmployment` (T18): the employer pays the notice-period wages plus
/// what has accrued; a worker who leaves forfeits this cycle's accrued pay.
pub fn terminate_employment(
    world: &World,
    envelope: &Envelope<Command>,
    id: ContractId,
) -> Result<Vec<Event>, Reject> {
    let actor = acting_citizen(world, envelope)?;
    let (k, org, workplace, pay, max_hours, notice_cycles) = employment(world, id)?;
    let (Party::Org(_), Party::Citizen(worker)) = k.parties else {
        return Err(Reject::new(
            RejectCode::UnknownContract,
            "malformed employment",
        ));
    };
    let o = &world.orgs[&org];
    let by_employer = o.manager == Some(actor.id) && envelope.on_behalf_of.is_none_or(|x| x == org);
    let by_worker = actor.id == worker;
    if !by_employer && !by_worker {
        return Err(Reject::new(
            RejectCode::NotParty,
            "only the worker or the employer terminates",
        ));
    }
    let accrued = accrued_pay(world, workplace, worker, pay);
    let mut events = Vec::new();
    let (notice_pay, forfeited) = if by_employer {
        let notice = match pay {
            Pay::Hourly(w) => Money(w.0 * i64::from(max_hours) * i64::from(notice_cycles)),
            Pay::PieceRate(_) => Money::ZERO,
        };
        (notice + accrued, Money::ZERO)
    } else {
        (Money::ZERO, accrued)
    };
    events.push(Event::EmploymentTerminated {
        contract: id,
        by: if by_employer {
            Party::Org(org)
        } else {
            Party::Citizen(worker)
        },
        notice_pay,
        forfeited,
    });
    events.push(Event::Unassigned {
        workplace,
        citizen: worker,
    });
    if notice_pay > Money::ZERO {
        let paid = notice_pay.min(o.treasury);
        events.push(Event::Paid {
            citizen: worker,
            org,
            contract: Some(id),
            amount: paid,
            explain: Explain::new(RuleId::PayHourly, "notice pay + accrued", paid)
                .input("notice_pay", notice_pay)
                .input("treasury", o.treasury),
        });
        if paid < notice_pay {
            events.push(Event::PaymentMissed {
                citizen: worker,
                org,
                contract: id,
                owed: notice_pay,
                paid,
            });
        }
    }
    Ok(events)
}

/// Step 8a: payroll. Each org pays its active employment contracts from the
/// treasury; if it cannot cover payday, everyone is paid pro rata and each
/// shortfall is a `PaymentMissed` (the contract ends and the org is flagged).
/// Then contracts whose term has run out end.
#[allow(clippy::too_many_lines)]
pub fn cycle_end_8a_payroll(b: &mut TickBuilder) {
    let cycle = b.cycle;
    let tpc = b.world.params.time.ticks_per_cycle;
    let org_ids: Vec<OrgId> = b.world.orgs.keys().copied().collect();
    for org in org_ids {
        // Owed per contract.
        let mut owed: Vec<(
            ContractId,
            crate::ids::CitizenId,
            WorkplaceId,
            Money,
            Pay,
            u32,
            f64,
        )> = Vec::new();
        for &cid in &b.world.orgs[&org].employees {
            let Some(k) = b.world.contracts.get(&cid) else {
                continue;
            };
            if k.status != ContractStatus::Active {
                continue;
            }
            let ContractBody::Employment { workplace, pay, .. } = k.body else {
                continue;
            };
            let Party::Citizen(worker) = k.parties.1 else {
                continue;
            };
            let (hours, attributed) = b
                .world
                .workplaces
                .get(&workplace)
                .and_then(|w| w.workers.get(&worker))
                .map_or((0, 0.0), |a| (a.cycle_tick_hours, a.cycle_attributed));
            let amount = accrued_pay(&b.world, workplace, worker, pay);
            if amount > Money::ZERO {
                owed.push((cid, worker, workplace, amount, pay, hours, attributed));
            }
        }
        let total: Money = owed.iter().map(|o| o.3).sum();
        let treasury = b.world.orgs[&org].treasury;
        let short = total > treasury;
        let mut breached: Vec<(WorkplaceId, crate::ids::CitizenId)> = Vec::new();
        for (cid, worker, workplace, amount, pay, hours, attributed) in owed {
            let paid = if short {
                Money(amount.0 * treasury.0 / total.0)
            } else {
                amount
            };
            let explain = match pay {
                Pay::Hourly(w) => Explain::new(RuleId::PayHourly, "hours x rate", paid)
                    .input("hours", f64::from(hours) / f64::from(tpc))
                    .input("rate", w),
                Pay::PieceRate(r) => {
                    Explain::new(RuleId::PayPieceRate, "attributed_output x rate", paid)
                        .input("attributed_output", attributed)
                        .input("rate", r)
                }
            };
            if paid > Money::ZERO {
                b.emit(Event::Paid {
                    citizen: worker,
                    org,
                    contract: Some(cid),
                    amount: paid,
                    explain,
                });
            }
            if short {
                breached.push((workplace, worker));
                b.emit(Event::PaymentMissed {
                    citizen: worker,
                    org,
                    contract: cid,
                    owed: amount,
                    paid,
                });
            }
        }
        for (workplace, worker) in breached {
            b.emit(Event::Unassigned {
                workplace,
                citizen: worker,
            });
        }
        // Term expiries end the remaining contracts.
        let ended: Vec<(ContractId, WorkplaceId, crate::ids::CitizenId)> = b.world.orgs[&org]
            .employees
            .iter()
            .filter_map(|cid| {
                let k = b.world.contracts.get(cid)?;
                let ContractBody::Employment { workplace, .. } = k.body else {
                    return None;
                };
                let Party::Citizen(worker) = k.parties.1 else {
                    return None;
                };
                let expired = k
                    .term_cycles
                    .is_some_and(|t| cycle + 1 >= k.created_tick / tpc + t);
                (k.status != ContractStatus::Ended && expired).then_some((*cid, workplace, worker))
            })
            .collect();
        for (cid, workplace, worker) in ended {
            if b.world.contracts[&cid].status != ContractStatus::Ended {
                b.emit(Event::EmploymentTerminated {
                    contract: cid,
                    by: Party::Org(org),
                    notice_pay: Money::ZERO,
                    forfeited: Money::ZERO,
                });
            }
            b.emit(Event::Unassigned {
                workplace,
                citizen: worker,
            });
        }
    }
}

/// Whether a citizen holds any active employment contract.
#[must_use]
pub fn employed(world: &World, citizen: crate::ids::CitizenId) -> bool {
    world.contracts.values().any(|k| {
        matches!(k.body, ContractBody::Employment { .. })
            && k.status != ContractStatus::Ended
            && k.parties.1 == Party::Citizen(citizen)
    })
}
