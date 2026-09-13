//! `--detail`: per-citizen, per-org, recipe-flow, trade-tape, goods-movement
//! and order-book-depth CSVs from one run (S0.14e).
//!
//! Everything here is read from the world after each step or folded from the
//! events the loop already holds; the engine is untouched. Iteration is over
//! `BTreeMap`s and ordered event slices, so two runs of the same seed write
//! byte-identical files. The schemas are the contract for tuning reports;
//! `docs/tuning/README.md` "Detail output" lists them.

use crate::Row;
use crate::observer::Observer;
use isms_core::event::Event;
use isms_core::ids::{CitizenId, Cycle, Epoch, OrgId, Tick, WorkplaceId};
use isms_core::kinds::{Effort, Good, JobFamily, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::market::{depth, last_price};
use isms_core::money::Money;
use isms_core::world::{Citizen, Instrument, Ownership, Price, SaleAsset, Side, World};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// Where and how much to write.
#[derive(Clone, Debug)]
pub struct DetailOptions {
    /// Parent directory; each run writes into `<dir>/<preset>-<seed>/`.
    pub dir: PathBuf,
    /// Also write `citizens_ticks.csv` (one row per citizen per tick).
    pub ticks: bool,
}

/// The files a detail run writes, in the order they are listed in the README.
pub const FILES: [&str; 6] = [
    "citizens.csv",
    "orgs.csv",
    "flows.csv",
    "trades.csv",
    "moves.csv",
    "depth.csv",
];

/// The optional seventh file.
pub const TICKS_FILE: &str = "citizens_ticks.csv";

// ---------------------------------------------------------------------------
// Row schemas. Field order is column order.

#[derive(Clone, Debug, Default, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct CitizenRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub citizen: String,
    pub handle: String,
    pub dormant: bool,
    pub food_meter: f64,
    pub shelter_meter: f64,
    pub comfort_meter: f64,
    pub balance_credits: f64,
    pub pantry_food: u32,
    pub pantry_wares: u32,
    pub pantry_other: u32,
    pub cycle_wages_credits: f64,
    pub wages_total_credits: f64,
    pub workplace: String,
    pub workplace_kind: String,
    pub org: String,
    pub contract: String,
    pub alloc_hours: u8,
    pub effort: String,
    pub tick_hours_worked: u64,
    pub attributed_output: f64,
    pub skill_family: String,
    pub skill_level: f64,
    pub skill_max_level: f64,
    pub budget: u8,
    pub fatigue_debt: u8,
    pub output_mult: f64,
    pub food_eaten: u32,
    pub wares_consumed: u32,
    pub housed: bool,
    pub dwelling: String,
    pub in_hardship: bool,
    pub destitute: bool,
    pub defaulted: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CitizenTickRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub tick: Tick,
    pub citizen: String,
    pub food_meter: f64,
    pub shelter_meter: f64,
    pub comfort_meter: f64,
    pub balance_credits: f64,
    pub pantry_food: u32,
    pub pantry_wares: u32,
    pub food_eaten: u32,
    pub wares_consumed: u32,
    pub output_mult: f64,
    pub in_hardship: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct OrgRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub org: String,
    pub name: String,
    pub kind: String,
    pub ownership: String,
    pub manager: String,
    pub treasury_credits: f64,
    pub inv_grain: u32,
    pub inv_ore: u32,
    pub inv_materials: u32,
    pub inv_food: u32,
    pub inv_wares: u32,
    pub inv_machines: u32,
    pub workplaces: u32,
    pub machines: u32,
    pub employees: u32,
    pub positions: u32,
    pub members: u32,
    pub wages_paid_credits: f64,
    pub dividends_paid_credits: f64,
    pub declared_dividend_credits: f64,
    pub payment_missed: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FlowRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub workplace: String,
    pub kind: String,
    pub org: String,
    pub machines: u32,
    pub workers: u32,
    pub tick_hours: u64,
    pub output_good: String,
    pub units: u64,
    pub in_grain: u64,
    pub in_ore: u64,
    pub in_materials: u64,
    pub in_food: u64,
    pub in_wares: u64,
    pub in_machines: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TradeRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub tick: Tick,
    /// `round` (placed by a householder script) or `tick` (standing plan).
    pub phase: String,
    pub instrument: String,
    pub buyer: String,
    pub seller: String,
    pub buy_order: String,
    pub sell_order: String,
    pub qty: u32,
    pub price_credits: f64,
    pub value_credits: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MoveRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub tick: Tick,
    pub kind: String,
    pub from: String,
    pub to: String,
    /// A good, or `credits` for a money-only row (`qty` is then 0).
    pub good: String,
    pub qty: u32,
    pub amount_credits: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DepthRow {
    pub preset: String,
    pub seed: u64,
    pub epoch: Epoch,
    pub cycle: Cycle,
    pub tick: Tick,
    pub instrument: String,
    pub best_bid_credits: Option<f64>,
    pub bid_qty: u32,
    pub bid_levels: u32,
    pub best_ask_credits: Option<f64>,
    pub ask_qty: u32,
    pub ask_levels: u32,
    pub last_price_credits: Option<f64>,
}

// ---------------------------------------------------------------------------
// Labels: snake_case, matching the serde names of the engine's enums.

fn good_label(g: Good) -> &'static str {
    match g {
        Good::Grain => "grain",
        Good::Ore => "ore",
        Good::Materials => "materials",
        Good::Food => "food",
        Good::Wares => "wares",
        Good::Machines => "machines",
    }
}

fn workplace_kind_label(k: WorkplaceKind) -> &'static str {
    match k {
        WorkplaceKind::Farm => "farm",
        WorkplaceKind::Mine => "mine",
        WorkplaceKind::Foundry => "foundry",
        WorkplaceKind::Mill => "mill",
        WorkplaceKind::Workshop => "workshop",
        WorkplaceKind::MachineShop => "machine_shop",
        WorkplaceKind::Builder => "builder",
    }
}

fn family_label(f: JobFamily) -> &'static str {
    match f {
        JobFamily::Farming => "farming",
        JobFamily::Mining => "mining",
        JobFamily::Smelting => "smelting",
        JobFamily::Milling => "milling",
        JobFamily::Crafting => "crafting",
        JobFamily::Machining => "machining",
        JobFamily::Building => "building",
    }
}

fn effort_label(e: Effort) -> &'static str {
    match e {
        Effort::Low => "low",
        Effort::Normal => "normal",
        Effort::High => "high",
    }
}

fn org_kind_label(k: OrgKind) -> &'static str {
    match k {
        OrgKind::Firm => "firm",
        OrgKind::Cooperative => "cooperative",
        OrgKind::Collective => "collective",
        OrgKind::StateEnterprise => "state_enterprise",
        OrgKind::Union => "union",
        OrgKind::Association => "association",
    }
}

fn ownership_label(o: &Ownership) -> &'static str {
    match o {
        Ownership::Shares { .. } => "shares",
        Ownership::Members => "members",
        Ownership::Society => "society",
    }
}

fn party_label(p: Party) -> String {
    match p {
        Party::Citizen(c) => c.to_string(),
        Party::Org(o) => o.to_string(),
    }
}

fn holder_label(h: Holder) -> String {
    match h {
        Holder::Citizen(c) => c.to_string(),
        Holder::Org(o) => o.to_string(),
        Holder::Workplace(w) => w.to_string(),
        Holder::OrderEscrow(r) => format!("escrow:{r}"),
        Holder::ContractEscrow(k) => format!("escrow:{k}"),
        Holder::Store => "store".to_owned(),
        Holder::StateStock => "state_stock".to_owned(),
        Holder::Treasury => "treasury".to_owned(),
    }
}

fn instrument_label(i: Instrument) -> String {
    match i {
        Instrument::Good(g) => good_label(g).to_owned(),
        Instrument::Share(o) => format!("share:{o}"),
    }
}

fn credits(m: Money) -> f64 {
    m.as_credits_f64()
}

fn tenths(v: u16) -> f64 {
    f64::from(v) / 10.0
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Per-cycle accumulators folded from events.

#[derive(Clone, Debug, Default)]
struct CitizenAcc {
    food_eaten: u32,
    wares_consumed: u32,
    attributed_output: f64,
}

#[derive(Clone, Debug, Default)]
struct OrgAcc {
    wages_paid: Money,
    dividends_paid: Money,
}

#[derive(Clone, Debug, Default)]
struct FlowAcc {
    output: Option<Good>,
    units: u64,
    tick_hours: u64,
    inputs: BTreeMap<Good, u64>,
    workers: std::collections::BTreeSet<CitizenId>,
}

/// The `--detail` observer: one `csv::Writer` per file, flushed on `finish`.
pub struct DetailWriter {
    preset: String,
    seed: u64,
    dir: PathBuf,
    citizens: csv::Writer<std::fs::File>,
    ticks: Option<csv::Writer<std::fs::File>>,
    orgs: csv::Writer<std::fs::File>,
    flows: csv::Writer<std::fs::File>,
    trades: csv::Writer<std::fs::File>,
    moves: csv::Writer<std::fs::File>,
    depth: csv::Writer<std::fs::File>,
    /// Sum of `Skill.tick_hours` per citizen at the last cycle close (or
    /// epoch start): hours this cycle are the difference.
    hours_snapshot: BTreeMap<CitizenId, u64>,
    citizen_acc: BTreeMap<CitizenId, CitizenAcc>,
    org_acc: BTreeMap<OrgId, OrgAcc>,
    flow_acc: BTreeMap<WorkplaceId, FlowAcc>,
    /// The first write error, reported by `finish`.
    error: Option<io::Error>,
}

impl std::fmt::Debug for DetailWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetailWriter")
            .field("dir", &self.dir)
            .field("ticks", &self.ticks.is_some())
            .finish_non_exhaustive()
    }
}

/// The column names of a row type, taken from serializing its default value
/// so the header can never drift from the struct.
fn headers<T: Default + Serialize>() -> io::Result<Vec<String>> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.serialize(T::default()).map_err(io::Error::other)?;
    let bytes = w.into_inner().map_err(io::Error::other)?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text
        .lines()
        .next()
        .unwrap_or_default()
        .split(',')
        .map(str::to_owned)
        .collect())
}

/// Open a file and write its header at once, so a file with no rows (the
/// trade tape of a moneyless society) still names its columns.
fn open<T: Default + Serialize>(dir: &Path, name: &str) -> io::Result<csv::Writer<std::fs::File>> {
    let mut w = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(dir.join(name))
        .map_err(io::Error::other)?;
    w.write_record(headers::<T>()?).map_err(io::Error::other)?;
    Ok(w)
}

impl DetailWriter {
    /// Create `<opts.dir>/<preset>-<seed>/` and open the files.
    pub fn create(opts: &DetailOptions, preset: &str, seed: u64) -> io::Result<Self> {
        let dir = opts.dir.join(format!("{preset}-{seed}"));
        std::fs::create_dir_all(&dir)?;
        Ok(DetailWriter {
            preset: preset.to_owned(),
            seed,
            citizens: open::<CitizenRow>(&dir, FILES[0])?,
            orgs: open::<OrgRow>(&dir, FILES[1])?,
            flows: open::<FlowRow>(&dir, FILES[2])?,
            trades: open::<TradeRow>(&dir, FILES[3])?,
            moves: open::<MoveRow>(&dir, FILES[4])?,
            depth: open::<DepthRow>(&dir, FILES[5])?,
            ticks: if opts.ticks {
                Some(open::<CitizenTickRow>(&dir, TICKS_FILE)?)
            } else {
                None
            },
            dir,
            hours_snapshot: BTreeMap::new(),
            citizen_acc: BTreeMap::new(),
            org_acc: BTreeMap::new(),
            flow_acc: BTreeMap::new(),
            error: None,
        })
    }

    /// The run's directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Flush every file and surface the first error met while writing.
    pub fn finish(mut self) -> io::Result<PathBuf> {
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        self.citizens.flush()?;
        self.orgs.flush()?;
        self.flows.flush()?;
        self.trades.flush()?;
        self.moves.flush()?;
        self.depth.flush()?;
        if let Some(t) = self.ticks.as_mut() {
            t.flush()?;
        }
        Ok(self.dir)
    }

    fn note(&mut self, r: csv::Result<()>) {
        if let Err(e) = r
            && self.error.is_none()
        {
            self.error = Some(io::Error::other(e));
        }
    }

    fn snapshot_hours(&mut self, world: &World) {
        self.hours_snapshot = world
            .citizens
            .values()
            .map(|c| (c.id, skill_hours(c)))
            .collect();
    }

    fn fold_event(&mut self, epoch: Epoch, cycle: Cycle, tick: Tick, phase: &str, e: &Event) {
        match e {
            Event::Trade {
                instrument,
                buyer,
                seller,
                buy_order,
                sell_order,
                qty,
                price,
                tick: t,
            } => {
                let row = TradeRow {
                    preset: self.preset.clone(),
                    seed: self.seed,
                    epoch,
                    cycle,
                    tick: *t,
                    phase: phase.to_owned(),
                    instrument: instrument_label(*instrument),
                    buyer: party_label(*buyer),
                    seller: party_label(*seller),
                    buy_order: buy_order.to_string(),
                    sell_order: sell_order.to_string(),
                    qty: *qty,
                    price_credits: credits(*price),
                    value_credits: credits(*price) * f64::from(*qty),
                };
                let r = self.trades.serialize(row);
                self.note(r);
            }
            Event::Produced {
                workplace,
                output,
                units,
                inputs_consumed,
                per_worker,
                ..
            } => {
                let acc = self.flow_acc.entry(*workplace).or_default();
                acc.output = Some(*output);
                acc.units += u64::from(*units);
                for (g, q) in inputs_consumed {
                    *acc.inputs.entry(*g).or_insert(0) += u64::from(*q);
                }
                for w in per_worker {
                    acc.tick_hours += u64::from(w.tick_hours);
                    if w.tick_hours > 0 {
                        acc.workers.insert(w.citizen);
                    }
                    self.citizen_acc
                        .entry(w.citizen)
                        .or_default()
                        .attributed_output += w.attributed_output;
                }
            }
            Event::Paid { org, amount, .. } => {
                self.org_acc.entry(*org).or_default().wages_paid += *amount;
            }
            Event::DividendPaid { org, amount, .. } => {
                self.org_acc.entry(*org).or_default().dividends_paid += *amount;
            }
            Event::TickResolved { citizen_deltas, .. } => {
                for d in citizen_deltas {
                    let acc = self.citizen_acc.entry(d.citizen).or_default();
                    acc.food_eaten += d.food_eaten;
                    acc.wares_consumed += d.wares_consumed;
                }
            }
            other => self.fold_move(epoch, cycle, tick, other),
        }
    }

    /// Goods and money that move outside the order books, into `moves.csv`.
    fn fold_move(&mut self, epoch: Epoch, cycle: Cycle, tick: Tick, e: &Event) {
        match e {
            Event::SaleAccepted {
                buyer,
                seller,
                asset,
                price,
                ..
            } => {
                let (good, qty) = match asset {
                    SaleAsset::Good(g, q) => (good_label(*g).to_owned(), *q),
                    SaleAsset::Shares(o, q) => {
                        (format!("share:{o}"), u32::try_from(*q).unwrap_or(u32::MAX))
                    }
                    SaleAsset::Dwelling(d) => (format!("dwelling:{d}"), 1),
                };
                let amount = match price {
                    Price::Money(m) => credits(*m),
                    Price::Good(..) => 0.0,
                };
                self.push_move(
                    epoch,
                    cycle,
                    tick,
                    "sale_accepted",
                    party_label(*seller),
                    party_label(*buyer),
                    good,
                    qty,
                    amount,
                );
            }
            Event::Drew { citizen, goods, .. } => {
                for (g, q) in goods {
                    self.push_move(
                        epoch,
                        cycle,
                        tick,
                        "drew",
                        "store".to_owned(),
                        citizen.to_string(),
                        good_label(*g).to_owned(),
                        *q,
                        0.0,
                    );
                }
            }
            Event::Seeded { holder, asset } => {
                self.push_asset_move(
                    epoch,
                    cycle,
                    tick,
                    "seeded",
                    "seeded",
                    holder_label(*holder),
                    *asset,
                );
            }
            Event::Transferred {
                from, to, asset, ..
            } => {
                self.push_asset_move(
                    epoch,
                    cycle,
                    tick,
                    "transferred",
                    party_label(*from),
                    party_label(*to),
                    *asset,
                );
            }
            Event::StoreReturned { .. } => self.fold_store_returned(epoch, cycle, tick, e),
            _ => {}
        }
    }

    fn fold_store_returned(&mut self, epoch: Epoch, cycle: Cycle, tick: Tick, e: &Event) {
        if let Event::StoreReturned {
            citizen,
            holder,
            goods,
            money,
        } = e
        {
            for (g, q) in goods {
                self.push_move(
                    epoch,
                    cycle,
                    tick,
                    "store_returned",
                    citizen.to_string(),
                    holder_label(*holder),
                    good_label(*g).to_owned(),
                    *q,
                    0.0,
                );
            }
            if *money != Money::ZERO {
                self.push_move(
                    epoch,
                    cycle,
                    tick,
                    "store_returned",
                    citizen.to_string(),
                    holder_label(*holder),
                    "credits".to_owned(),
                    0,
                    credits(*money),
                );
            }
        }
    }

    fn push_asset_move(
        &mut self,
        epoch: Epoch,
        cycle: Cycle,
        tick: Tick,
        kind: &str,
        from: impl Into<String>,
        to: String,
        asset: Asset,
    ) {
        match asset {
            Asset::Money(m) => self.push_move(
                epoch,
                cycle,
                tick,
                kind,
                from.into(),
                to,
                "credits".to_owned(),
                0,
                credits(m),
            ),
            Asset::Good(g, q) => self.push_move(
                epoch,
                cycle,
                tick,
                kind,
                from.into(),
                to,
                good_label(g).to_owned(),
                q,
                0.0,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_move(
        &mut self,
        epoch: Epoch,
        cycle: Cycle,
        tick: Tick,
        kind: &str,
        from: String,
        to: String,
        good: String,
        qty: u32,
        amount_credits: f64,
    ) {
        let row = MoveRow {
            preset: self.preset.clone(),
            seed: self.seed,
            epoch,
            cycle,
            tick,
            kind: kind.to_owned(),
            from,
            to,
            good,
            qty,
            amount_credits,
        };
        let r = self.moves.serialize(row);
        self.note(r);
    }

    fn write_depth(&mut self, world: &World, epoch: Epoch, cycle: Cycle, tick: Tick) {
        if world.books.is_empty() {
            return;
        }
        let mut instruments: Vec<Instrument> =
            Good::ALL.iter().map(|g| Instrument::Good(*g)).collect();
        instruments.extend(
            world
                .books
                .keys()
                .filter(|i| matches!(i, Instrument::Share(_)))
                .copied(),
        );
        for inst in instruments {
            let bids = depth(world, inst, Side::Bid);
            let asks = depth(world, inst, Side::Ask);
            let row = DepthRow {
                preset: self.preset.clone(),
                seed: self.seed,
                epoch,
                cycle,
                tick,
                instrument: instrument_label(inst),
                best_bid_credits: bids.first().map(|(p, _)| credits(*p)),
                bid_qty: bids.iter().map(|(_, q)| *q).sum(),
                bid_levels: u32::try_from(bids.len()).unwrap_or(u32::MAX),
                best_ask_credits: asks.first().map(|(p, _)| credits(*p)),
                ask_qty: asks.iter().map(|(_, q)| *q).sum(),
                ask_levels: u32::try_from(asks.len()).unwrap_or(u32::MAX),
                last_price_credits: last_price(world, inst).map(credits),
            };
            let r = self.depth.serialize(row);
            self.note(r);
        }
    }

    fn write_ticks(
        &mut self,
        world: &World,
        epoch: Epoch,
        cycle: Cycle,
        tick: Tick,
        tick_events: &[Event],
    ) {
        let Some(Event::TickResolved { citizen_deltas, .. }) = tick_events
            .iter()
            .rev()
            .find(|e| matches!(e, Event::TickResolved { .. }))
        else {
            return;
        };
        let mut rows = Vec::with_capacity(citizen_deltas.len());
        for d in citizen_deltas {
            let Some(c) = world.citizens.get(&d.citizen) else {
                continue;
            };
            rows.push(CitizenTickRow {
                preset: self.preset.clone(),
                seed: self.seed,
                epoch,
                cycle,
                tick,
                citizen: c.id.to_string(),
                food_meter: tenths(d.needs.food),
                shelter_meter: tenths(d.needs.shelter),
                comfort_meter: tenths(d.needs.comfort),
                balance_credits: credits(c.household.balance),
                pantry_food: pantry(c, Good::Food),
                pantry_wares: pantry(c, Good::Wares),
                food_eaten: d.food_eaten,
                wares_consumed: d.wares_consumed,
                output_mult: d.output_mult,
                in_hardship: c.flags.in_hardship,
            });
        }
        if let Some(w) = self.ticks.as_mut() {
            for row in rows {
                if let Err(e) = w.serialize(row)
                    && self.error.is_none()
                {
                    self.error = Some(io::Error::other(e));
                }
            }
        }
    }

    fn write_citizens(&mut self, world: &World, epoch: Epoch, cycle: Cycle) {
        let mut rows = Vec::with_capacity(world.citizens.len());
        for c in world.citizens.values() {
            let acc = self.citizen_acc.get(&c.id).cloned().unwrap_or_default();
            let worked =
                skill_hours(c).saturating_sub(self.hours_snapshot.get(&c.id).copied().unwrap_or(0));
            let pos = position(world, c);
            let (workplace, workplace_kind, org, contract, alloc_hours, effort, family) = match pos
            {
                Some(p) => (
                    p.workplace.to_string(),
                    workplace_kind_label(p.kind).to_owned(),
                    p.org.to_string(),
                    opt(p.contract),
                    p.hours,
                    p.effort.map(effort_label).unwrap_or_default().to_owned(),
                    Some(p.kind.job_family()),
                ),
                None => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    0,
                    String::new(),
                    None,
                ),
            };
            let skill_level = family
                .and_then(|f| c.labor.skill.get(&f))
                .map_or(0.0, |s| s.level);
            let skill_max_level = c.labor.skill.values().map(|s| s.level).fold(0.0, f64::max);
            let other: u32 = c
                .household
                .pantry
                .iter()
                .filter(|(g, _)| !matches!(g, Good::Food | Good::Wares))
                .map(|(_, q)| *q)
                .sum();
            rows.push(CitizenRow {
                preset: self.preset.clone(),
                seed: self.seed,
                epoch,
                cycle,
                citizen: c.id.to_string(),
                handle: c.handle.clone(),
                dormant: c.dormant,
                food_meter: tenths(c.needs.food),
                shelter_meter: tenths(c.needs.shelter),
                comfort_meter: tenths(c.needs.comfort),
                balance_credits: credits(c.household.balance),
                pantry_food: pantry(c, Good::Food),
                pantry_wares: pantry(c, Good::Wares),
                pantry_other: other,
                cycle_wages_credits: credits(c.last_cycle_wages),
                wages_total_credits: credits(c.wages_total),
                workplace,
                workplace_kind,
                org,
                contract,
                alloc_hours,
                effort,
                tick_hours_worked: worked,
                attributed_output: acc.attributed_output,
                skill_family: family.map(family_label).unwrap_or_default().to_owned(),
                skill_level,
                skill_max_level,
                budget: c.labor.budget,
                fatigue_debt: c.labor.fatigue_debt,
                output_mult: c.labor.output_mult,
                food_eaten: acc.food_eaten,
                wares_consumed: acc.wares_consumed,
                housed: c.household.dwelling.is_some(),
                dwelling: opt(c.household.dwelling),
                in_hardship: c.flags.in_hardship,
                destitute: c.flags.destitute,
                defaulted: c.flags.defaulted,
            });
        }
        for row in rows {
            let r = self.citizens.serialize(row);
            self.note(r);
        }
    }

    fn write_orgs(&mut self, world: &World, epoch: Epoch, cycle: Cycle) {
        let mut rows = Vec::with_capacity(world.orgs.len());
        for o in world.orgs.values() {
            let acc = self.org_acc.get(&o.id).cloned().unwrap_or_default();
            let inv = |g: Good| o.inventory.get(&g).copied().unwrap_or(0);
            let wps: Vec<_> = o
                .workplaces
                .iter()
                .filter_map(|w| world.workplaces.get(w))
                .collect();
            rows.push(OrgRow {
                preset: self.preset.clone(),
                seed: self.seed,
                epoch,
                cycle,
                org: o.id.to_string(),
                name: o.name.clone(),
                kind: org_kind_label(o.kind).to_owned(),
                ownership: ownership_label(&o.ownership).to_owned(),
                manager: opt(o.manager),
                treasury_credits: credits(o.treasury),
                inv_grain: inv(Good::Grain),
                inv_ore: inv(Good::Ore),
                inv_materials: inv(Good::Materials),
                inv_food: inv(Good::Food),
                inv_wares: inv(Good::Wares),
                inv_machines: inv(Good::Machines),
                workplaces: u32::try_from(o.workplaces.len()).unwrap_or(u32::MAX),
                machines: wps.iter().map(|w| w.machines).sum(),
                employees: u32::try_from(o.employees.len()).unwrap_or(u32::MAX),
                positions: u32::try_from(wps.iter().map(|w| w.workers.len()).sum::<usize>())
                    .unwrap_or(u32::MAX),
                members: u32::try_from(o.members.len()).unwrap_or(u32::MAX),
                wages_paid_credits: credits(acc.wages_paid),
                dividends_paid_credits: credits(acc.dividends_paid),
                declared_dividend_credits: o.declared_dividend.map_or(0.0, credits),
                payment_missed: o.payment_missed,
            });
        }
        for row in rows {
            let r = self.orgs.serialize(row);
            self.note(r);
        }
    }

    fn write_flows(&mut self, world: &World, epoch: Epoch, cycle: Cycle) {
        let mut rows = Vec::with_capacity(world.workplaces.len());
        for w in world.workplaces.values() {
            let acc = self.flow_acc.get(&w.id).cloned().unwrap_or_default();
            let inp = |g: Good| acc.inputs.get(&g).copied().unwrap_or(0);
            rows.push(FlowRow {
                preset: self.preset.clone(),
                seed: self.seed,
                epoch,
                cycle,
                workplace: w.id.to_string(),
                kind: workplace_kind_label(w.kind).to_owned(),
                org: w.org.to_string(),
                machines: w.machines,
                workers: u32::try_from(acc.workers.len()).unwrap_or(u32::MAX),
                tick_hours: acc.tick_hours,
                output_good: acc.output.map(good_label).unwrap_or_default().to_owned(),
                units: acc.units,
                in_grain: inp(Good::Grain),
                in_ore: inp(Good::Ore),
                in_materials: inp(Good::Materials),
                in_food: inp(Good::Food),
                in_wares: inp(Good::Wares),
                in_machines: inp(Good::Machines),
            });
        }
        for row in rows {
            let r = self.flows.serialize(row);
            self.note(r);
        }
    }
}

fn pantry(c: &Citizen, g: Good) -> u32 {
    c.household.pantry.get(&g).copied().unwrap_or(0)
}

/// Cumulative tick-hours over every skill family; never reset within an epoch.
fn skill_hours(c: &Citizen) -> u64 {
    c.labor.skill.values().map(|s| s.tick_hours).sum()
}

struct Position {
    workplace: WorkplaceId,
    kind: WorkplaceKind,
    org: OrgId,
    contract: Option<isms_core::ids::ContractId>,
    hours: u8,
    effort: Option<Effort>,
}

/// The citizen's primary position: the allocation with the most hours (first
/// wins on ties), else any workplace that lists the citizen as a worker.
fn position(world: &World, c: &Citizen) -> Option<Position> {
    let best =
        c.labor.allocations.iter().fold(
            None::<&isms_core::world::Allocation>,
            |best, a| match best {
                Some(b) if b.hours >= a.hours => Some(b),
                _ => Some(a),
            },
        );
    if let Some(a) = best {
        let w = world.workplaces.get(&a.workplace)?;
        let asg = w.workers.get(&c.id);
        return Some(Position {
            workplace: w.id,
            kind: w.kind,
            org: w.org,
            contract: asg.and_then(|x| x.contract),
            hours: a.hours,
            effort: Some(a.effort),
        });
    }
    world.workplaces.values().find_map(|w| {
        w.workers.get(&c.id).map(|asg| Position {
            workplace: w.id,
            kind: w.kind,
            org: w.org,
            contract: asg.contract,
            hours: asg.hours,
            effort: Some(asg.effort),
        })
    })
}

impl Observer for DetailWriter {
    fn epoch_started(&mut self, world: &World, epoch: Epoch, events: &[Event]) {
        self.snapshot_hours(world);
        self.citizen_acc.clear();
        self.org_acc.clear();
        self.flow_acc.clear();
        for e in events {
            self.fold_event(epoch, 0, world.meta.tick, "epoch", e);
        }
    }

    fn tick_done(
        &mut self,
        world: &World,
        epoch: Epoch,
        tick: Tick,
        round: &[Event],
        tick_events: &[Event],
    ) {
        let cycle = world.cycle_of(tick);
        for e in round {
            self.fold_event(epoch, cycle, tick, "round", e);
        }
        for e in tick_events {
            self.fold_event(epoch, cycle, tick, "tick", e);
        }
        self.write_depth(world, epoch, cycle, tick);
        if self.ticks.is_some() {
            self.write_ticks(world, epoch, cycle, tick, tick_events);
        }
    }

    fn cycle_closed(&mut self, world: &World, epoch: Epoch, cycle: Cycle, _row: &Row) {
        self.write_citizens(world, epoch, cycle);
        self.write_orgs(world, epoch, cycle);
        self.write_flows(world, epoch, cycle);
        self.snapshot_hours(world);
        self.citizen_acc.clear();
        self.org_acc.clear();
        self.flow_acc.clear();
    }

    fn finished(&mut self, _world: &World) {}
}
