//! The householder script and the legacy-firm manager script (GDD §11.3;
//! TDD §9.3, D8). Both are pure decision functions over the `World` that return
//! ordinary commands; `run_round` executes them through `handle` between
//! ticks, so householders are just another client. `SCRIPT.md` is the
//! published plain-language version.

use crate::apply::apply;
use crate::command::{Command, Envelope, handle};
use crate::event::{Actor, Event};
use crate::ids::{CitizenId, OrgId, Tick, WorkplaceId};
use crate::kinds::{CitizenKind, ClientKind, Effort, Good, WorkplaceKind};
use crate::ledger::Party;
use crate::market::{last_price, open_bid_qty};
use crate::money::Money;
use crate::rules::Rules;
use crate::world::{
    Allocation, BuyRule, Instrument, LaborPlan, LeaseAsset, OfferBody, OrderSource, Ownership, Pay,
    Price, SaleAsset, ShareHolder, Side, StandingPlan, VoteDefault, World,
};

/// What a good costs a householder: the last market price, or the published
/// list price where prices are administered (nothing where there is no money).
fn price_of(world: &World, good: Good) -> Money {
    last_price(world, Instrument::Good(good))
        .or_else(|| crate::state_store::list_price(world, good))
        .unwrap_or(Money::ZERO)
}

/// One cycle's living cost: 24 Food and 4 Wares at last price, plus rent (Q15).
#[must_use]
pub fn living_cost(world: &World, citizen: &crate::world::Citizen) -> Money {
    let p = &world.params.householder;
    let mut cost = Money(price_of(world, Good::Food).0 * i64::from(p.living_cost_food))
        + Money(price_of(world, Good::Wares).0 * i64::from(p.living_cost_wares));
    if let Some(k) = citizen
        .household
        .dwelling
        .and_then(|d| world.dwellings.get(&d))
        .and_then(|d| d.lease)
        .and_then(|k| world.contracts.get(&k))
        && let crate::world::ContractBody::Lease { rent_per_cycle, .. } = k.body
    {
        cost += rent_per_cycle;
    }
    cost
}

/// The plan a householder keeps (GDD §11.3, TDD §9.3).
#[must_use]
pub fn householder_plan(world: &World, citizen: &crate::world::Citizen) -> StandingPlan {
    let p = &world.params.householder;
    let money = world.constitution.has_money();
    let living = living_cost(world, citizen);
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let balance_above = Money((living.0 as f64 * p.wares_balance_living_cost_mult) as i64);
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let keep_balance = Money((citizen.wages_total.0 as f64 * p.save_fraction) as i64);
    let labor = match world.constitution.labor {
        crate::constitution::LaborMode::Norm => LaborPlan::FollowNorm,
        crate::constitution::LaborMode::Assigned => LaborPlan::AcceptAssignment,
        crate::constitution::LaborMode::Free => LaborPlan::Explicit,
    };
    StandingPlan {
        labor,
        keep_food_at_least: p.keep_food_at_least,
        max_food_price: None,
        buy_wares_when: money.then_some(BuyRule {
            comfort_below: p.wares_comfort_below,
            balance_above,
            max_price: None,
        }),
        keep_balance_at_least: if money { keep_balance } else { Money::ZERO },
        standing_orders: Vec::new(),
        vote_default: VoteDefault::None,
    }
}

/// Hourly-equivalent pay of an offer, for ranking.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn hourly_equivalent(world: &World, workplace: WorkplaceId, pay: Pay) -> Money {
    match pay {
        Pay::Hourly(w) => w,
        Pay::PieceRate(r) => {
            let kind = world
                .workplaces
                .get(&workplace)
                .map_or(WorkplaceKind::Farm, |w| w.kind);
            #[allow(clippy::cast_possible_truncation)]
            Money((r.0 as f64 * world.params.recipes[&kind].base_rate) as i64)
        }
    }
}

/// A householder's commands this round (market systems). Every command is
/// valid against the current world, so a rejection is a script bug.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn decide(world: &World, id: CitizenId) -> Vec<Command> {
    let Some(c) = world.citizens.get(&id) else {
        return Vec::new();
    };
    if c.dormant || c.kind != CitizenKind::Householder {
        return Vec::new();
    }
    let mut cmds = Vec::new();
    let money = world.constitution.has_money();

    // 1. Keep the published plan current.
    let plan = householder_plan(world, c);
    if c.plan != plan {
        cmds.push(Command::SetStandingPlan {
            plan: Box::new(plan),
        });
    }

    // 2. Work: take the best open job when unemployed (or, under a work norm,
    // the least-staffed workplace); allocate the contract's or the norm's hours
    // once in a position.
    let norm = world.constitution.labor == crate::constitution::LaborMode::Norm;
    let norm_hours = world
        .policy
        .work_norm_hours
        .unwrap_or(world.params.labor.base_budget_hours);
    let assigned: Vec<(WorkplaceId, u8, u8)> = world
        .workplaces
        .values()
        .filter_map(|w| {
            w.workers.get(&id).map(|a| {
                (
                    w.id,
                    a.hours,
                    a.contract.map_or(
                        if norm {
                            norm_hours
                        } else {
                            world.params.labor.base_budget_hours
                        },
                        |k| match world.contracts.get(&k).map(|k| &k.body) {
                            Some(crate::world::ContractBody::Employment { max_hours, .. }) => {
                                *max_hours
                            }
                            _ => world.params.labor.base_budget_hours,
                        },
                    ),
                )
            })
        })
        .collect();
    let share = world.constitution.compensation == crate::constitution::Compensation::Share;
    if assigned.is_empty() && norm {
        if let Some(workplace) = crate::orgs::least_staffed(world) {
            cmds.push(Command::JoinWorkplace { workplace });
        }
    } else if assigned.is_empty() && share {
        // Ask to join the best coop that would take a member, one live request
        // at a time (a request at a coop that has since filled up does not count).
        let pending = world.offers.values().any(|o| match o.body {
            OfferBody::Membership { citizen, org } if citizen == id => world
                .orgs
                .get(&org)
                .is_some_and(|o| crate::coop::would_admit(world, o)),
            _ => false,
        });
        if !pending {
            // The coop whose last share per member was highest; with nothing
            // shared yet anywhere, the least staffed by the balancing weights.
            // Requests already waiting on a coop count as members for staffing,
            // so forty citizens choosing in turn spread out instead of piling up.
            let staffing = |o: &crate::world::Org| {
                let weight: u32 = o
                    .workplaces
                    .iter()
                    .filter_map(|w| world.workplaces.get(w))
                    .map(|w| {
                        world
                            .params
                            .labor
                            .balance_weights
                            .get(&w.kind)
                            .copied()
                            .unwrap_or(1)
                    })
                    .sum();
                let waiting = world
                    .offers
                    .values()
                    .filter(|f| matches!(f.body, OfferBody::Membership { org: x, .. } if x == o.id))
                    .count();
                f64::from(u32::try_from(o.members.len() + waiting).unwrap_or(u32::MAX))
                    / f64::from(weight.max(1))
            };
            let best = world
                .orgs
                .values()
                .filter(|o| {
                    crate::coop::open_places(world, o) > 0
                        && crate::coop::admitting(world, o)
                        && !o.members.contains(&id)
                })
                .max_by(|a, b| {
                    crate::coop::surplus_per_member(a)
                        .partial_cmp(&crate::coop::surplus_per_member(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(
                            staffing(b)
                                .partial_cmp(&staffing(a))
                                .unwrap_or(std::cmp::Ordering::Equal),
                        )
                        .then(b.id.cmp(&a.id))
                });
            if let Some(o) = best {
                cmds.push(Command::RequestMembership { org: o.id });
            }
        }
    } else if assigned.is_empty() {
        let best = world
            .offers
            .values()
            .filter_map(|o| match o.body {
                OfferBody::Employment {
                    workplace,
                    pay,
                    places,
                    ..
                } if places > 0 => {
                    let room = crate::orgs::check_room(world, workplace).is_ok();
                    let crowd = world
                        .workplaces
                        .get(&workplace)
                        .map_or(0, |w| w.workers.len());
                    room.then_some((hourly_equivalent(world, workplace, pay), crowd, o.id))
                }
                _ => None,
            })
            .max_by_key(|(pay, crowd, id)| {
                (*pay, std::cmp::Reverse(*crowd), std::cmp::Reverse(*id))
            });
        if let Some((_, _, offer)) = best {
            cmds.push(Command::AcceptEmployment { offer });
        }
    } else if let Some((workplace, hours, max_hours)) = assigned.first().copied()
        && hours == 0
    {
        let h = max_hours.min(c.labor.budget);
        if h > 0 {
            cmds.push(Command::SetLabor {
                allocations: vec![Allocation {
                    workplace,
                    hours: h,
                    effort: Effort::Normal,
                }],
            });
        }
    }

    // 2b. Mobility (share systems): a member may move to a coop that shares more.
    if share && !assigned.is_empty() {
        // A member whose coop shared out less than a living last cycle moves
        // to one with a place open that shared more (labor-membership is free,
        // GDD 6.5); the steward never leaves (a coop without a manager is dead,
        // and at seeding every steward is alone). Leaving forfeits the share.
        if let Some(mine) = world
            .orgs
            .values()
            .find(|o| o.kind == crate::kinds::OrgKind::Cooperative && o.members.contains(&id))
            && mine.manager != Some(id)
            && crate::coop::tenure_cycles(world, mine, id) >= 2
        {
            let my_share = crate::coop::surplus_per_member(mine);
            let living = living_cost(world, c).as_credits_f64();
            let better = world.orgs.values().any(|o| {
                o.id != mine.id
                    && crate::coop::open_places(world, o) > 0
                    && crate::coop::admitting(world, o)
                    && crate::coop::surplus_per_member(o) > my_share
            });
            if my_share < living && better {
                cmds.push(Command::LeaveOrg { org: mine.id });
            }
        }
    }
    // 3. Housing: the cheapest open lease within 25% of income (or of the legacy wage).
    if c.household.dwelling.is_none() {
        let income = if c.wages_total > Money::ZERO {
            c.last_cycle_wages
        } else {
            Money(
                world.params.money.legacy_wage.0 * i64::from(world.params.labor.base_budget_hours),
            )
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let max_rent =
            Money((income.0 as f64 * world.params.householder.rent_income_fraction_max) as i64);
        let cheapest = world
            .offers
            .values()
            .filter_map(|o| match o.body {
                OfferBody::Lease {
                    asset: LeaseAsset::Dwelling(_),
                    rent_per_cycle,
                    ..
                } if o.by != Party::Citizen(id) && (!money || rent_per_cycle <= max_rent) => {
                    Some((rent_per_cycle, o.id))
                }
                _ => None,
            })
            .min();
        if let Some((_, offer)) = cheapest {
            cmds.push(Command::AcceptLease { offer });
        }
    }

    // 4. Sell surplus: anything that is not Food or Wares, at last price.
    if money && world.rules_order_books() {
        for (g, q) in &c.household.pantry {
            if matches!(g, Good::Food | Good::Wares) || *q == 0 {
                continue;
            }
            let resting = world.books.get(&Instrument::Good(*g)).is_some_and(|b| {
                b.orders
                    .values()
                    .any(|o| o.owner == Party::Citizen(id) && o.side == Side::Ask)
            });
            let price = price_of(world, *g);
            if !resting && price > Money::ZERO {
                cmds.push(Command::PlaceOrder {
                    instrument: Instrument::Good(*g),
                    side: Side::Ask,
                    qty: *q,
                    limit_price: price,
                    expires_tick: None,
                });
            }
        }
    }
    cmds
}

/// The reference price of a good: labor at the legacy wage per unit at the base
/// rate plus reference-priced inputs, times the markup (TDD 5.7 derives the
/// start prices this way). Bottom-up over the commodity graph, so it never
/// depends on a market price and cannot compound (Q44).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn reference_price(world: &World, good: Good) -> Money {
    let producer = WorkplaceKind::ALL
        .iter()
        .find(|k| world.params.recipes[k].produces.as_good() == Some(good));
    let Some(kind) = producer else {
        return world
            .params
            .money
            .start_prices
            .get(&good)
            .copied()
            .unwrap_or(Money::ZERO);
    };
    let recipe = &world.params.recipes[kind];
    let labor = world.params.money.legacy_wage.0 as f64 / recipe.base_rate;
    let inputs: f64 = recipe
        .consumes
        .iter()
        .map(|(g, per)| reference_price(world, *g).0 as f64 * f64::from(*per))
        .sum();
    Money(((labor + inputs) * (1.0 + world.params.householder.legacy_markup)).round() as i64)
        .max(Money(1))
}

/// Cost-plus ask price for a workplace kind (Q40): labor cost per unit at the
/// legacy wage plus inputs at last price, times the markup, never below the
/// start price.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn cost_plus(world: &World, kind: WorkplaceKind) -> Money {
    let recipe = &world.params.recipes[&kind];
    let wage = world.params.money.legacy_wage.0 as f64;
    let labor = wage / recipe.base_rate;
    let inputs: f64 = recipe
        .consumes
        .iter()
        .map(|(g, per)| reference_price(world, *g).0 as f64 * f64::from(*per))
        .sum();
    let price = ((labor + inputs) * (1.0 + world.params.householder.legacy_markup)).round() as i64;
    let floor = recipe.produces.as_good().map_or(0, |g| {
        world.params.money.start_prices.get(&g).map_or(0, |m| m.0)
    });
    Money(price.max(floor).max(1))
}

/// The legacy manager's commands for one org this round (market systems).
#[must_use]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn decide_manager(world: &World, org_id: OrgId) -> Vec<Command> {
    let Some(org) = world.orgs.get(&org_id) else {
        return Vec::new();
    };
    let Some(manager) = org.manager else {
        return Vec::new();
    };
    if world
        .citizens
        .get(&manager)
        .is_none_or(|c| c.kind != CitizenKind::Householder || c.dormant)
    {
        return Vec::new();
    }
    // A collective or state enterprise has no prices to set and nobody to hire;
    // its manager's one job is to put the society's Machines to work (Q61):
    // one per workplace per round, so the stock spreads across the orgs.
    if org.ownership == Ownership::Society {
        let stock = crate::ledger::stock_holder(world, org_id);
        let mut cmds = Vec::new();
        let mut machines = crate::ledger::goods_at(world, stock, Good::Machines);
        for wp_id in &org.workplaces {
            let Some(wp) = world.workplaces.get(wp_id) else {
                continue;
            };
            if wp.kind != WorkplaceKind::MachineShop && machines > 0 {
                cmds.push(Command::InstallMachines {
                    org: org_id,
                    workplace: *wp_id,
                    qty: 1,
                });
                machines -= 1;
            }
        }
        return cmds;
    }
    if !world.constitution.has_money() || !world.rules_order_books() {
        return Vec::new();
    }
    let p = &world.params;
    let me = Party::Org(org_id);
    let mut cmds = Vec::new();
    let coop = org.kind == crate::kinds::OrgKind::Cooperative;
    // Money this round's bids have already committed (a coop keeps no payroll
    // reserve, so the Machine rule below must not count it twice).
    let mut committed = Money::ZERO;
    let wage = p.money.legacy_wage;
    let open_wages: Vec<Money> = world
        .offers
        .values()
        .filter_map(|o| match o.body {
            OfferBody::Employment {
                pay: Pay::Hourly(w),
                places,
                ..
            } if places > 0 => Some(w),
            _ => None,
        })
        .collect();
    let median_wage = if open_wages.is_empty() {
        wage
    } else {
        let mut v = open_wages;
        v.sort();
        v[v.len() / 2]
    };

    for wp_id in &org.workplaces {
        let Some(wp) = world.workplaces.get(wp_id) else {
            continue;
        };
        let recipe = &p.recipes[&wp.kind];
        let workers = u32::try_from(wp.workers.len()).unwrap_or(0);
        let per_worker = Money(median_wage.0 * i64::from(p.labor.base_budget_hours));
        let affordable = if coop {
            u32::MAX
        } else {
            u32::try_from(org.treasury.0 / per_worker.0.max(1)).unwrap_or(0)
        };
        let places = p
            .labor
            .max_workers_per_workplace
            .min(affordable)
            .saturating_sub(workers);
        let has_offer = world.offers.values().any(|o| matches!(o.body, OfferBody::Employment { workplace, places, .. } if workplace == *wp_id && places > 0));
        // Hiring cap (Q2): stop hiring when output stock exceeds N cycles of
        // full production (a Builders' coop: by its dwellings standing empty).
        let glutted = crate::coop::glutted(world, org, wp);
        if coop && places > 0 && !glutted {
            // Admissions instead of hiring (Q86): pending requests, oldest first.
            let mut requests: Vec<(crate::ids::OfferId, CitizenId)> = world
                .offers
                .values()
                .filter_map(|o| match o.body {
                    OfferBody::Membership { org: x, citizen } if x == org_id => {
                        Some((o.id, citizen))
                    }
                    _ => None,
                })
                .collect();
            requests.sort();
            for (_, citizen) in requests.into_iter().take(places as usize) {
                if !crate::labor::has_position(world, citizen) {
                    cmds.push(Command::AdmitMember {
                        org: org_id,
                        citizen,
                    });
                }
            }
        } else if !coop && places > 0 && !has_offer && !glutted {
            cmds.push(Command::OfferEmployment {
                org: org_id,
                workplace: *wp_id,
                pay: Pay::Hourly(median_wage),
                max_hours: p.householder.legacy_offer_max_hours,
                term_cycles: None,
                notice_cycles: p.householder.legacy_offer_notice_cycles,
                places,
            });
        }
        // Input bids (Q1): one cycle of recipe demand at the current headcount.
        let planned = workers.max(2).min(p.labor.max_workers_per_workplace);
        let hours = f64::from(planned) * f64::from(p.labor.base_budget_hours);
        let demand_units = recipe.base_rate * hours;
        // Keep one cycle of payroll in hand; only the rest may sit in input
        // bids. A coop keeps its obligations falling due plus what its members
        // would earn at the legacy wage (Q91): with no reserve at all a Mill
        // spends every credit on Grain and its members share out nothing.
        let wage_equivalent =
            Money(median_wage.0 * i64::from(workers) * i64::from(p.labor.base_budget_hours));
        let reserve = if coop {
            crate::coop::obligations_due(world, org_id) + wage_equivalent
        } else {
            wage_equivalent
        };
        let input_budget = (org.treasury - reserve).max_zero();
        for (g, per) in &recipe.consumes {
            // A glutted coop stops buying inputs: its members carry the loss
            // that a firm's treasury would (Q91).
            if coop && glutted {
                break;
            }
            let need = (demand_units * f64::from(*per)).ceil() as u32;
            let have = org.inventory.get(g).copied().unwrap_or(0) + open_bid_qty(world, me, *g);
            if need > have {
                let limit = Money(
                    (reference_price(world, *g).0 as f64 * (1.0 + p.householder.legacy_markup))
                        .round() as i64,
                );
                let affordable = if limit > Money::ZERO {
                    (input_budget.0 / limit.0) as u32
                } else {
                    0
                };
                let qty = (need - have).min(affordable);
                if qty > 0 && limit > Money::ZERO {
                    committed += Money(limit.0 * i64::from(qty));
                    cmds.push(Command::PlaceOrder {
                        instrument: Instrument::Good(*g),
                        side: Side::Bid,
                        qty,
                        limit_price: limit,
                        expires_tick: None,
                    });
                }
            }
        }
        // Asks for the output at cost-plus, refreshed when the price moves.
        if let Some(out) = recipe.produces.as_good() {
            let held = org.inventory.get(&out).copied().unwrap_or(0);
            let price = cost_plus(world, wp.kind);
            let mine: Vec<&crate::world::Order> = world
                .books
                .get(&Instrument::Good(out))
                .map(|b| {
                    b.orders
                        .values()
                        .filter(|o| o.owner == me && o.side == Side::Ask)
                        .collect()
                })
                .unwrap_or_default();
            let stale = mine.iter().any(|o| o.limit_price != price);
            if stale {
                for o in &mine {
                    cmds.push(Command::CancelOrder { order: o.id });
                }
            }
            if held > 0 && (stale || mine.is_empty()) {
                cmds.push(Command::PlaceOrder {
                    instrument: Instrument::Good(out),
                    side: Side::Ask,
                    qty: held,
                    limit_price: price,
                    expires_tick: None,
                });
            }
        }
        // Machines: buy one when the treasury exceeds N cycles of payroll. A coop
        // has no payroll; it reserves what its members would earn at the legacy
        // wage instead (Q91): keyed to the last share-out, a coop that had none
        // would put every credit into Machines and never share anything.
        let payroll =
            Money(median_wage.0 * i64::from(workers) * i64::from(p.labor.base_budget_hours));
        let threshold =
            Money((payroll.0 as f64 * p.householder.legacy_machine_buy_payroll_mult) as i64);
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let machine_price = Money(
            (reference_price(world, Good::Machines).0 as f64 * (1.0 + p.householder.legacy_markup))
                .round() as i64,
        );
        let free = if coop {
            (org.treasury - committed).max_zero()
        } else {
            org.treasury
        };
        // A coop's Machine rule is keyed to its members, so a coop of one with
        // a steady income would turn every credit into Machines (the Builders'
        // rent did, 350 Machines for one steward): it stops at
        // `max_machines_per_member` per working member (Q98).
        let machines_wanted =
            !coop || wp.machines < workers.saturating_mul(p.coop.max_machines_per_member);
        // A coop that cannot afford one applies to the bank (S0.17c, Q94).
        if coop
            && workers > 0
            && machines_wanted
            && machine_price > Money::ZERO
            && free <= threshold + machine_price
            && open_bid_qty(world, me, Good::Machines) == 0
            && let Some(bank) = crate::bank::bank_org(world)
            && !crate::bank::pending_application(world, org_id)
            && !crate::bank::open_bank_loan(world, bank, org_id)
        {
            cmds.push(Command::RequestBankLoan {
                org: org_id,
                principal: machine_price.min(p.bank.max_loan),
                term_cycles: p.bank.max_term_cycles,
            });
        }
        if workers > 0
            && machines_wanted
            && free > threshold + machine_price
            && machine_price > Money::ZERO
            && open_bid_qty(world, me, Good::Machines) == 0
        {
            cmds.push(Command::PlaceOrder {
                instrument: Instrument::Good(Good::Machines),
                side: Side::Bid,
                qty: 1,
                limit_price: machine_price,
                expires_tick: None,
            });
        }
        // Bought Machines are installed; a Machine Shop sells what it makes.
        if wp.kind != WorkplaceKind::MachineShop
            && let Some(m) = org.inventory.get(&Good::Machines).copied()
            && m > 0
        {
            cmds.push(Command::InstallMachines {
                org: org_id,
                workplace: *wp_id,
                qty: m,
            });
        }
    }
    // Dwellings: keep every unoccupied dwelling on offer at the legacy rent.
    for d in world
        .dwellings
        .values()
        .filter(|d| d.owner == crate::world::Owner::Org(org_id) && d.occupant.is_none())
    {
        let offered = world.offers.values().any(|o| matches!(o.body, OfferBody::Lease { asset: LeaseAsset::Dwelling(x), .. } if x == d.id) || matches!(o.body, OfferBody::Sale { asset: SaleAsset::Dwelling(x), .. } if x == d.id));
        if !offered {
            cmds.push(Command::OfferLease {
                asset: LeaseAsset::Dwelling(d.id),
                rent_per_cycle: p.money.legacy_rent,
                term_cycles: None,
            });
        }
    }
    // For sale at book value: a standing offer of every OrgSelf share.
    if let Ownership::Shares { holdings, .. } = &org.ownership
        && let Some(own) = holdings.get(&ShareHolder::OrgSelf).copied()
        && own > 0
    {
        let book = crate::shares::book_value(world, org);
        let listed = world.offers.values().find_map(|o| match o.body {
            OfferBody::Sale {
                asset: SaleAsset::Shares(x, q),
                price: Price::Money(m),
                ..
            } if x == org_id && o.by == me => Some((o.id, q, m)),
            _ => None,
        });
        match listed {
            None => cmds.push(Command::OfferSale {
                asset: SaleAsset::Shares(org_id, own),
                price: Price::Money(book),
                to: None,
            }),
            Some((offer, _, m)) if (m.0 - book.0).abs() * 10 > book.0.max(1) => {
                cmds.push(Command::CancelSale { offer });
            }
            Some(_) => {}
        }
    }
    cmds
}

/// A scripted command the engine refused: a script bug, reported in full.
#[derive(Clone, Debug)]
pub struct Rejection {
    pub citizen: CitizenId,
    pub org: Option<OrgId>,
    pub command: Command,
    pub reject: crate::command::Reject,
}

/// Run every householder's script (and the scripts of the orgs they manage)
/// once, in citizen order, applying each command's events to `world` as it
/// goes. Returns the events and every rejected command.
#[allow(clippy::semicolon_if_nothing_returned)]
pub fn run_round(world: &mut World, rules: &Rules, tick: Tick) -> (Vec<Event>, Vec<Rejection>) {
    let mut events = Vec::new();
    let mut rejected = Vec::new();
    let ids: Vec<CitizenId> = world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
        .map(|c| c.id)
        .collect();
    for id in ids {
        let mut cmds: Vec<(Option<OrgId>, Command)> =
            decide(world, id).into_iter().map(|c| (None, c)).collect();
        let managed: Vec<OrgId> = world
            .orgs
            .values()
            .filter(|o| o.manager == Some(id))
            .map(|o| o.id)
            .collect();
        for org in managed {
            cmds.extend(
                decide_manager(world, org)
                    .into_iter()
                    .map(|c| (Some(org), c)),
            );
        }
        for (org, command) in cmds {
            let env = Envelope {
                actor: Actor::Citizen(id),
                on_behalf_of: org,
                client_kind: ClientKind::Householder,
                received_at_tick: tick,
                command,
            };
            match handle(world, rules, &env) {
                Ok(evs) => {
                    for e in evs {
                        apply(world, &e);
                        events.push(e);
                    }
                }
                Err(reject) => rejected.push(Rejection {
                    citizen: id,
                    org,
                    command: env.command.clone(),
                    reject,
                }),
            }
        }
    }
    (events, rejected)
}

impl World {
    /// Whether this society trades on order books.
    #[must_use]
    pub fn rules_order_books(&self) -> bool {
        self.constitution.pricing == crate::constitution::Pricing::Market
    }
}

/// Marker so `OrderSource` stays referenced for the plan executor's tests.
#[must_use]
pub const fn manual() -> OrderSource {
    OrderSource::Manual
}
