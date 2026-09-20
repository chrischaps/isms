//! `World` -> wire views (TDD 10.3). Pure functions over a world snapshot and
//! a `Viewer`; no I/O.

use crate::state::clock_of;
use crate::viewer::Viewer;
use isms_api_types::society::{
    AllocationView, BookSummary, BookView, CitizenPublic, CitizenSelfView, ContractView,
    DwellingView, EffortCosts, FirmValuation, FoundingCosts, HouseholdView, LaborView, Level,
    NeedsView, OfferView, OrderView, OrgView, RecipeView, ScoreRow, SkillView, SlotSummary,
    SocietyPulse, WorkerView, WorkplaceView, cents, instrument_name,
};
use isms_core::ids::{CitizenId, OrgId};
use isms_core::kinds::{CitizenKind, Good, WorkplaceKind};
use isms_core::ledger::{Asset, Party};
use isms_core::market::{depth, last_price};
use isms_core::needs::TENTHS;
use isms_core::shares::{book_value, net_worth, self_made, shares_held};
use isms_core::world::{
    Contract, ContractBody, EscrowKey, Instrument, Offer, OfferBody, Owner, Ownership, Side, World,
};
use std::collections::BTreeMap;

fn tenths(v: u16) -> f64 {
    f64::from(v) / f64::from(TENTHS)
}

pub fn needs(world: &World, c: CitizenId) -> Option<NeedsView> {
    let n = &world.citizens.get(&c)?.needs;
    Some(NeedsView {
        food: tenths(n.food),
        shelter: tenths(n.shelter),
        comfort: tenths(n.comfort),
        low_food_ticks_this_cycle: n.low_food_ticks_this_cycle,
    })
}

pub fn dwelling(world: &World, id: isms_core::ids::DwellingId) -> Option<DwellingView> {
    let d = world.dwellings.get(&id)?;
    let rent = d
        .lease
        .and_then(|l| world.contracts.get(&l))
        .and_then(|c| match c.body {
            ContractBody::Lease { rent_per_cycle, .. } => Some(cents(rent_per_cycle)),
            _ => None,
        });
    // The open sale or lease on it (the engine's private `housing::under_offer`, with the id).
    let offer = world
        .offers
        .values()
        .find(|o| match &o.body {
            OfferBody::Sale {
                asset: isms_core::world::SaleAsset::Dwelling(x),
                ..
            }
            | OfferBody::Lease {
                asset: isms_core::world::LeaseAsset::Dwelling(x),
                ..
            } => *x == id,
            _ => false,
        })
        .map(|o| o.id.0);
    Some(DwellingView {
        id: id.0,
        owner: serde_json::to_value(d.owner).unwrap_or_default(),
        occupant: d.occupant.map(|c| c.0),
        lease: d.lease.map(|l| l.0),
        rent_per_cycle: rent,
        offer,
    })
}

pub fn household(world: &World, c: CitizenId) -> Option<HouseholdView> {
    let h = &world.citizens.get(&c)?.household;
    Some(HouseholdView {
        balance: cents(h.balance),
        pantry: h.pantry.clone(),
        pantry_capacity: world.params.pantry.clone(),
        dwelling: h.dwelling.and_then(|d| dwelling(world, d)),
    })
}

pub fn contract(world: &World, viewer: &Viewer, k: &Contract) -> Option<ContractView> {
    let me = viewer.citizen?;
    let party = Party::Citizen(me);
    let is_party = k.parties.0 == party || k.parties.1 == party;
    let managed = [k.parties.0, k.parties.1]
        .iter()
        .any(|p| matches!(p, Party::Org(o) if viewer.manages.contains(o)));
    let role = if is_party {
        "party"
    } else if managed {
        "manager"
    } else {
        return None;
    };
    let _ = world;
    Some(ContractView {
        id: k.id.0,
        parties: serde_json::to_value(k.parties).unwrap_or_default(),
        created_tick: k.created_tick,
        term_cycles: k.term_cycles,
        status: serde_json::to_value(k.status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        body: serde_json::to_value(&k.body).unwrap_or_default(),
        role: role.to_owned(),
    })
}

pub fn contracts(world: &World, viewer: &Viewer) -> Vec<ContractView> {
    world
        .contracts
        .values()
        .filter_map(|k| contract(world, viewer, k))
        .collect()
}

pub fn labor(world: &World, viewer: &Viewer, c: CitizenId) -> Option<LaborView> {
    let citizen = world.citizens.get(&c)?;
    let l = &citizen.labor;
    let allocations = l
        .allocations
        .iter()
        .filter_map(|a| {
            let wp = world.workplaces.get(&a.workplace)?;
            let org = world.orgs.get(&wp.org)?;
            Some(AllocationView {
                workplace: a.workplace.0,
                org: org.id.0,
                org_name: org.name.clone(),
                kind: wp.kind,
                hours: a.hours,
                effort: a.effort,
            })
        })
        .collect();
    let skills = l
        .skill
        .iter()
        .map(|(family, s)| SkillView {
            family: serde_json::to_value(family)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            level: s.level,
            hours: f64::from(u32::try_from(s.tick_hours).unwrap_or(u32::MAX))
                / f64::from(world.ticks_per_cycle()),
        })
        .collect();
    let employment = contracts(world, viewer)
        .into_iter()
        .filter(|k| k.body.get("employment").is_some() && k.role == "party")
        .collect();
    let lp = &world.params.labor;
    Some(LaborView {
        budget: l.budget,
        fatigue_debt: l.fatigue_debt,
        output_mult: l.output_mult,
        allocations,
        skills,
        employment,
        effort: EffortCosts {
            output_mult: [
                lp.effort_output_mult.low,
                lp.effort_output_mult.normal,
                lp.effort_output_mult.high,
            ],
            food_decay_mult: [
                lp.effort_food_decay_mult.low,
                lp.effort_food_decay_mult.normal,
                lp.effort_food_decay_mult.high,
            ],
            high_effort_debt_after_cycles: lp.high_effort_debt_after_cycles,
            high_effort_debt_hours: lp.high_effort_debt_hours,
            max_workplaces: lp.max_workplaces,
        },
    })
}

pub fn citizen_self(world: &World, c: CitizenId) -> Option<CitizenSelfView> {
    let citizen = world.citizens.get(&c)?;
    Some(CitizenSelfView {
        id: c.0,
        handle: citizen.handle.clone(),
        dormant: citizen.dormant,
        joined_tick: citizen.joined_tick,
        last_seen_tick: citizen.last_seen_tick,
        flags: serde_json::to_value(&citizen.flags).unwrap_or_default(),
    })
}

pub fn pulse(world: &World) -> SocietyPulse {
    let mut population = 0;
    let mut active_humans = 0;
    let mut unemployed = 0;
    for c in world.citizens.values() {
        population += 1;
        if c.dormant {
            continue;
        }
        if c.kind == CitizenKind::Human {
            active_humans += 1;
        }
        if c.labor.allocations.is_empty() {
            unemployed += 1;
        }
    }
    SocietyPulse {
        population,
        active_humans,
        price_index: world.price_index,
        food_last_price: last_price(world, Instrument::Good(Good::Food)).map(cents),
        unemployed,
    }
}

// -- market --------------------------------------------------------------------

pub fn instruments(world: &World) -> Vec<Instrument> {
    let mut list: Vec<Instrument> = world.books.keys().copied().collect();
    for good in [Good::Food, Good::Wares] {
        if !list.contains(&Instrument::Good(good)) {
            list.push(Instrument::Good(good));
        }
    }
    list
}

pub fn book_summary(world: &World, i: Instrument) -> BookSummary {
    let bids = depth(world, i, Side::Bid);
    let asks = depth(world, i, Side::Ask);
    BookSummary {
        instrument: instrument_name(i),
        last_price: last_price(world, i).map(cents),
        best_bid: bids.iter().map(|(p, _)| cents(*p)).max(),
        best_ask: asks.iter().map(|(p, _)| cents(*p)).min(),
        bid_depth: bids.iter().map(|(_, q)| *q).sum(),
        ask_depth: asks.iter().map(|(_, q)| *q).sum(),
    }
}

pub fn book(world: &World, viewer: &Viewer, i: Instrument) -> BookView {
    let mine: Vec<Party> = viewer
        .citizen
        .into_iter()
        .map(Party::Citizen)
        .chain(viewer.manages.iter().map(|o| Party::Org(*o)))
        .collect();
    let my_orders = world
        .books
        .get(&i)
        .map(|b| {
            b.orders
                .values()
                .filter(|o| mine.contains(&o.owner))
                .map(|o| OrderView {
                    id: o.id.0,
                    side: serde_json::to_value(o.side)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default(),
                    qty: o.qty,
                    remaining: o.remaining,
                    limit_price: cents(o.limit_price),
                    placed_tick: o.placed_tick,
                    expires_tick: o.expires_tick,
                    source: serde_json::to_value(o.source)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    BookView {
        clock: clock_of(world),
        instrument: instrument_name(i),
        last_price: last_price(world, i).map(cents),
        bids: depth(world, i, Side::Bid)
            .into_iter()
            .map(|(p, q)| Level {
                price: cents(p),
                qty: q,
            })
            .collect(),
        asks: depth(world, i, Side::Ask)
            .into_iter()
            .map(|(p, q)| Level {
                price: cents(p),
                qty: q,
            })
            .collect(),
        my_orders,
        tape: Vec::new(),
    }
}

// -- orgs ----------------------------------------------------------------------

pub fn org(world: &World, viewer: &Viewer, id: OrgId) -> Option<OrgView> {
    let o = world.orgs.get(&id)?;
    let i_manage = viewer.manages.contains(&id);
    let workplaces = o
        .workplaces
        .iter()
        .filter_map(|wp| world.workplaces.get(wp))
        .map(|wp| WorkplaceView {
            id: wp.id.0,
            kind: wp.kind,
            slot: wp.slot.map(|s| s.0),
            machines: wp.machines,
            cycle_output: wp.cycle_output,
            workers: wp
                .workers
                .iter()
                .map(|(c, a)| {
                    let (attributed, _) = viewer.worker_figures(world, wp.id, *c);
                    WorkerView {
                        citizen: c.0,
                        handle: world
                            .citizens
                            .get(c)
                            .map(|z| z.handle.clone())
                            .unwrap_or_default(),
                        hours: a.hours,
                        attributed_this_cycle: attributed.then_some(a.cycle_attributed),
                    }
                })
                .collect(),
        })
        .collect();
    let my_shares = viewer
        .citizen
        .map_or(0, |c| shares_held(world, Party::Citizen(c), id));
    // Money the org's resting bids hold: the treasury is already net of it.
    let escrow = world
        .books
        .values()
        .flat_map(|b| b.orders.values())
        .filter(|r| r.owner == Party::Org(id))
        .filter_map(|r| world.escrow.get(&EscrowKey::Order(r.id)))
        .map(|a| match a {
            Asset::Money(m) => cents(*m),
            Asset::Good(..) => 0,
        })
        .sum();
    let dwellings = world
        .dwellings
        .values()
        .filter(|d| d.owner == Owner::Org(id))
        .filter_map(|d| dwelling(world, d.id))
        .collect();
    Some(OrgView {
        id: id.0,
        kind: o.kind,
        name: o.name.clone(),
        manager: o.manager.map(|c| c.0),
        treasury: cents(o.treasury),
        inventory: o.inventory.clone(),
        ownership: serde_json::to_value(&o.ownership).unwrap_or_default(),
        book_value: cents(book_value(world, o)),
        escrow,
        declared_dividend: o.declared_dividend.map(cents),
        payment_missed: o.payment_missed,
        last_payment_missed: None,
        employees: u32::try_from(o.employees.len()).unwrap_or(u32::MAX),
        members: o.members.iter().map(|c| c.0).collect(),
        workplaces,
        dwellings,
        my_shares,
        i_manage,
    })
}

/// The preset's recipes, by workplace kind, in the wire's names.
pub fn recipes(world: &World) -> Vec<RecipeView> {
    fn name<T: serde::Serialize>(v: T) -> String {
        serde_json::to_value(v)
            .ok()
            .and_then(|j| j.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
    world
        .params
        .recipes
        .iter()
        .map(|(kind, r)| RecipeView {
            workplace_kind: name(kind),
            produces: name(r.produces),
            produces_asset: r.produces.as_good().is_none(),
            consumes: r.consumes.iter().map(|(g, n)| (name(g), *n)).collect(),
            base_rate: r.base_rate,
        })
        .collect()
}

pub fn founding(world: &World) -> FoundingCosts {
    FoundingCosts {
        money: if world.constitution.has_money() {
            cents(world.params.money.founding_cost_money)
        } else {
            0
        },
        materials: world.params.founding.materials,
    }
}

/// Slot-limited kinds only: total slots and how many are still free.
pub fn slots(world: &World) -> BTreeMap<WorkplaceKind, SlotSummary> {
    let mut out: BTreeMap<WorkplaceKind, SlotSummary> = BTreeMap::new();
    for slot in world.land.slots.values() {
        let e = out
            .entry(slot.kind)
            .or_insert(SlotSummary { total: 0, free: 0 });
        e.total += 1;
        if slot.workplace.is_none() {
            e.free += 1;
        }
    }
    out
}

pub fn orgs(world: &World, viewer: &Viewer) -> Vec<OrgView> {
    world
        .orgs
        .keys()
        .filter_map(|id| org(world, viewer, *id))
        .collect()
}

// -- offers --------------------------------------------------------------------

pub fn offer(o: &Offer) -> OfferView {
    let kind = match o.body {
        OfferBody::Employment { .. } => "employment",
        OfferBody::Sale { .. } => "sale",
        OfferBody::Wanted { .. } => "wanted",
        OfferBody::Credit { .. } => "credit",
        OfferBody::Lease { .. } => "lease",
        OfferBody::Membership { .. } => "membership",
        OfferBody::BankLoan { .. } => "bank_loan",
        OfferBody::CollectiveAgreement { .. } => "collective_agreement",
    };
    OfferView {
        id: o.id.0,
        by: serde_json::to_value(o.by).unwrap_or_default(),
        created_tick: o.created_tick,
        kind: kind.to_owned(),
        body: serde_json::to_value(&o.body).unwrap_or_default(),
    }
}

pub fn offers(world: &World) -> Vec<OfferView> {
    world.offers.values().map(offer).collect()
}

// -- society -------------------------------------------------------------------

pub fn citizens_public(world: &World) -> Vec<CitizenPublic> {
    world
        .citizens
        .values()
        .map(|c| CitizenPublic {
            id: c.id.0,
            handle: c.handle.clone(),
            kind: serde_json::to_value(c.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            dormant: c.dormant,
            joined_tick: c.joined_tick,
            flags: serde_json::to_value(&c.flags).unwrap_or_default(),
            honors: isms_core::metrics::honors_of(c),
        })
        .collect()
}

pub fn scoreboard(world: &World) -> Vec<ScoreRow> {
    let mut rows: Vec<ScoreRow> = world
        .citizens
        .values()
        .filter(|c| !c.dormant)
        .map(|c| {
            let firms = world
                .orgs
                .values()
                .filter_map(|o| {
                    let held = shares_held(world, Party::Citizen(c.id), o.id);
                    if held == 0 {
                        return None;
                    }
                    let issued = match &o.ownership {
                        Ownership::Shares { issued, .. } => *issued,
                        _ => 0,
                    };
                    Some(FirmValuation {
                        org: o.id.0,
                        name: o.name.clone(),
                        book_value: cents(book_value(world, o)),
                        shares_held: held,
                        shares_issued: issued,
                    })
                })
                .collect();
            ScoreRow {
                citizen: c.id.0,
                handle: c.handle.clone(),
                net_worth: cents(net_worth(world, c.id)),
                self_made: cents(self_made(world, c.id)),
                firms,
                honors: isms_core::metrics::honors_of(c),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.net_worth
            .cmp(&a.net_worth)
            .then(a.citizen.cmp(&b.citizen))
    });
    rows
}

pub fn firm_count(world: &World) -> u32 {
    u32::try_from(
        world
            .orgs
            .values()
            .filter(|o| o.kind == isms_core::kinds::OrgKind::Firm)
            .count(),
    )
    .unwrap_or(u32::MAX)
}

pub fn credit_outstanding(world: &World) -> i64 {
    world
        .contracts
        .values()
        .filter(|k| k.status == isms_core::world::ContractStatus::Active)
        .map(|k| match k.body {
            ContractBody::Credit {
                installment,
                installments_left,
                ..
            } => installment.0 * i64::from(installments_left),
            _ => 0,
        })
        .sum()
}
