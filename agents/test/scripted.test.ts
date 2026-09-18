// The scripted brains over recorded views: what they read, and what they decide.

import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeClient, type HomeView } from "../src/api/client.ts";
import { fixtureTransport } from "../src/api/transport.ts";
import { ScriptedBrain } from "../src/brain/scripted/index.ts";
import { judge, PROBES } from "../src/brain/scripted/fuzz.ts";
import { Script, currentJobs, jobOffers, median } from "../src/brain/scripted/script.ts";
import { wageMaximiser } from "../src/brain/scripted/strategies.ts";
import { loadPersona } from "../src/player/persona.ts";
import { newTurn, type ToolContext } from "../src/tools/context.ts";
import { fixtureName } from "../src/api/transport.ts";
import { readFileSync } from "node:fs";

const FIX = join(import.meta.dirname, "fixtures", "api");
const BASE = "http://127.0.0.1:18090";
const fixture = <T>(key: string): T => (JSON.parse(readFileSync(join(FIX, fixtureName(key)), "utf8")) as { body: T }).body;

function ctx(overrides: Parameters<typeof fixtureTransport>[1] = {}): ToolContext {
  const client = makeClient({ baseUrl: BASE, auth: { key: "k" }, transport: fixtureTransport(FIX, overrides) });
  return { client, sid: 1, turn: newTurn(4), now: () => 0 };
}

const ok = { status: 200, body: { clock: {}, events: [] } };
const board = () => fixture<{ offers: Parameters<typeof jobOffers>[0] }>("GET /s/1/notice-board");

/** The recorded home view was captured before the player had a job; this gives it the board's best one. */
function employedHome(): HomeView {
  const home = fixture<HomeView>("GET /s/1/home");
  const best = jobOffers(board().offers)[0]!;
  const contract = {
    id: 1,
    body: { employment: { org: best.org, workplace: best.workplace, pay: { hourly: best.hourly }, max_hours: best.maxHours, notice_cycles: 1, places: best.places, term_cycles: null } },
    created_tick: 0,
    parties: {},
    role: "party",
    status: "active",
    term_cycles: null,
  } as unknown as HomeView["labor"]["employment"][number];
  return { ...home, labor: { ...home.labor, employment: [contract], allocations: [] } };
}

describe("reading offers", () => {
  it("ranks employment offers by hourly pay", () => {
    const board = fixture<{ offers: Parameters<typeof jobOffers>[0] }>("GET /s/1/notice-board");
    const jobs = jobOffers(board.offers);
    expect(jobs.length).toBeGreaterThan(0);
    for (let i = 1; i < jobs.length; i++) expect(jobs[i - 1]!.hourly).toBeGreaterThanOrEqual(jobs[i]!.hourly);
    expect(jobs[0]).toMatchObject({ hourly: expect.any(Number), workplace: expect.any(Number), org: expect.any(Number) });
  });
  it("reads the player's own contracts", () => {
    const home = fixture<HomeView>("GET /s/1/home");
    const jobs = currentJobs(home);
    expect(jobs.length).toBe(home.labor.employment.length);
  });
  it("takes a median", () => {
    expect(median([])).toBe(0);
    expect(median([3, 1, 2])).toBe(2);
    expect(median([1, 2, 3, 4])).toBe(3);
  });
});

describe("the wage-maximiser", () => {
  it("takes the best offer when unemployed", async () => {
    const home = fixture<HomeView>("GET /s/1/home");
    const unemployed = { ...home, labor: { ...home.labor, employment: [], allocations: [] } };
    const best = jobOffers(board().offers)[0]!;
    const c = ctx({ "PUT /s/1/plan": ok, [`POST /s/1/offers/${best.id}/accept`]: ok });
    const s = new Script(c, { clock: unemployed.clock, home: unemployed, notes: "", lastTurn: null }, {}, "wm");
    const intent = await wageMaximiser(s);
    expect(intent).toContain(`took the best offer ${best.id}`);
    const accept = c.turn.calls.find((x) => x.tool === "accept_offer");
    expect(accept?.input).toEqual({ offer: best.id });
  });
  it("keeps the job when nothing pays clearly more, and works it", async () => {
    const home = employedHome();
    const c = ctx({ "PUT /s/1/plan": ok, "PUT /s/1/labor": ok });
    const s = new Script(c, { clock: home.clock, home, notes: "", lastTurn: null }, {}, "wm");
    const intent = await wageMaximiser(s);
    expect(intent).toMatch(/^no better offer than \d+\/h; worked full hours$/);
    expect(c.turn.calls.some((x) => x.tool === "accept_offer")).toBe(false);
    const labor = c.turn.calls.find((x) => x.tool === "set_labor");
    expect(labor?.input).toEqual({ allocations: [{ workplace: jobOffers(board().offers)[0]!.workplace, hours: expect.any(Number), effort: "normal" }] });
  });
});

describe("the rule-prober", () => {
  it("judges an accepted probe and a server error as defects, raw ids as copy", () => {
    const p = PROBES[0]!;
    expect(judge(p, { ok: true, data: {} })).toMatch(/^DEFECT: accepted/);
    expect(judge(p, { ok: false, status: 500, code: "HTTP_500", title: "x", detail: "boom" })).toMatch(/^DEFECT: server error 500/);
    expect(judge(p, { ok: false, status: 422, code: "x", title: "x", detail: "c47 already works at w2" })).toMatch(/^COPY: raw ids/);
    expect(judge(p, { ok: false, status: 422, code: "x", title: "x", detail: "the contract allows at most 8 hours" })).toBeNull();
  });
  it("probes round robin and journals the verdict as a note", async () => {
    const persona = loadPersona(join(import.meta.dirname, "..", "personas", "rule-prober.md"));
    const brain = new ScriptedBrain(persona, "rule-prober-1");
    const home = employedHome();
    const c = ctx({ "PUT /s/1/plan": ok, "PUT /s/1/labor": ok });
    const first = await brain.takeTurn(c, { clock: home.clock, home, notes: "", lastTurn: null });
    expect(first.ended_by).toBe("script");
    expect(first.intent).toContain(`probed "${PROBES[0]!.name}"`);
    // The labor fixture is a 200: a probe past the budget was accepted, which is a defect the note carries.
    expect(first.did_not_understand[0]).toMatch(/^DEFECT: accepted "hours past the budget"/);
    const c2 = ctx({ "PUT /s/1/labor": ok });
    const second = await brain.takeTurn(c2, { clock: home.clock, home, notes: "", lastTurn: null });
    expect(second.intent).toContain(`probed "${PROBES[1]!.name}"`);
  });
});

describe("contracts in their notice period", () => {
  it("are not worked: only active contracts count as jobs", () => {
    const home = employedHome();
    const ending = { ...home.labor.employment[0]!, id: 2, status: "notice" };
    const view = { ...home, labor: { ...home.labor, employment: [home.labor.employment[0]!, ending] } };
    expect(currentJobs(view).map((j) => j.contract)).toEqual([1]);
  });
});
