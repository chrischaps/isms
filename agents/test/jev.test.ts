// The Jev brain over recorded views (SJ.1): what it lays out, what it sends,
// and what it does with an answer. A fixture stands in for the decisions API.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeClient, type HomeView } from "../src/api/client.ts";
import { fixtureName, fixtureTransport, type Transport } from "../src/api/transport.ts";
import { layOut } from "../src/brain/jev/candidates.ts";
import { JevBrain } from "../src/brain/jev/index.ts";
import { buildQuestions } from "../src/brain/jev/questions.ts";
import { buildState, standingOf, standingText } from "../src/brain/jev/state.ts";
import { jobOffers, Script } from "../src/brain/scripted/script.ts";
import { Budget } from "../src/budget/budget.ts";
import { ConfigSchema, type Config } from "../src/config.ts";
import { loadPersona } from "../src/player/persona.ts";
import { jevSection } from "../src/report/report.ts";
import { newTurn, type ToolContext } from "../src/tools/context.ts";
import type { TurnRecord } from "../src/journal/journal.ts";

const FIX = join(import.meta.dirname, "fixtures", "api");
const BASE = "http://127.0.0.1:18090";
const JEV = "POST /api/alpha/decisions";
const fixture = <T>(key: string): T => (JSON.parse(readFileSync(join(FIX, fixtureName(key)), "utf8")) as { body: T }).body;
const persona = (slug: string) => loadPersona(join(import.meta.dirname, "..", "personas", `${slug}.md`));
const ok = { status: 200, body: { clock: {}, events: [] } };
const board = () => fixture<{ offers: Parameters<typeof jobOffers>[0] }>("GET /s/1/notice-board");

type Overrides = Parameters<typeof fixtureTransport>[1];

function ctx(transport: Transport): ToolContext {
  const client = makeClient({ baseUrl: BASE, auth: { key: "k" }, transport });
  return { client, sid: 1, turn: newTurn(4), now: () => 0 };
}

function unemployedHome(): HomeView {
  const home = fixture<HomeView>("GET /s/1/home");
  return { ...home, labor: { ...home.labor, employment: [], allocations: [] } };
}

/** The recorded view with a contract at `hourly` (the board's best by default). */
function employedHome(hourly?: number): HomeView {
  const home = fixture<HomeView>("GET /s/1/home");
  const best = jobOffers(board().offers)[0]!;
  const contract = {
    id: 1,
    body: { employment: { org: best.org, workplace: best.workplace, pay: { hourly: hourly ?? best.hourly }, max_hours: best.maxHours, notice_cycles: 1, places: best.places, term_cycles: null } },
    created_tick: 0,
    parties: {},
    role: "party",
    status: "active",
    term_cycles: null,
  } as unknown as HomeView["labor"]["employment"][number];
  return { ...home, labor: { ...home.labor, employment: [contract], allocations: [] } };
}

/** A canned decisions answer: `choice` per slot with a confidence, the mass on the choice. */
function answer(picks: Record<string, [string, number]>, extra: Record<string, unknown> = {}) {
  const answers: Record<string, unknown> = {};
  for (const [slot, [choice, confidence]] of Object.entries(picks)) {
    answers[slot] = { type: "choice", choice, probabilities: { [choice]: confidence }, confidence };
  }
  return { status: 200, body: { model: "jev-1.13.0", answers: { ...answers, ...extra }, usage: { input_tokens: 300, output_tokens: 0, cost: 0.0000126 } } };
}

function brain(slug: string, overrides: Overrides, opts: { budget?: Budget; transport?: Transport; cfg?: Config } = {}) {
  const transport = opts.transport ?? fixtureTransport(FIX, overrides);
  const cfg = opts.cfg ?? ConfigSchema.parse({});
  const b = new JevBrain({ persona: persona(slug), player: `${slug}-1`, handle: slug, cfg, budget: opts.budget ?? new Budget(1_000_000, 1), key: "or-key", transport });
  return { brain: b, ctx: ctx(transport) };
}

const turnInput = (home: HomeView) => ({ clock: home.clock, home, notes: "", lastTurn: null });

// -- SJ.3: a manager's own workplace, a firm with hands, a builder with a roof, a lender ----------

type OrgsBody = { orgs: { id: number; i_manage: boolean; workplaces: { id: number; kind: string; workers: unknown[]; last_cycle_output?: number }[]; dwellings: unknown[]; treasury: number; inventory: Record<string, number> }[] };
const orgsView = () => fixture<OrgsBody>("GET /s/1/orgs");
/** The recorded orgs with founder-1's Works (org 18, a mine at workplace 18) as the caller's own, amended as the test needs. */
function managedOrgs(patch: { kind?: string; workers?: { citizen: number; handle: string; hours: number }[]; last_cycle_output?: number; dwellings?: unknown[]; treasury?: number; inventory?: Record<string, number> } = {}) {
  const v = orgsView();
  const mine = v.orgs.find((o) => o.id === 18)!;
  mine.i_manage = true;
  mine.treasury = patch.treasury ?? 0;
  mine.inventory = patch.inventory ?? {};
  mine.dwellings = patch.dwellings ?? [];
  const wp = mine.workplaces[0]!;
  wp.kind = patch.kind ?? "mine";
  wp.workers = (patch.workers ?? []).map((w) => ({ ...w, attributed_this_cycle: null }));
  wp.last_cycle_output = patch.last_cycle_output ?? 0;
  return { status: 200, body: v };
}
const books = (prices: Record<string, number>) => ({ status: 200, body: { books: Object.entries(prices).map(([instrument, last_price]) => ({ instrument, last_price, best_ask: null, best_bid: null })), price_index: 1 } });
/** A home with hours set at the wage job, so `work` asks only for a reason. */
function workingHome(): HomeView {
  const home = employedHome();
  const best = jobOffers(board().offers)[0]!;
  return { ...home, household: { ...home.household, balance: 20000 }, labor: { ...home.labor, allocations: [{ workplace: best.workplace, hours: best.maxHours, effort: "normal", org_name: "x", kind: "farm" }] } } as HomeView;
}
const firm = { org: 18, workplace: 18, kind: "mine", produces: "ore", base_rate: 10, consumes: {}, inventory: {}, price: 97 };

/** A founder at home: the position founding granted at its own workplace (E-3, Q160), held with no contract, beside a wage job. */
function founderHome(): HomeView {
  const home = workingHome();
  const position = { workplace: 18, org: 18, org_name: "Iron & Sons", kind: "mine", contract: null } as unknown as NonNullable<HomeView["labor"]["positions"]>[number];
  return { ...home, labor: { ...home.labor, positions: [position] } };
}

describe("a manager's own workplace (SJ.3)", () => {
  it("offers work_own once a day when the firm slot has seen a firm and no hours are there, and puts the hours at the position founding granted (E-3, Q160)", async () => {
    const home = founderHome();
    const c = ctx(fixtureTransport(FIX));
    const script = new Script(c, turnInput(home), { firm }, "f");
    const slots = await layOut("founder", script);
    const work = slots.find((s) => s.key === "work")!;
    expect(work.candidates.map((x) => x.option)).toEqual(["work_own", "keep_hours"]);
    // Housed at output x0.7 in the recorded view: 10 an hour x 8 h x 0.7 = 56 ore, at 0.97.
    expect(work.candidates[0]!.describe).toContain("about 56 ore a day at your rate, worth 54.32");
    expect(work.candidates[0]!.describe).toContain("unpaid");
    expect(work.facts.own_workplace).toMatchObject({ kind: "mine", hours_there: 0, day_output_units: 56 });
    // Declined, it is not re-asked until tomorrow.
    expect((await layOut("founder", script)).some((s) => s.key === "work")).toBe(false);
    // Taken: the hours go straight to the position; nothing is posted or accepted (the token-wage self-hire is gone).
    const { brain: b, ctx: c2 } = brain("founder", { "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, "GET /s/1/orgs": managedOrgs(), "GET /s/1/books": books({ ore: 97 }), [JEV]: answer({ work: ["work_own", 0.9], venture: ["run_quietly", 0.9] }) });
    (b as unknown as { memory: Record<string, unknown> }).memory.firm = firm;
    const out = await b.takeTurn(c2, turnInput(home));
    expect(out.error).toBeNull();
    expect(c2.turn.calls.some((x) => x.tool === "accept_offer" || x.tool === "post_employment_offer")).toBe(false);
    expect(c2.turn.calls.find((x) => x.tool === "set_labor")?.input).toEqual({ allocations: [{ workplace: 18, hours: 8, effort: "normal" }] });
    expect(out.intent).toContain("work work_own (0.90): set 8 h at my own mine");
  });
  it("offers no work_own without a position at the firm, and caps the day's output by the inputs the firm holds", async () => {
    const roofs = { ...firm, kind: "builder", produces: "dwelling", base_rate: 0.5, consumes: { materials: 10 }, inventory: { materials: 10 }, price: null };
    // No position: a firm founded without a workplace, or one added later, grants none (Q160).
    const without = await layOut("builder", new Script(ctx(fixtureTransport(FIX)), turnInput(workingHome()), { firm: roofs }, "b"));
    expect(without.find((s) => s.key === "work")?.candidates.map((x) => x.option) ?? []).not.toContain("work_own");
    // 0.5 x 8 x 0.7 = 2.8 -> 2 dwellings by rate, but the ten Materials held make one.
    const slots = await layOut("builder", new Script(ctx(fixtureTransport(FIX)), turnInput(founderHome()), { firm: roofs }, "b"));
    expect(slots.find((s) => s.key === "work")!.candidates[0]!.describe).toContain("about 1 dwelling a day at your rate, the firm's to sell");
  });
  it("gives an owner with no wage job the whole day at its own workplace", async () => {
    const home = founderHome();
    const alone = { ...home, labor: { ...home.labor, employment: [], allocations: [] } } as HomeView;
    const slots = await layOut("founder", new Script(ctx(fixtureTransport(FIX)), turnInput(alone), { firm }, "f"));
    const work = slots.find((s) => s.key === "work")!;
    expect(work.candidates.map((x) => x.option)).toContain("work_own");
    expect(work.candidates.find((x) => x.option === "work_own")!.describe).not.toContain("your job pays");
    const full = work.candidates.find((x) => x.option === "full_normal")!;
    const c = ctx(fixtureTransport(FIX, { "PUT /s/1/labor": ok }));
    await full.act!(new Script(c, turnInput(alone), { firm }, "f"));
    expect(c.turn.calls.find((x) => x.tool === "set_labor")?.input).toMatchObject({ allocations: [{ workplace: 18, effort: "normal" }] });
  });
});

describe("the firm with hands (SJ.3)", () => {
  const hands = [{ citizen: 42, handle: "speculator-1", hours: 8 }, { citizen: 43, handle: "saver-1", hours: 8 }];
  it("says what the hands cost against yesterday's output, and offers lay_off only when wages outran it after a full day", async () => {
    const home = workingHome();
    const lossy = { "GET /s/1/orgs": managedOrgs({ workers: hands, last_cycle_output: 4 }), "GET /s/1/books": books({ ore: 97 }) };
    // Hired two days ago: a full day of the hands' work is on the record.
    const hired = { firm, hiredCycle: home.clock.cycle - 2, jobPosted: true };
    const slots = await layOut("founder", new Script(ctx(fixtureTransport(FIX, lossy)), turnInput(home), { ...hired }, "f"));
    const venture = slots.find((s) => s.key === "venture")!;
    expect(venture.candidates.map((x) => x.option)).toContain("lay_off");
    expect(venture.ask).toContain("2 hired hand(s) at its mine, costing about 128.00 a day; yesterday it made 4 ore, worth 3.88 at the last price");
    expect(venture.facts).toMatchObject({ workers: 2, wages_per_day: "128.00", yesterday_output_units: 4, yesterday_output_value: "3.88" });
    // Output worth more than the wages: no lay-off.
    const paying = { ...lossy, "GET /s/1/orgs": managedOrgs({ workers: hands, last_cycle_output: 160 }) };
    const rich = await layOut("founder", new Script(ctx(fixtureTransport(FIX, paying)), turnInput(home), { ...hired }, "f"));
    expect(rich.find((s) => s.key === "venture")?.candidates.map((x) => x.option) ?? []).not.toContain("lay_off");
    // Hired only yesterday: the record is not a full day's yet.
    const fresh = await layOut("founder", new Script(ctx(fixtureTransport(FIX, lossy)), turnInput(home), { ...hired, hiredCycle: home.clock.cycle - 1 }, "f"));
    expect(fresh.find((s) => s.key === "venture")?.candidates.map((x) => x.option) ?? []).not.toContain("lay_off");
    // No price for the output yet: nothing to weigh the wages against.
    const unpriced = await layOut("founder", new Script(ctx(fixtureTransport(FIX, { ...lossy, "GET /s/1/books": books({}) })), turnInput(home), { ...hired }, "f"));
    expect(unpriced.find((s) => s.key === "venture")?.candidates.map((x) => x.option) ?? []).not.toContain("lay_off");
  });
  it("ends the dearest hand's contract on the firm's behalf", async () => {
    const home = workingHome();
    const contract = (id: number, citizen: number, hourly: number) => ({ id, parties: [{ org: 18 }, { citizen }], created_tick: 0, term_cycles: null, status: "active", role: "manager", body: { employment: { org: 18, workplace: 18, pay: { hourly }, max_hours: 8, notice_cycles: 1, places: 2, term_cycles: null } } });
    const { brain: b, ctx: c } = brain("founder", {
      "PUT /s/1/plan": ok,
      "GET /s/1/orgs": managedOrgs({ workers: hands, last_cycle_output: 4 }),
      "GET /s/1/books": books({ ore: 97 }),
      "GET /s/1/contracts": { status: 200, body: { clock: home.clock, contracts: [contract(55, 42, 800), contract(56, 43, 920), { ...contract(57, 42, 990), status: "ended" }] } },
      "POST /s/1/contracts/56/terminate": ok,
      [JEV]: answer({ venture: ["lay_off", 0.8] }),
    });
    const m = (b as unknown as { memory: Record<string, unknown> }).memory;
    m.firm = firm;
    m.hiredCycle = home.clock.cycle - 2;
    m.jobPosted = true;
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.error).toBeNull();
    expect(c.turn.calls.find((x) => x.tool === "terminate_contract")?.input).toEqual({ contract: 56, on_behalf_of: 18 });
    expect(out.intent).toContain("laid off a hand at 9.20/h");
    expect(m.jobPosted).toBe(false);
  });
});

describe("the builder's roof (SJ.3)", () => {
  const roof = { id: 60, owner: { org: 18 }, occupant: null, lease: null, rent_per_cycle: null, offer: null };
  const built = () => ({ "GET /s/1/orgs": managedOrgs({ kind: "builder", dwellings: [roof], treasury: 5000 }), "GET /s/1/books": books({ materials: 200 }) });
  it("asks sell or let for a finished, unoffered dwelling, with the sale price and the rent worked out", async () => {
    const home = workingHome();
    const slots = await layOut("builder", new Script(ctx(fixtureTransport(FIX, built())), turnInput(home), {}, "b"));
    const d = slots.find((s) => s.key === "dwelling")!;
    expect(d.candidates.map((x) => x.option)).toEqual(["sell_dwelling", "lease_dwelling", "hold_dwelling"]);
    // Ten Materials at 2.00 and two worker-hours at the board's median wage, plus a fifth.
    const wage = jobOffers(board().offers).map((o) => o.hourly).sort((a, b) => a - b);
    const median = wage.length % 2 ? wage[Math.floor(wage.length / 2)]! : Math.round((wage[wage.length / 2 - 1]! + wage[wage.length / 2]!) / 2);
    const price = Math.round((2000 + 2 * median) * 1.2);
    expect(d.facts).toMatchObject({ finished_unoffered: 1, sale_price: (price / 100).toFixed(2), rent_per_day: "8.00" });
    // Offered or occupied, it is not asked about again; no dwelling, no slot.
    const offered = await layOut("builder", new Script(ctx(fixtureTransport(FIX, { ...built(), "GET /s/1/orgs": managedOrgs({ kind: "builder", dwellings: [{ ...roof, offer: 5 }] }) })), turnInput(home), {}, "b"));
    expect(offered.some((s) => s.key === "dwelling")).toBe(false);
    const none = await layOut("builder", new Script(ctx(fixtureTransport(FIX, { ...built(), "GET /s/1/orgs": managedOrgs({ kind: "builder" }) })), turnInput(home), {}, "b"));
    expect(none.some((s) => s.key === "dwelling")).toBe(false);
    // The firm slot the builder shares with the founder names its inputs.
    const venture = slots.find((s) => s.key === "venture")!;
    expect(venture.candidates.map((x) => x.option)).toEqual(expect.arrayContaining(["take_ask_materials", "bid_under_materials"]));
    expect(venture.candidates.find((x) => x.option === "take_ask_materials")!.describe).toContain("10 materials, one dwelling's worth");
  });
  it("posts the sale, or the lease, on the firm's behalf", async () => {
    const home = workingHome();
    const sell = brain("builder", { "PUT /s/1/plan": ok, ...built(), "POST /s/1/offers/sale": ok, [JEV]: answer({ dwelling: ["sell_dwelling", 0.8], venture: ["run_quietly", 0.9] }) });
    const sold = await sell.brain.takeTurn(sell.ctx, turnInput(home));
    expect(sold.error).toBeNull();
    const sale = sell.ctx.turn.calls.find((x) => x.tool === "post_sale_offer")?.input as { price: { money: number } };
    expect(sale).toMatchObject({ asset: { dwelling: 60 }, to: null, on_behalf_of: 18 });
    expect(sale.price.money).toBeGreaterThan(2000);
    const lease = brain("builder", { "PUT /s/1/plan": ok, ...built(), "POST /s/1/offers/lease": ok, [JEV]: answer({ dwelling: ["lease_dwelling", 0.8], venture: ["run_quietly", 0.9] }) });
    await lease.brain.takeTurn(lease.ctx, turnInput(home));
    expect(lease.ctx.turn.calls.find((x) => x.tool === "post_lease_offer")?.input).toEqual({ asset: { dwelling: 60 }, rent_per_cycle: 800, term_cycles: null, on_behalf_of: 18 });
  });
});

describe("the lender (SJ.3)", () => {
  const roll = { status: 200, body: { clock: {}, citizens: [{ id: 1, handle: "H-1", kind: "householder", dormant: false, joined_tick: 0, flags: { defaulted: false } }, { id: 2, handle: "H-2", kind: "householder", dormant: false, joined_tick: 0, flags: { defaulted: true } }] } };
  it("offers a quarter of the balance at two rates, once a day, with the roll's defaults in the question", async () => {
    const home = unemployedHome();
    const script = new Script(ctx(fixtureTransport(FIX, { "GET /s/1/citizens": roll })), turnInput(home), {}, "l");
    const slots = await layOut("lender", script);
    const lend = slots.find((s) => s.key === "lend")!;
    expect(lend.candidates.map((x) => x.option)).toEqual(["lend_cheap", "lend_dear", "hold_money"]);
    expect(lend.candidates[0]!.irreversible).toBe(true);
    // 968.56 / 4 in whole credits: 242.00; five days at 1% earn 12.10, at 3% 36.30.
    expect(lend.ask).toContain("a quarter of it, 242.00, could go out for 5 days");
    expect(lend.ask).toContain("At 1% a day it would earn 12.10; at 3%, 36.30");
    expect(lend.ask).toContain("Of 2 citizens on the roll, 1 have defaulted");
    expect(lend.facts).toMatchObject({ principal: "242.00", defaulted: 1, citizens_on_roll: 2 });
    expect((await layOut("lender", script)).some((s) => s.key === "lend")).toBe(false);
    // A loan of mine already on the board: nothing to decide.
    const mine = { id: 70, by: { citizen: home.citizen.id }, created_tick: 0, kind: "credit", body: { credit: { principal: 24200, rate_per_cycle_bp: 100, term_cycles: 5, collateral: null, to: null } } };
    const posted = await layOut("lender", new Script(ctx(fixtureTransport(FIX, { "GET /s/1/citizens": roll, "GET /s/1/notice-board": { status: 200, body: { ...board(), offers: [...board().offers, mine] } } })), turnInput(home), {}, "l"));
    expect(posted.some((s) => s.key === "lend")).toBe(false);
    // Too little to lend a quarter of.
    const poor = { ...home, household: { ...home.household, balance: 12000 } };
    expect((await layOut("lender", new Script(ctx(fixtureTransport(FIX, { "GET /s/1/citizens": roll })), turnInput(poor), {}, "l"))).some((s) => s.key === "lend")).toBe(false);
  });
  it("posts the loan it chose", async () => {
    const home = unemployedHome();
    const { brain: b, ctx: c } = brain("lender", { "PUT /s/1/plan": ok, "GET /s/1/citizens": roll, "POST /s/1/offers/credit": ok, [JEV]: answer({ lend: ["lend_dear", 0.9], job: ["wait", 0.9] }) });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.error).toBeNull();
    expect(c.turn.calls.find((x) => x.tool === "post_credit_offer")?.input).toEqual({ principal: 24200, rate_per_cycle_bp: 300, term_cycles: 5, collateral: null, to: null, on_behalf_of: null });
    expect(out.intent).toContain("offered 242.00 at 3% a day over 5 days (dear)");
  });
});

// -- SJ.5: prices from the players ----------------------------------------------------------------

/** A books view with resting sides: `[last, bid, ask, bidDepth, askDepth]` per good. */
const booksWith = (rows: Record<string, [number | null, number | null, number | null, number?, number?]>) => ({
  status: 200,
  body: { books: Object.entries(rows).map(([instrument, [last_price, best_bid, best_ask, bid_depth, ask_depth]]) => ({ instrument, last_price, best_bid, best_ask, bid_depth: bid_depth ?? 0, ask_depth: ask_depth ?? 0 })), price_index: 1 },
});

describe("pricing in the firm slot (SJ.5)", () => {
  it("offers undercut one tick under the best ask and hold_price 5% over the last, with the days unsold in the sentence", async () => {
    const home = workingHome();
    const stocked = { "GET /s/1/orgs": managedOrgs({ inventory: { ore: 40 }, treasury: 5000 }), "GET /s/1/books": booksWith({ ore: [92, 88, 92, 30, 400] }) };
    // Day-old shelf memory: 30 ore yesterday, 40 today, so nothing sold.
    const memory = { firm, shelf: { ore: { cycle: home.clock.cycle - 1, qty: 30 } }, unsold: { ore: 1 } };
    const slots = await layOut("founder", new Script(ctx(fixtureTransport(FIX, stocked)), turnInput(home), memory, "f"));
    const venture = slots.find((s) => s.key === "venture")!;
    const options = venture.candidates.map((x) => x.option);
    expect(options).toEqual(expect.arrayContaining(["undercut", "hold_price"]));
    expect(options).not.toContain("sell_output");
    expect(venture.candidates.find((x) => x.option === "undercut")!.describe).toBe("ask 40 ore at 0.91, one tick under the best ask (400 ore asked at 0.92), so yours sells first: 40 ore unsold for 2 day(s)");
    expect(venture.candidates.find((x) => x.option === "hold_price")!.describe).toContain("ask 40 ore at 0.97, 5% over the last price");
    expect(venture.facts.books).toEqual({ ore: { last: "0.92", best_bid: "0.88", best_ask: "0.92", bid_depth: 30, ask_depth: 400, days_unsold: 2 } });
    expect(memory.unsold).toEqual({ ore: 2 });
    // The same day again: the shelf is read once a day, so the count holds.
    await layOut("founder", new Script(ctx(fixtureTransport(FIX, stocked)), turnInput(home), memory, "f"));
    expect(memory.unsold).toEqual({ ore: 2 });
  });
  it("places the undercut ask on the firm's behalf, and resets the days unsold when the shelf shrank", async () => {
    const home = workingHome();
    const { brain: b, ctx: c } = brain("founder", { "PUT /s/1/plan": ok, "GET /s/1/orgs": managedOrgs({ inventory: { ore: 12 }, treasury: 5000 }), "GET /s/1/books": booksWith({ ore: [92, 88, 92, 30, 400] }), "POST /s/1/orders": ok, [JEV]: answer({ venture: ["undercut", 0.8] }) });
    const m = (b as unknown as { memory: Record<string, unknown> }).memory;
    Object.assign(m, { firm, shelf: { ore: { cycle: home.clock.cycle - 1, qty: 30 } }, unsold: { ore: 3 } });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.error).toBeNull();
    expect(c.turn.calls.find((x) => x.tool === "place_order")?.input).toEqual({ instrument: "ore", side: "ask", qty: 12, limit_price: 91, on_behalf_of: 18 });
    expect(out.intent).toContain("asked 12 ore @91 (undercut)");
    expect(m.unsold).toEqual({ ore: 0 });
  });
  it("buys an input at the ask or bids one tick under it, from the treasury", async () => {
    const home = workingHome();
    const roofs = { "GET /s/1/orgs": managedOrgs({ kind: "builder", treasury: 5000 }), "GET /s/1/books": booksWith({ materials: [200, 190, 204, 10, 60] }) };
    const slots = await layOut("builder", new Script(ctx(fixtureTransport(FIX, roofs)), turnInput(home), {}, "b"));
    const venture = slots.find((s) => s.key === "venture")!;
    expect(venture.candidates.find((x) => x.option === "take_ask_materials")!.describe).toContain("at once at 2.04 each: 60 materials are asked at 2.04");
    expect(venture.candidates.find((x) => x.option === "bid_under_materials")!.describe).toContain("at 2.03 each, one tick under");
    const { brain: b, ctx: c } = brain("builder", { "PUT /s/1/plan": ok, ...roofs, "POST /s/1/orders": ok, [JEV]: answer({ venture: ["bid_under_materials", 0.8] }) });
    await b.takeTurn(c, turnInput(home));
    expect(c.turn.calls.find((x) => x.tool === "place_order")?.input).toEqual({ instrument: "materials", side: "bid", qty: 10, limit_price: 203, on_behalf_of: 18 });
    // Too poor for the ask but not for the bid under it: only the resting bid is offered.
    const thin = await layOut("builder", new Script(ctx(fixtureTransport(FIX, { ...roofs, "GET /s/1/orgs": managedOrgs({ kind: "builder", treasury: 2035 }) })), turnInput(home), {}, "b"));
    expect(thin.find((s) => s.key === "venture")!.candidates.map((x) => x.option)).toContain("bid_under_materials");
    expect(thin.find((s) => s.key === "venture")!.candidates.map((x) => x.option)).not.toContain("take_ask_materials");
  });
});

describe("a foundry (SJ.5b)", () => {
  it("buys a day's ore at its rate, or what the treasury covers, and sells the Materials it makes", async () => {
    const home = workingHome();
    const prices = booksWith({ ore: [80, null, 80, 0, 500], materials: [172, null, 172, 0, 300] });
    const rich = await layOut("founder", new Script(ctx(fixtureTransport(FIX, { "GET /s/1/orgs": managedOrgs({ kind: "foundry", treasury: 10000, inventory: { materials: 30 } }), "GET /s/1/books": prices })), turnInput(home), {}, "f"));
    const venture = rich.find((s) => s.key === "venture")!;
    // Base rate 10 an hour x 8 h = 80 ore a day, one each.
    expect(venture.candidates.find((x) => x.option === "take_ask_ore")!.describe).toContain("buy 80 ore, a day's worth at the foundry, at once at 0.80 each");
    expect(venture.candidates.find((x) => x.option === "undercut")!.describe).toContain("ask 30 materials at 1.71");
    expect(venture.facts.books).toMatchObject({ materials: { days_unsold: 0 }, ore: { ask_depth: 500 } });
    // A treasury of 20.00 covers 25 ore at the ask: that many, not one.
    const thin = await layOut("founder", new Script(ctx(fixtureTransport(FIX, { "GET /s/1/orgs": managedOrgs({ kind: "foundry", treasury: 2000 }), "GET /s/1/books": prices })), turnInput(home), {}, "f"));
    expect(thin.find((s) => s.key === "venture")!.candidates.find((x) => x.option === "take_ask_ore")!.describe).toContain("buy 25 ore, what the treasury covers of a day's worth at the foundry");
  });
});

describe("which workplace (SJ.5)", () => {
  /** Twenty Materials and the fee in hand, a job held: ready to found. */
  const ready = () => {
    const home = workingHome();
    return { ...home, household: { ...home.household, balance: 50000, pantry: { ...home.household.pantry, materials: 20 } } } as unknown as HomeView;
  };
  // At the recorded recipes and these prices: mill 120 food x 1.31 = 157.20 less 120 grain x 0.61 = 73.20; foundry 80 materials x 2.00 = 160.00 less 80 ore x 0.92 = 73.60;
  // mine 80 ore x 0.92 = 73.60; workshop 40 wares x 4.12 = 164.80 less 40 materials x 2.00 = 80.00. Less a day's wage at the job held.
  const prices = books({ food: 131, grain: 61, ore: 92, materials: 200, wares: 412, machines: 1052 });
  it("ranks the four kinds by a day's margin, best first, once a day", async () => {
    const home = ready();
    const script = new Script(ctx(fixtureTransport(FIX, { "GET /s/1/books": prices })), turnInput(home), {}, "f");
    const slots = await layOut("founder", script);
    const which = slots.find((s) => s.key === "which_workplace")!;
    expect(which.candidates.map((x) => x.option)).toEqual(["open_foundry", "open_workshop", "open_mill", "open_mine", "decide_later"]);
    const wage = jobOffers(board().offers)[0]!.hourly * 8;
    expect(which.facts.foundry).toEqual({ units_a_day: 80, revenue: "160.00", inputs: "73.60", margin: ((16000 - 7360 - wage) / 100).toFixed(2) });
    expect(which.candidates[0]!.describe).toContain("open a foundry: about 80 materials a day, worth 160.00, less 73.60 for 80 ore");
    // Not yet chosen: founding is not offered this hour.
    expect(slots.find((s) => s.key === "venture")?.candidates.map((x) => x.option) ?? []).not.toContain("found_now");
    // Once a day.
    expect((await layOut("founder", script)).some((s) => s.key === "which_workplace")).toBe(false);
    // Without the Materials, no question.
    expect((await layOut("founder", new Script(ctx(fixtureTransport(FIX, { "GET /s/1/books": prices })), turnInput(workingHome()), {}, "f"))).some((s) => s.key === "which_workplace")).toBe(false);
  });
  it("founds what it chose", async () => {
    const home = ready();
    const chosen = brain("founder", { "PUT /s/1/plan": ok, "GET /s/1/books": prices, [JEV]: answer({ which_workplace: ["open_mill", 0.8] }) });
    const first = await chosen.brain.takeTurn(chosen.ctx, turnInput(home));
    expect(first.error).toBeNull();
    expect(first.intent).toContain("which_workplace open_mill (0.80): chose a mill");
    const m = (chosen.brain as unknown as { memory: Record<string, unknown> }).memory;
    expect(m.workplaceKind).toBe("mill");
    // Next hour: found_now names the mill and opens one.
    const founding = brain("founder", { "PUT /s/1/plan": ok, "GET /s/1/books": prices, "POST /s/1/orgs": ok, [JEV]: answer({ venture: ["found_now", 0.95] }) });
    Object.assign((founding.brain as unknown as { memory: Record<string, unknown> }).memory, { workplaceKind: "mill", "asked:which_workplace": home.clock.cycle });
    const out = await founding.brain.takeTurn(founding.ctx, turnInput(home));
    expect(out.error).toBeNull();
    expect(founding.ctx.turn.calls.find((x) => x.tool === "found_org")?.input).toEqual({ kind: "firm", name: "founder's Works", first_workplace: { kind: "mill", slot: null } });
    expect(out.intent).toContain("founded founder's Works with a mill");
  });
});

describe("a job while unemployed (SJ.5)", () => {
  it("re-asks with the days without a job and the wage gone unearned in the sentence", async () => {
    const home = unemployedHome();
    const script = new Script(ctx(fixtureTransport(FIX)), turnInput(home), { unemployedSince: home.clock.cycle - 3 }, "l");
    const job = (await layOut("lender", script)).find((s) => s.key === "job")!;
    const best = jobOffers(board().offers)[0]!;
    expect(job.ask).toContain(`You have had no job for 3 day(s) and ${(best.hourly * 24 / 100).toFixed(2)}, 3 day(s) of the best wage, has gone unearned.`);
    expect(job.facts).toMatchObject({ days_without_a_job: 3, unearned_credits: (best.hourly * 24 / 100).toFixed(2) });
    expect(job.candidates.at(-1)!).toMatchObject({ option: "wait", describe: "keep waiting, another day without a wage" });
    // The first hour without a job says nothing of days, and a job held forgets the count.
    const fresh = new Script(ctx(fixtureTransport(FIX)), turnInput(home), {}, "l");
    const day0 = (await layOut("lender", fresh)).find((s) => s.key === "job")!;
    expect(day0.ask).not.toContain("no job for");
    expect(fresh.memory.unemployedSince).toBe(home.clock.cycle);
    const held = new Script(ctx(fixtureTransport(FIX)), turnInput(employedHome()), { unemployedSince: 1 }, "l");
    await layOut("lender", held);
    expect(held.memory.unemployedSince).toBeUndefined();
  });
});

describe("comfort (SJ.5)", () => {
  const low = (comfort: number, wares = 0) => {
    const home = employedHome();
    return { ...home, needs: { ...home.needs, comfort }, household: { ...home.household, pantry: { ...home.household.pantry, wares } } } as unknown as HomeView;
  };
  it("offers Wares at the ask or under it once a day while Comfort is under 60, for every persona", async () => {
    const script = new Script(ctx(fixtureTransport(FIX, { "GET /s/1/books": booksWith({ wares: [412, 400, 412, 5, 90] }) })), turnInput(low(41)), {}, "s");
    const slots = await layOut("saver", script);
    const comfort = slots.find((s) => s.key === "comfort")!;
    expect(comfort.candidates.map((x) => x.option)).toEqual(["take_ask", "bid_under", "go_without"]);
    // (80 - 41) / 6 = 6.5 -> capped at five, lifting Comfort by 30.
    expect(comfort.ask).toContain("Comfort is 41 of 100 and falls 1 an hour; 5 Wares would lift it by about 30. 90 are asked at 4.12; the best bid is 4.00.");
    expect(comfort.candidates[0]!.describe).toBe("buy 5 Wares now at 4.12 each, 20.60 in all, lifting Comfort by about 30");
    expect(comfort.candidates[1]!.describe).toContain("bid 4.11 each for 5 Wares, one tick under the ask");
    expect(comfort.facts).toMatchObject({ comfort: 41, wares_wanted: 5, comfort_lift: 30, cost_at_ask: "20.60" });
    expect((await layOut("saver", script)).some((s) => s.key === "comfort")).toBe(false);
    // Comfortable, or Wares already in the pantry: no question.
    expect((await layOut("saver", new Script(ctx(fixtureTransport(FIX)), turnInput(low(60)), {}, "s"))).some((s) => s.key === "comfort")).toBe(false);
    expect((await layOut("saver", new Script(ctx(fixtureTransport(FIX)), turnInput(low(41, 2)), {}, "s"))).some((s) => s.key === "comfort")).toBe(false);
  });
  it("bids for the Wares it chose", async () => {
    const { brain: b, ctx: c } = brain("slacker", { "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, "GET /s/1/books": booksWith({ wares: [412, 400, 412, 5, 90] }), "POST /s/1/orders": ok, [JEV]: answer({ work: ["few_low", 0.9], comfort: ["take_ask", 0.8] }) });
    const out = await b.takeTurn(c, turnInput(low(50)));
    expect(out.error).toBeNull();
    expect(c.turn.calls.find((x) => x.tool === "place_order")?.input).toEqual({ instrument: "wares", side: "bid", qty: 5, limit_price: 412 });
    expect(out.intent).toContain("comfort take_ask (0.80): bid 5 wares @412 (at the ask)");
  });
});

describe("per-slot floors (SJ.3)", () => {
  it("holds a slot under its own floor while the run's floor lets another act", async () => {
    const home = employedHome();
    const cfg = ConfigSchema.parse({ jev: { floors: { plan: 0.9 } } });
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, [JEV]: answer({ work: ["full_normal", 0.4], plan: ["put_aside", 0.8] }) }, { cfg });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.decisions).toMatchObject([
      { slot: "work", option: "full_normal", confidence: 0.4, none: false, acted: true },
      { slot: "plan", option: "put_aside", confidence: 0.8, none: false, acted: false },
    ]);
    expect(c.turn.calls.filter((x) => x.tool === "set_plan")).toHaveLength(1);
  });
});

describe("laying out the hour", () => {
  it("offers a job, not hours, to someone without a contract", async () => {
    const home = unemployedHome();
    const c = ctx(fixtureTransport(FIX));
    const slots = await layOut("founder", new Script(c, turnInput(home), {}, "f"));
    const keys = slots.map((s) => s.key);
    expect(keys).toContain("job");
    expect(keys).not.toContain("work");
    const job = slots.find((s) => s.key === "job")!;
    expect(job.candidates.map((x) => x.option)).toEqual(expect.arrayContaining(["take_best", "wait"]));
    expect(job.candidates.at(-1)!.act).toBeNull();
  });
  it("offers hours, and no switch, when nothing pays more than the job held; and the plan, when the balance is fat", async () => {
    const home = employedHome();
    const c = ctx(fixtureTransport(FIX));
    const slots = await layOut("wage-maximiser", new Script(c, turnInput(home), {}, "wm"));
    expect(slots.map((s) => s.key)).toEqual(["work", "plan"]);
    const plan = slots.find((s) => s.key === "plan")!;
    expect(plan.candidates.map((x) => x.option)).toEqual(["put_aside", "keep_plan"]);
  });
  it("asks about the hours again only for a reason, once they are set", async () => {
    const home = employedHome();
    const best = jobOffers(board().offers)[0]!;
    // Hours set, a modest balance (neither under a day's wage nor over five), Food high: nothing to change.
    const set = {
      ...home,
      household: { ...home.household, balance: 20000 },
      labor: { ...home.labor, allocations: [{ workplace: best.workplace, hours: best.maxHours, effort: "normal", org_name: "x", kind: "farm" }] },
    } as HomeView;
    const quiet = await layOut("saver", new Script(ctx(fixtureTransport(FIX)), turnInput(set), {}, "sv"));
    expect(quiet.map((s) => s.key)).not.toContain("work");
    const rich = { ...set, household: { ...set.household, balance: home.household.balance } } as HomeView;
    const easy = await layOut("saver", new Script(ctx(fixtureTransport(FIX)), turnInput(rich), {}, "sv"));
    expect(easy.find((s) => s.key === "work")!.candidates.map((x) => x.option)).toEqual(["ease_off", "keep_hours"]);
    const tired = { ...set, labor: { ...set.labor, fatigue_debt: 3 } } as HomeView;
    const slots = await layOut("saver", new Script(ctx(fixtureTransport(FIX)), turnInput(tired), {}, "sv"));
    const work = slots.find((s) => s.key === "work")!;
    expect(work.candidates.map((x) => x.option)).toEqual(["rest_more", "keep_hours"]);
    expect(work.ask).toContain("fatigue debt");
  });
  it("offers a roof to the unhoused when a dwelling is to let", async () => {
    const home = unemployedHome();
    const lease = { id: 77, by: { org: 1 }, created_tick: 0, kind: "lease", body: { lease: { asset: { dwelling: 12 }, rent_per_cycle: 800, term_cycles: null } } };
    const withLease = { status: 200, body: { ...board(), offers: [...board().offers, lease] } };
    const slots = await layOut("slacker", new Script(ctx(fixtureTransport(FIX, { "GET /s/1/notice-board": withLease })), turnInput(home), {}, "sl"));
    const housing = slots.find((s) => s.key === "housing")!;
    expect(housing.candidates.map((x) => x.option)).toEqual(["rent_cheapest", "stay_unhoused"]);
    expect(housing.ask).toContain("8.00 a day");
    expect(housing.facts).toMatchObject({ housed: false, dwellings_to_let: 1, rent_pct_of_day_wage: 13 });
  });
  it("offers the borrower the cheapest loan once", async () => {
    const home = unemployedHome();
    const credit = (id: number, bp: number) => ({ id, by: { citizen: 3 }, created_tick: 0, kind: "credit", body: { credit: { principal: 50000, rate_per_cycle_bp: bp, term_cycles: 5, collateral: null, to: null } } });
    const withCredit = { status: 200, body: { ...board(), offers: [...board().offers, credit(90, 200), credit(91, 50)] } };
    const script = new Script(ctx(fixtureTransport(FIX, { "GET /s/1/notice-board": withCredit })), turnInput(home), {}, "b");
    const slots = await layOut("borrower", script);
    const c = slots.find((s) => s.key === "credit")!;
    expect(c.candidates[0]!).toMatchObject({ option: "take_credit", irreversible: true });
    expect(c.ask).toContain("0.50% a day");
    script.memory.borrowed = true;
    expect((await layOut("borrower", script)).map((s) => s.key)).not.toContain("credit");
  });
  it("offers the switch when an open offer pays more", async () => {
    const best = jobOffers(board().offers)[0]!;
    const home = employedHome(Math.floor(best.hourly / 2));
    const c = ctx(fixtureTransport(FIX));
    const slots = await layOut("wage-maximiser", new Script(c, turnInput(home), {}, "wm"));
    const job = slots.find((s) => s.key === "job")!;
    expect(job.candidates.map((x) => x.option)).toEqual(["switch", "stay"]);
    expect(job.candidates[0]!.irreversible).toBe(true);
    expect(job.ask).toMatch(/\d+% more/);
  });
});

describe("the state and the questions", () => {
  it("is the same bytes for the same view, and carries no ids and no persona", async () => {
    const home = unemployedHome();
    const p = persona("founder");
    const build = async () => {
      const c = ctx(fixtureTransport(FIX));
      const slots = await layOut("founder", new Script(c, turnInput(home), {}, "f"));
      return { state: JSON.stringify(buildState(turnInput(home), slots, {})), questions: buildQuestions(p, slots) };
    };
    const a = await build();
    const b = await build();
    expect(a.state).toBe(b.state);
    expect(a.state).not.toContain('"id"');
    expect(a.state).not.toContain(p.name);
    expect(a.state).not.toContain(p.goals[0]!);
    const q = a.questions.job!;
    expect(q.type).toBe("choice");
    expect(q.instructions).toContain(p.name);
    expect(q.instructions).toContain("Decide only this");
    expect(Object.keys(q.criteria)).toContain("wait");
  });
  it("places the citizen on the scoreboard, with the gaps precomputed", async () => {
    const home = unemployedHome();
    const me = home.citizen.id;
    const rows = [
      { citizen: 900, handle: "a", net_worth: 200000, self_made: 0, firms: [] },
      { citizen: me, handle: "me", net_worth: 96856, self_made: 0, firms: [] },
      { citizen: 901, handle: "b", net_worth: 90000, self_made: 0, firms: [] },
    ];
    const st = standingOf({ clock: home.clock, rows } as never, me, 90000);
    expect(st).toMatchObject({ rank: 2, of: 3, trend_since_yesterday: "rising", gap_to_above_credits: "1031.44", gap_to_below_credits: "68.56" });
    expect(standingText(st)).toBe("You stand 2 of 3 by net worth (968.56 credits), rising since yesterday; 1031.44 behind the one above; 68.56 ahead of the one below.");
    expect(standingOf(null, me, null)).toBeNull();
    const q = buildQuestions(persona("founder"), [], standingText(st));
    expect(q).toEqual({});
    const state = buildState(turnInput(home), [], {}, st);
    expect(state.standing?.rank).toBe(2);
  });
  it("puts the ambition with the persona, not in the state", async () => {
    const home = unemployedHome();
    const p = persona("founder");
    expect(p.ambition).toMatch(/scoreboard/);
    const c = ctx(fixtureTransport(FIX));
    const slots = await layOut("founder", new Script(c, turnInput(home), {}, "f"));
    const q = buildQuestions(p, slots, "You stand 2 of 3.");
    expect(q.job!.instructions).toContain("Ambition:");
    expect(q.job!.instructions).toContain("You stand 2 of 3.");
    expect(JSON.stringify(buildState(turnInput(home), slots, {}))).not.toContain("Ambition");
    expect(persona("slacker").ambition).toBeNull();
  });
  it("reads the food trend from the last turn", async () => {
    const home = unemployedHome();
    const c = ctx(fixtureTransport(FIX));
    const slots = await layOut("founder", new Script(c, turnInput(home), {}, "f"));
    expect(buildState(turnInput(home), slots, {}).me.food_trend).toBe("unknown");
    expect(buildState(turnInput(home), slots, { lastFood: home.needs.food + 10 }).me.food_trend).toBe("falling");
  });
});

describe("a Jev turn", () => {
  it("takes the job it was told to take", async () => {
    const home = unemployedHome();
    const best = jobOffers(board().offers)[0]!;
    const { brain: b, ctx: c } = brain("founder", { "PUT /s/1/plan": ok, [`POST /s/1/offers/${best.id}/accept`]: ok, [JEV]: answer({ job: ["take_best", 0.8] }) });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.ended_by).toBe("script");
    expect(out.error).toBeNull();
    expect(c.turn.calls.find((x) => x.tool === "accept_offer")?.input).toEqual({ offer: best.id });
    expect(out.decisions).toMatchObject([{ slot: "job", option: "take_best", confidence: 0.8, none: false, acted: true }]);
    expect(out.intent).toContain("job take_best (0.80)");
    expect(out.usage.input).toBe(300);
    expect(out.usage.usd).toBeCloseTo((300 * 0.042) / 1_000_000, 12);
  });
  it("keeps what each slot offered beside the choice, and the whole request only when asked (E-6)", async () => {
    const home = unemployedHome();
    const best = jobOffers(board().offers)[0]!;
    const overrides = { "PUT /s/1/plan": ok, [`POST /s/1/offers/${best.id}/accept`]: ok, [JEV]: answer({ job: ["take_best", 0.8] }) };
    const { brain: b, ctx: c } = brain("founder", overrides);
    const out = await b.takeTurn(c, turnInput(home));
    const job = out.decisions!.find((d) => d.slot === "job")!;
    // Every option the slot listed, the do-nothing one last, so "was it ever offered" is a journal read.
    expect(job.offered[0]).toBe("take_best");
    expect(job.offered[job.offered.length - 1]).toBe("wait");
    expect(out.request).toBeUndefined();

    const cfg = ConfigSchema.parse({ jev: { journal_questions: true } });
    const { brain: b2, ctx: c2 } = brain("founder", overrides, { cfg });
    const kept = await b2.takeTurn(c2, turnInput(home));
    expect(Object.keys(kept.request!.questions.job!.criteria)).toEqual(job.offered);
    expect(kept.request!.questions.job!.instructions).toContain("Decide only this:");
    expect((kept.request!.state as { slots: Record<string, unknown> }).slots.job).toBeDefined();
  });
  it("sets the hours the scripted helper would", async () => {
    const home = employedHome();
    const best = jobOffers(board().offers)[0]!;
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, [JEV]: answer({ work: ["full_normal", 0.9], plan: ["put_aside", 0.8] }) });
    await b.takeTurn(c, turnInput(home));
    const labor = c.turn.calls.find((x) => x.tool === "set_labor");
    expect(labor?.input).toEqual({ allocations: [{ workplace: best.workplace, hours: Math.min(best.maxHours, home.labor.budget), effort: "normal" }] });
    // The plan slot runs last and keeps the rest of the plan as it was.
    const plans = c.turn.calls.filter((x) => x.tool === "set_plan");
    expect(plans).toHaveLength(2);
    expect(plans[1]!.input).toMatchObject({ keep_balance_at_least: Math.floor(home.household.balance * 0.6), keep_food_at_least: 24, labor: "explicit" });
    expect(c.turn.calls.map((x) => x.tool).lastIndexOf("set_labor")).toBeLessThan(c.turn.calls.map((x) => x.tool).lastIndexOf("set_plan"));
  });
  it("holds when every slot chose to do nothing", async () => {
    const home = employedHome();
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, [JEV]: answer({ work: ["none", 0.7], plan: ["keep_plan", 0.9] }) });
    const out = await b.takeTurn(c, turnInput(home));
    expect(c.turn.calls.filter((x) => x.tool !== "set_plan" && x.tool !== "notice_board" && x.tool !== "scoreboard").map((x) => x.tool)).toEqual([]);
    expect(out.decisions).toMatchObject([
      { slot: "work", option: "none", confidence: 0.7, none: true, acted: false },
      { slot: "plan", option: "keep_plan", confidence: 0.9, none: true, acted: false },
    ]);
  });
  it("drops an irreversible choice made without confidence", async () => {
    const best = jobOffers(board().offers)[0]!;
    const home = employedHome(Math.floor(best.hourly / 2));
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, [JEV]: answer({ work: ["none", 0.9], job: ["switch", 0.3], plan: ["keep_plan", 0.9] }) });
    const out = await b.takeTurn(c, turnInput(home));
    expect(c.turn.calls.some((x) => x.tool === "accept_offer")).toBe(false);
    expect(out.decisions?.find((d) => d.slot === "job")).toMatchObject({ option: "switch", acted: false, none: false });
    expect(out.intent).toContain("job switch (0.30)");
  });
  it("runs the hours before the persona's own business", async () => {
    const best = jobOffers(board().offers)[0]!;
    const home = employedHome(Math.floor(best.hourly / 2));
    const { brain: b, ctx: c } = brain("wage-maximiser", {
      "PUT /s/1/plan": ok,
      "PUT /s/1/labor": ok,
      [`POST /s/1/offers/${best.id}/accept`]: ok,
      "POST /s/1/contracts/1/terminate": ok,
      [JEV]: answer({ job: ["switch", 0.95], work: ["few_low", 0.6], plan: ["keep_plan", 0.9] }),
    });
    const out = await b.takeTurn(c, turnInput(home));
    const acts = c.turn.calls.filter((x) => x.tool !== "set_plan" && x.tool !== "notice_board" && x.tool !== "scoreboard").map((x) => x.tool);
    expect(acts).toEqual(["set_labor", "accept_offer", "terminate_contract"]);
    expect(out.decisions?.map((d) => d.slot)).toEqual(["work", "job", "plan"]);
  });
  it("reports an option it never offered, and a slot it never asked", async () => {
    const home = employedHome();
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, [JEV]: answer({ work: ["nap", 0.9], plan: ["keep_plan", 0.9] }, { ballot_0: { type: "choice", choice: "yes", confidence: 1, probabilities: { yes: 1 } } }) });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.did_not_understand).toHaveLength(2);
    expect(out.did_not_understand[0]).toMatch(/"nap", which was not offered/);
    expect(out.did_not_understand[1]).toMatch(/never asked: ballot_0/);
    expect(out.decisions).toMatchObject([{ slot: "plan", option: "keep_plan", confidence: 0.9, none: true, acted: false }]);
  });
  it("says nothing was decided when no slot had a choice in it", async () => {
    const home = unemployedHome();
    const empty = { status: 200, body: { offers: [] } };
    // A modest balance, so the plan has no reason to move either.
    const poor = { ...home, household: { ...home.household, balance: 5000 } };
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, "GET /s/1/notice-board": empty });
    const out = await b.takeTurn(c, turnInput(poor));
    expect(out.intent).toMatch(/nothing to decide/);
    expect(c.turn.calls.some((x) => x.tool === "accept_offer")).toBe(false);
  });
});

describe("when the decisions API fails", () => {
  it("ends the turn in error and acts on nothing", async () => {
    const home = employedHome();
    const { brain: b, ctx: c } = brain("wage-maximiser", { "PUT /s/1/plan": ok, [JEV]: { status: 500, body: { error: "boom" } } });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.ended_by).toBe("error");
    expect(out.error).toMatch(/500 from the decisions API/);
    expect(out.decisions).toEqual([]);
    expect(c.turn.calls.some((x) => x.tool === "set_labor")).toBe(false);
  });
  it("retries a gateway error once and takes the second answer", async () => {
    const home = employedHome();
    const best = jobOffers(board().offers)[0]!;
    let n = 0;
    const inner = fixtureTransport(FIX, { "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, [JEV]: answer({ work: ["full_normal", 0.9], plan: ["keep_plan", 0.9] }) });
    const flaky: Transport = async (input, init) => {
      if (String(input).includes("/decisions") && n++ === 0) return new Response("<html>520</html>", { status: 520 });
      return inner(input, init);
    };
    const { brain: b, ctx: c } = brain("wage-maximiser", {}, { transport: flaky });
    const out = await b.takeTurn(c, turnInput(home));
    expect(out.ended_by).toBe("script");
    expect(n).toBe(2);
    expect(c.turn.calls.find((x) => x.tool === "set_labor")?.input).toMatchObject({ allocations: [{ workplace: best.workplace }] });
  });
  it("stops asking after the key is refused", async () => {
    const home = employedHome();
    let jevCalls = 0;
    const inner = fixtureTransport(FIX, { "PUT /s/1/plan": ok, [JEV]: { status: 401, body: { error: "bad key" } } });
    const counting: Transport = async (input, init) => {
      if (String(input).includes("/decisions")) jevCalls += 1;
      return inner(input, init);
    };
    const { brain: b, ctx: c } = brain("wage-maximiser", {}, { transport: counting });
    const first = await b.takeTurn(c, turnInput(home));
    expect(first.ended_by).toBe("error");
    expect(b.dead).toMatch(/401/);
    const second = await b.takeTurn(ctx(counting), turnInput(home));
    expect(second.ended_by).toBe("error");
    expect(jevCalls).toBe(1);
  });
});

describe("the report's Jev section", () => {
  const turn = (player: string, tick: number, decisions: TurnRecord["decisions"], rejections = 0): TurnRecord => ({
    kind: "turn",
    run: "r",
    player,
    persona: "founder",
    brain: "jev",
    model: "~typesafe/jev-latest",
    epoch: 1,
    cycle: 1,
    tick,
    ms: 400,
    situation: { balance: 0, food: 50, shelter: 50, comfort: 50, housed: true, jobs: 1, pantry_food: 0 },
    intent: "",
    calls: [],
    rejections: Array.from({ length: rejections }, () => ({ tool: "x", code: "c", detail: "d" })),
    did_not_understand: [],
    ended_by: "script",
    error: null,
    usage: { input: 300, output: 0, cache_read: 0, cache_write: 0, usd: 0.0000126 },
    decisions,
  });
  it("counts held turns, dropped choices and flip-flops", () => {
    const d = (slot: string, option: string, acted: boolean, none = false, confidence = 0.8) => ({ slot, option, confidence, none, acted });
    const lines = jevSection([
      turn("f-1", 1, [d("job", "switch", false), d("work", "full_normal", true)]),
      turn("f-1", 2, [d("job", "stay", false, true), d("work", "none", false, true)]),
      turn("f-1", 3, [d("job", "switch", true)], 1),
    ]);
    const row = lines.find((l) => l.startsWith("| f-1 |"))!;
    expect(row).toContain("| 3 | 1 (33%) |");
    expect(row).toContain("| 1 of 5 |");
    expect(row).toContain("| 1 | 1 | 0 | 400 |");
    // Journals from before E-6 carry no `offered`.
    expect(lines.some((l) => l.startsWith("| job | switch 2, stay 1 | — |"))).toBe(true);
  });
  it("lists what each slot offered, so an option never chosen is still counted (E-6)", () => {
    const d = (slot: string, offered: string[], option: string) => ({ slot, offered, option, confidence: 0.8, none: false, acted: true });
    const lines = jevSection([
      turn("f-1", 1, [d("firm", ["hire", "lay_off", "keep_staff"], "hire")]),
      turn("f-1", 2, [d("firm", ["hire", "keep_staff"], "hire")]),
    ]);
    expect(lines.some((l) => l.startsWith("| firm | hire 2 | hire 2, keep_staff 2, lay_off 1 |"))).toBe(true);
  });
  it("is absent when no Jev player took a turn", () => {
    expect(jevSection([])).toEqual([]);
  });
});
