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
import { ConfigSchema } from "../src/config.ts";
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

function brain(slug: string, overrides: Overrides, opts: { budget?: Budget; transport?: Transport } = {}) {
  const transport = opts.transport ?? fixtureTransport(FIX, overrides);
  const cfg = ConfigSchema.parse({});
  const b = new JevBrain({ persona: persona(slug), player: `${slug}-1`, handle: slug, cfg, budget: opts.budget ?? new Budget(1_000_000, 1), key: "or-key", transport });
  return { brain: b, ctx: ctx(transport) };
}

const turnInput = (home: HomeView) => ({ clock: home.clock, home, notes: "", lastTurn: null });

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
    expect(out.decisions).toEqual([{ slot: "job", option: "take_best", confidence: 0.8, none: false, acted: true }]);
    expect(out.intent).toContain("job take_best (0.80)");
    expect(out.usage.input).toBe(300);
    expect(out.usage.usd).toBeCloseTo((300 * 0.042) / 1_000_000, 12);
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
    expect(out.decisions).toEqual([
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
    expect(out.decisions).toEqual([{ slot: "plan", option: "keep_plan", confidence: 0.9, none: true, acted: false }]);
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
    expect(lines.some((l) => l.startsWith("| job | switch 2, stay 1"))).toBe(true);
  });
  it("is absent when no Jev player took a turn", () => {
    expect(jevSection([])).toEqual([]);
  });
});
