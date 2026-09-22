// S2.10: the Commune's tools, personas and probes over responses recorded from
// a lab Commune (test/fixtures/commune/, `GET /s/1/...` names; refusals/ holds
// the alternates a probe meets). No server, no key.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeClient, type HomeView } from "../src/api/client.ts";
import { fixtureName, fixtureTransport } from "../src/api/transport.ts";
import { PROBES, judge, probesFor } from "../src/brain/scripted/fuzz.ts";
import { ScriptedBrain } from "../src/brain/scripted/index.ts";
import { Script, electionFor, normOf, type OfficesView, type ProposalsView } from "../src/brain/scripted/script.ts";
import { chronicler, freeRider, rationer, steward } from "../src/brain/scripted/strategies.ts";
import { rulesFor } from "../src/brain/llm/prompts.ts";
import { loadPersona } from "../src/player/persona.ts";
import { extract, situationText } from "../src/player/situation.ts";
import { governanceParagraph } from "../src/report/report.ts";
import type { TurnRecord } from "../src/journal/journal.ts";
import { FREEPORT, hasAssembly, type SocietyFacts } from "../src/society.ts";
import * as act from "../src/tools/act.ts";
import { invoke, newTurn, type ToolContext } from "../src/tools/context.ts";
import * as read from "../src/tools/read.ts";

const FIX = join(import.meta.dirname, "fixtures", "commune");
const BASE = "http://127.0.0.1:18090";
const fixture = <T>(key: string): T => (JSON.parse(readFileSync(join(FIX, fixtureName(key)), "utf8")) as { body: T }).body;
const refusal = (name: string): { status: number; body: unknown } => JSON.parse(readFileSync(join(FIX, "refusals", `${name}.json`), "utf8"));

/** The recorded lab Commune's capabilities, as `readFacts` would fold them. */
const caps = fixture<{ money: boolean; governance: string; labor: string; common_store: boolean; offices: { kind: string }[]; proposal_kinds: string[]; proposers: string }>("GET /societies/1/capabilities");
const COMMUNE: SocietyFacts = {
  preset: "commune",
  display: "The Commune",
  money: caps.money,
  governance: caps.governance,
  labor: caps.labor,
  common_store: caps.common_store,
  offices: caps.offices.map((o) => o.kind),
  proposal_kinds: caps.proposal_kinds,
  proposers: caps.proposers,
};

function ctx(overrides: Parameters<typeof fixtureTransport>[1] = {}, maxActions = 4): ToolContext {
  const client = makeClient({ baseUrl: BASE, auth: { key: "k" }, transport: fixtureTransport(FIX, overrides) });
  return { client, sid: 1, turn: newTurn(maxActions), now: () => 0 };
}
const ok = { status: 200, body: { clock: {}, events: [] } };
const home = () => fixture<HomeView>("GET /s/1/home");
const script = (c: ToolContext, h = home(), memory: Record<string, unknown> = {}) => new Script(c, { clock: h.clock, home: h, notes: "", lastTurn: null }, memory, "x", COMMUNE);
const persona = (slug: string) => loadPersona(join(import.meta.dirname, "..", "personas", `${slug}.md`));

describe("the society's facts", () => {
  it("fold the capabilities into what the brains read", () => {
    expect(COMMUNE).toMatchObject({ money: false, labor: "norm", governance: "direct", common_store: true, offices: ["coordinator"], proposers: "anyone" });
    expect(hasAssembly(COMMUNE)).toBe(true);
    expect(hasAssembly(FREEPORT)).toBe(false);
  });
  it("give the LLM brain the society's own rules, and Freeport's unchanged first line", () => {
    const commune = rulesFor(COMMUNE);
    expect(commune).toContain("no money and no wages");
    expect(commune).toContain("Common Store");
    expect(commune).toContain("take a position");
    expect(rulesFor(FREEPORT)).toContain("private firms, order books");
    expect(rulesFor(FREEPORT)).not.toContain("assembly is where");
  });
});

describe("the Commune read tools", () => {
  it("return the assembly, the offices, the Store, the Ledger and the Plan as the server sent them", async () => {
    const c = ctx();
    const p = await invoke(read.proposals, {}, c);
    expect(p.ok && (p.data as ProposalsView).open.map((x) => x.title)).toEqual(["Seven hours", "Honor founder-1"]);
    const one = await invoke(read.proposal, { proposal: 1 }, c);
    expect(one.ok && (one.data as { kind_tag: string }).kind_tag).toBe("honor");
    const o = await invoke(read.offices, {}, c);
    expect(o.ok && (o.data as OfficesView).offices[0]?.election?.candidates[0]?.handle).toBe("founder-1");
    const s = await invoke(read.store, {}, c);
    expect(s.ok && (s.data as { rule: string; stock: { good: string }[] }).rule).toBe("need_first");
    expect(s.ok && (s.data as { stock: { good: string }[] }).stock[0]?.good).toBe("food");
    const l = await invoke(read.ledger, {}, c);
    expect(l.ok && (l.data as { norm_hours: number; least_staffed: number; rows: unknown[] })).toMatchObject({ norm_hours: 6, least_staffed: 1 });
    const plan = await invoke(read.publishedPlan, {}, c);
    expect(plan.ok && (plan.data as { advisory: boolean; founding_materials: number }).advisory).toBe(true);
    expect(c.turn.actionsUsed).toBe(0);
  });
});

describe("the Commune act tools", () => {
  it("send the wire shapes the web client sends", async () => {
    let sent: { method: string; path: string; body: unknown } | null = null;
    const transport = async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
      const req = new Request(input, init);
      const text = await req.text();
      sent = { method: req.method, path: decodeURIComponent(new URL(req.url).pathname), body: text ? JSON.parse(text) : null };
      return new Response(JSON.stringify({ clock: {}, events: [], first_seq: null }), { status: 200, headers: { "content-type": "application/json" } });
    };
    const c: ToolContext = { client: makeClient({ baseUrl: BASE, auth: { key: "k" }, transport }), sid: 1, turn: newTurn(20) };
    await invoke(act.propose, { title: "Seven hours", text: "Short.", kind: { policy_change: { patch: { work_norm_hours: 7 } } } }, c);
    expect(sent).toEqual({ method: "POST", path: "/s/1/proposals", body: { title: "Seven hours", text: "Short.", kind: { policy_change: { patch: { work_norm_hours: 7 } } } } });
    await invoke(act.propose, { title: "A word", text: "The text.", kind: "resolution" }, c);
    expect(sent).toMatchObject({ body: { kind: "resolution", text: "The text." } });
    await invoke(act.vote, { proposal: 3, ballot: "no" }, c);
    expect(sent).toEqual({ method: "PUT", path: "/s/1/proposals/3/ballot", body: { ballot: "no" } });
    await invoke(act.approve, { office: "coordinator", candidates: [40, 41] }, c);
    expect(sent).toEqual({ method: "PUT", path: "/s/1/offices/coordinator/ballot", body: { candidates: [40, 41] } });
    await invoke(act.stand, { office: "coordinator" }, c);
    expect(sent).toMatchObject({ method: "POST", path: "/s/1/offices/coordinator/candidacy" });
    await invoke(act.withdrawCandidacy, { office: "coordinator" }, c);
    expect(sent).toMatchObject({ method: "DELETE", path: "/s/1/offices/coordinator/candidacy" });
    await invoke(act.publishPlan, { targets: { "1": 12, "2": 8 } }, c);
    expect(sent).toEqual({ method: "PUT", path: "/s/1/offices/coordinator/plan", body: { targets: { "1": 12, "2": 8 } } });
    await invoke(act.openWorkplace, { kind: "workshop" }, c);
    expect(sent).toEqual({ method: "POST", path: "/s/1/workplaces", body: { kind: "workshop", slot: null } });
    await invoke(act.closeWorkplace, { workplace: 5 }, c);
    expect(sent).toMatchObject({ method: "DELETE", path: "/s/1/workplaces/5" });
    await invoke(act.postFloor, { proposal: 0, body: "For the record." }, c);
    expect(sent).toEqual({ method: "POST", path: "/s/1/channels/assembly:0/messages", body: { body: "For the record." } });
    await invoke(act.takePosition, { workplace: 1 }, c);
    expect(sent).toMatchObject({ method: "POST", path: "/s/1/workplaces/1/position" });
    await invoke(act.leavePosition, { workplace: 1 }, c);
    expect(sent).toMatchObject({ method: "DELETE", path: "/s/1/workplaces/1/position" });
    await invoke(act.setPlan, { keep_food_at_least: 24, keep_balance_at_least: 0, vote_default: { follow: 41 } }, c);
    expect(sent).toMatchObject({ body: { plan: { vote_default: { follow: 41 }, keep_food_at_least: 24 } } });
    expect(c.turn.actionsUsed).toBe(13);
  });
  it("hand the recorded refusals back verbatim", async () => {
    const c = ctx();
    const plan = await invoke(act.publishPlan, { targets: {} }, c);
    expect(plan).toMatchObject({ ok: false, status: 422, code: "not_an_office_holder", detail: "You do not sit as Coordinator" });
    const twice = await invoke(act.stand, { office: "coordinator" }, ctx({ "POST /s/1/offices/coordinator/candidacy": refusal("POST_stand_twice") }));
    expect(twice).toMatchObject({ ok: false, code: "already_exists" });
    const field = await invoke(act.propose, { title: "Tax", kind: { policy_change: { patch: { tax_rate: 0.3 } } } }, ctx({ "POST /s/1/proposals": refusal("POST_directorate_field") }));
    expect(field.ok).toBe(false);
    if (!field.ok) expect(field.status).toBe(422);
  });
});

describe("the situation under a norm", () => {
  it("counts a contract-less position as a job and names it for the model", () => {
    const h = home();
    expect(h.labor.employment).toEqual([]);
    expect(extract(h).jobs).toBe(1);
    expect(situationText(h)).toContain("Positions under the norm: workplace 1 (Legacy Farm No. 2, farm)");
  });
});

describe("the Commune personas", () => {
  it("the steward works the norm, stands for coordinator, moves a resolution and votes yes", async () => {
    const h = home();
    const unstood = { ...fixture<OfficesView>("GET /s/1/offices") };
    unstood.offices = unstood.offices.map((o) => ({ ...o, election: o.election ? { ...o.election, i_stand: false } : null }));
    const c = ctx({ "PUT /s/1/plan": ok, "GET /s/1/offices": { status: 200, body: unstood }, "POST /s/1/offices/coordinator/candidacy": ok, "POST /s/1/proposals": ok, "PUT /s/1/proposals/0/ballot": ok, "PUT /s/1/proposals/1/ballot": ok }, 6);
    const intent = await steward(script(c, h));
    expect(intent).toContain("stood for coordinator");
    expect(intent).toContain("moved a resolution");
    expect(intent).toContain("voted yes on #0");
    const tools = c.turn.calls.map((x) => x.tool);
    expect(tools).toContain("stand");
    expect(tools).toContain("propose");
    expect(tools).not.toContain("take_position");
    expect(tools).not.toContain("set_labor");
  });
  it("the steward publishes a Plan when it sits", async () => {
    const h = home();
    const seated = { ...fixture<OfficesView>("GET /s/1/offices") };
    seated.offices = seated.offices.map((o) => ({ ...o, i_hold: true, election: null, holders: [{ citizen: h.citizen.id, handle: h.citizen.handle, term_ends_cycle: 4 }] }));
    let targets: unknown = null;
    const c = ctx({
      "PUT /s/1/plan": ok,
      "GET /s/1/offices": { status: 200, body: seated },
      "PUT /s/1/offices/coordinator/plan": ok,
      "POST /s/1/proposals": ok,
      "PUT /s/1/proposals/0/ballot": ok,
      "PUT /s/1/proposals/1/ballot": ok,
    });
    const s = script(c, h, { moved: true });
    const intent = await steward(s);
    expect(intent).toMatch(/published a Plan over \d+ workplaces/);
    targets = c.turn.calls.find((x) => x.tool === "publish_plan")?.input;
    expect(Object.keys((targets as { targets: Record<string, number> }).targets).length).toBeGreaterThan(0);
    expect(c.turn.calls.some((x) => x.tool === "open_workplace")).toBe(false); // the Store holds 0 Materials
  });
  it("takes a position where labor is scarcest when none is held", async () => {
    const h = home();
    const idle = { ...h, labor: { ...h.labor, positions: [], allocations: [] } };
    const c = ctx({ "PUT /s/1/plan": ok, "POST /s/1/workplaces/1/position": ok, "PUT /s/1/labor": ok, "GET /s/1/store": { status: 200, body: fixture("GET /s/1/store") }, "PUT /s/1/proposals/0/ballot": ok, "PUT /s/1/proposals/1/ballot": ok });
    const intent = await rationer(script(c, idle));
    expect(intent).toContain("took a position at workplace 1 and set 6 h");
    expect(c.turn.calls.find((x) => x.tool === "set_labor")?.input).toEqual({ allocations: [{ workplace: 1, hours: 6, effort: "normal" }] });
  });
  it("the rationer moves an hour onto the norm after a short day and votes by the hours", async () => {
    const store = fixture<{ yesterday: unknown[] }>("GET /s/1/store");
    const short = { ...store, yesterday: [{ good: "food", requested: 40, served: 30, short: 10, rationed_ticks: 3, shared: 0, shared_with: 0 }] };
    const c = ctx({ "PUT /s/1/plan": ok, "GET /s/1/store": { status: 200, body: short }, "POST /s/1/proposals": ok, "PUT /s/1/proposals/0/ballot": ok, "PUT /s/1/proposals/1/ballot": ok }, 6);
    const intent = await rationer(script(c));
    expect(intent).toContain("the Store ran short yesterday: food 10");
    expect(intent).toContain("moved the norm to 7");
    expect(c.turn.calls.find((x) => x.tool === "propose")?.input).toMatchObject({ kind: { policy_change: { patch: { work_norm_hours: 7 } } } });
    // "Seven hours" raises the norm: yes. The honor (#1) already carries my ballot in the recording, so it is left alone.
    expect(intent).toContain('voted yes on #0 "Seven hours"');
    expect(c.turn.calls.filter((x) => x.tool === "vote").map((x) => (x.input as { proposal: number }).proposal)).toEqual([0]);
  });
  it("the free-rider works two hours at low effort, moves an hour off, and votes no to seven", async () => {
    const c = ctx({ "PUT /s/1/plan": ok, "PUT /s/1/labor": ok, "POST /s/1/proposals": ok, "PUT /s/1/proposals/0/ballot": ok, "PUT /s/1/proposals/1/ballot": ok }, 6);
    const intent = await freeRider(script(c));
    expect(c.turn.calls.find((x) => x.tool === "set_labor")?.input).toEqual({ allocations: [{ workplace: 1, hours: 2, effort: "low" }] });
    expect(intent).toContain("moved the norm to 5");
    expect(intent).toContain('no on #0 "Seven hours"');
    expect(c.turn.calls.filter((x) => x.tool === "vote").map((x) => (x.input as { proposal: number }).proposal)).toEqual([0]);
  });
  it("the chronicler puts the count on a floor once, honors the top of the Ledger, and votes with the room", async () => {
    const c = ctx({ "PUT /s/1/plan": ok, "POST /s/1/channels/assembly%3A0/messages": { status: 201, body: {} }, "POST /s/1/proposals": ok, "PUT /s/1/proposals/0/ballot": ok, "PUT /s/1/proposals/1/ballot": ok }, 6);
    const memory: Record<string, unknown> = {};
    const intent = await chronicler(script(c, home(), memory));
    expect(intent).toContain("put the count on the floor of #0");
    expect(c.turn.calls.find((x) => x.tool === "post_floor")?.input).toMatchObject({ proposal: 0, body: expect.stringMatching(/^For the record, hour 2 of day 1: yes 0, no 0, abstain 0; 0 of the 2 a quorum needs\.$/) });
    expect(intent).toMatch(/moved to honor H-\d+/);
    // Nobody has voted on #0 (the roll is empty in the recording): the chronicler waits; #1 already carries my ballot.
    expect(c.turn.calls.some((x) => x.tool === "vote")).toBe(false);
    // One floor an hour: the second hour takes #1, the third finds nothing left to say.
    const c2 = ctx({ "POST /s/1/channels/assembly%3A0/messages": { status: 201, body: {} }, "POST /s/1/proposals": ok });
    await chronicler(script(c2, home(), memory));
    expect(c2.turn.calls.filter((x) => x.tool === "post_floor").map((x) => (x.input as { proposal: number }).proposal)).toEqual([1]);
    const c3 = ctx({ "POST /s/1/proposals": ok });
    await chronicler(script(c3, home(), memory));
    expect(c3.turn.calls.some((x) => x.tool === "post_floor")).toBe(false);
  });
  it("read helpers", () => {
    const p = fixture<ProposalsView>("GET /s/1/proposals");
    expect(normOf(p.open[0]!)).toBe(7);
    expect(normOf(p.open[1]!)).toBeNull();
    const e = electionFor(fixture<OfficesView>("GET /s/1/offices"), "coordinator");
    expect(e).toMatchObject({ open: true, iStand: true, iHold: false });
    expect(electionFor(null, "coordinator")).toBeNull();
  });
  it("live like householders where there is no assembly", async () => {
    const h = home();
    const c = ctx({ "PUT /s/1/plan": ok, "GET /s/1/notice-board": { status: 200, body: { offers: [] } } });
    const s = new Script(c, { clock: h.clock, home: h, notes: "", lastTurn: null }, {}, "x", FREEPORT);
    expect(await steward(s)).toBe("no job open; waited");
    expect(c.turn.calls.some((x) => x.tool === "offices")).toBe(false);
  });
});

describe("the rule-prober in the Commune", () => {
  it("adds governance probes there and leaves the Freeport list as it was", () => {
    const freeport = probesFor(FREEPORT);
    expect(freeport.map((p) => p.name)).toEqual(PROBES.filter((p) => !p.applies).map((p) => p.name));
    expect(freeport.some((p) => p.name.startsWith("vote"))).toBe(false);
    const commune = probesFor(COMMUNE).map((p) => p.name);
    for (const name of ["vote twice on one proposal", "an approval ballot naming a householder", "recall a householder from an office", "propose a Directorate field", "move an election", "publish a Plan without holding office", "hours past the budget under the norm"]) {
      expect(commune).toContain(name);
    }
  });
  it("judges a proposal id in a refusal as a copy finding", () => {
    const p = PROBES[0]!;
    expect(judge(p, { ok: false, status: 422, code: "x", title: "x", detail: "no open proposal p3" })).toMatch(/^COPY:/);
    expect(judge(p, { ok: false, status: 422, code: "x", title: "x", detail: "H-0 does not stand for Coordinator" })).toBeNull();
  });
  it("runs every governance probe against the recorded refusals without a defect", async () => {
    const brain = new ScriptedBrain(persona("rule-prober"), "rule-prober-1", COMMUNE);
    const h = home();
    const probes = probesFor(COMMUNE);
    (brain as unknown as { memory: Record<string, unknown> }).memory.planSet = true;
    const overrides = {
      "PUT /s/1/plan": { status: 422, body: { type: "about:blank", title: "rejected", status: 422, code: "unknown_org", detail: "no org 999999" } },
      "PUT /s/1/proposals/999999/ballot": refusal("PUT_ballot_missing"),
      "PUT /s/1/offices/coordinator/ballot": refusal("PUT_approve_householder"),
      "POST /s/1/proposals": refusal("POST_election"),
      "POST /s/1/channels/assembly%3A999999/messages": refusal("POST_floor_missing"),
      "POST /s/1/workplaces/999999/position": refusal("POST_position_missing"),
      "PUT /s/1/labor": { status: 422, body: { type: "about:blank", title: "rejected", status: 422, code: "invalid_quantity", detail: "the day has 16 hours" } },
      "POST /s/1/offices/coordinator/candidacy": refusal("POST_stand_twice"),
    };
    const notes: string[] = [];
    const intents: string[] = [];
    for (let i = 0; i < probes.length; i++) {
      const c = ctx(overrides, 4);
      const t = await brain.takeTurn(c, { clock: h.clock, home: h, notes: "", lastTurn: null });
      expect(t.ended_by).toBe("script");
      intents.push(t.intent);
      notes.push(...t.did_not_understand);
    }
    // The Freeport probes come first and are refused here too (no money, no firms); the governance ones follow.
    expect(intents.at(-1)).toContain('probed "hours past the budget under the norm"');
    expect(intents.some((i) => i.includes('probed "propose a Directorate field"'))).toBe(true);
    expect(notes.filter((n) => n.startsWith("DEFECT:"))).toEqual([]);
  });
  it("counts a double ballot once, and calls two of them on the roll a defect", async () => {
    const brain = new ScriptedBrain(persona("rule-prober"), "rule-prober-1", COMMUNE);
    const h = home();
    const probes = probesFor(COMMUNE);
    const at = probes.findIndex((p) => p.name === "vote twice on one proposal");
    const roll = (n: number) => {
      const p = fixture<ProposalsView>("GET /s/1/proposals").open[0]!;
      const ballots = Array.from({ length: n }, () => ({ citizen: h.citizen.id, handle: h.citizen.handle, ballot: "yes" }));
      return { status: 200, body: { ...p, ballots, tally: { ...p.tally, yes: n, cast: n } } };
    };
    const once = ctx({ "PUT /s/1/plan": ok, "PUT /s/1/proposals/0/ballot": ok, "GET /s/1/proposals/0": roll(1) });
    const b1 = new ScriptedBrain(persona("rule-prober"), "rule-prober-1", COMMUNE);
    (b1 as unknown as { memory: Record<string, unknown> }).memory.probe = at;
    const t1 = await b1.takeTurn(once, { clock: h.clock, home: h, notes: "", lastTurn: null });
    expect(t1.intent).toContain('probed "vote twice on one proposal"');
    expect(t1.did_not_understand).toEqual([]);
    const twice = ctx({ "PUT /s/1/plan": ok, "PUT /s/1/proposals/0/ballot": ok, "GET /s/1/proposals/0": roll(2) });
    (brain as unknown as { memory: Record<string, unknown> }).memory.probe = at;
    const t2 = await brain.takeTurn(twice, { clock: h.clock, home: h, notes: "", lastTurn: null });
    expect(t2.did_not_understand[0]).toMatch(/^DEFECT: 2 ballots of mine/);
  });
});

describe("the report's governance paragraph", () => {
  it("counts the governance tools and names the motions", () => {
    const turn = (player: string, cycle: number, calls: { tool: string; input: unknown; ok: boolean }[]) =>
      ({ kind: "turn", run: "t", player, persona: "x", brain: "scripted", model: null, epoch: 1, cycle, tick: 1, ms: 0, situation: { balance: 0, food: 50, shelter: 50, comfort: 50, housed: true, jobs: 1, pantry_food: 3 }, intent: "", calls: calls.map((c) => ({ ...c, ms: 0 })), rejections: [], did_not_understand: [], ended_by: "script", error: null, usage: { input: 0, output: 0, cache_read: 0, cache_write: 0, usd: 0 } }) as TurnRecord;
    const md = governanceParagraph([
      turn("steward-1", 1, [{ tool: "stand", input: { office: "coordinator" }, ok: true }, { tool: "propose", input: { title: "The Plan is a promise", kind: "resolution" }, ok: true }]),
      turn("rationer-1", 2, [{ tool: "vote", input: {}, ok: true }, { tool: "propose", input: { title: "An hour more", kind: { policy_change: {} } }, ok: false }, { tool: "set_labor", input: {}, ok: true }]),
    ]);
    expect(md).toContain("stand 1 accepted (steward-1)");
    expect(md).toContain("propose 1 accepted, 1 refused (steward-1, rationer-1)");
    expect(md).toContain('Motions moved: "The Plan is a promise" (resolution, steward-1, day 1).');
    expect(md).not.toContain("set_labor");
    expect(governanceParagraph([])).toContain("No governance tool was used");
  });
});
