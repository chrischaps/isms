// The tool layer against a recorded API (no server): reads return the server's
// view, acts return the problem document verbatim on a refusal, every act
// counts against the cap, and the harness's own refusals carry their codes.

import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeClient, type HomeView } from "../src/api/client.ts";
import { fixtureTransport } from "../src/api/transport.ts";
import * as act from "../src/tools/act.ts";
import { HARNESS_ACTION_CAP, HARNESS_BAD_INPUT, HARNESS_TURN_OVER, invoke, newTurn, type ToolContext } from "../src/tools/context.ts";
import { endTurn } from "../src/tools/end_turn.ts";
import * as read from "../src/tools/read.ts";

const FIX = join(import.meta.dirname, "fixtures", "api");
const BASE = "http://127.0.0.1:18090";

const problem = (code: string, detail: string) => ({ type: "about:blank", title: "rejected", status: 422, code, detail });

function ctx(overrides: Parameters<typeof fixtureTransport>[1] = {}, maxActions = 4): ToolContext {
  const client = makeClient({ baseUrl: BASE, auth: { key: "isms_test.key" }, transport: fixtureTransport(FIX, overrides) });
  return { client, sid: 1, turn: newTurn(maxActions), now: () => 0 };
}

describe("read tools", () => {
  it("return the server's view and never count as actions", async () => {
    const c = ctx();
    const r = await invoke(read.home, {}, c);
    expect(r.ok).toBe(true);
    const home = (r as { data: HomeView }).data;
    expect(home.citizen.handle).toBe("rule-prober-1");
    expect(home.clock.ticks_per_cycle).toBe(24);
    const board = await invoke(read.noticeBoard, {}, c);
    expect(board.ok).toBe(true);
    const orgs = await invoke(read.orgs, {}, c);
    expect(orgs.ok && (orgs.data as { recipes: unknown[] }).recipes.length).toBeGreaterThan(0);
    expect(c.turn.actionsUsed).toBe(0);
    expect(c.turn.calls.map((x) => x.tool)).toEqual(["home", "notice_board", "orgs"]);
  });
  it("say when a list was cut", async () => {
    const rows = Array.from({ length: 50 }, (_, i) => ({ seq: i, kind: "Paid", tick: i, cycle: 0, epoch: 0, payload: {} }));
    const c = ctx({ "GET /s/1/payslips": { status: 200, body: { clock: {}, payslips: rows } } });
    const r = await invoke(read.payslips, {}, c);
    expect(r.ok && (r.data as { note: string }).note).toBe("showing the last 40 of 50");
    expect(r.ok && (r.data as { rows: unknown[] }).rows.length).toBe(40);
  });
  it("hand a missing endpoint back as a refusal, not a throw", async () => {
    const r = await invoke(read.scoreboard, {}, ctx());
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.status).toBe(404);
  });
});

describe("act tools", () => {
  it("return the problem document verbatim on a refusal and still count the action", async () => {
    const c = ctx({ "POST /s/1/orders": { status: 422, body: problem("insufficient_funds", "Citizen(c47) has 965.94") } });
    const r = await invoke(act.placeOrder, { instrument: "food", side: "bid", qty: 1_000_000, limit_price: 100 }, c);
    expect(r).toEqual({ ok: false, status: 422, code: "insufficient_funds", title: "rejected", detail: "Citizen(c47) has 965.94" });
    expect(c.turn.actionsUsed).toBe(1);
    expect(c.turn.calls[0]).toMatchObject({ tool: "place_order", ok: false, code: "insufficient_funds", detail: "Citizen(c47) has 965.94" });
  });
  it("summarise what a command did", async () => {
    const committed = { clock: { epoch: 1, cycle: 1, tick: 3, engine_tick: 2, ticks_per_cycle: 24, epoch_ended: false }, first_seq: 900, events: [{ seq: 900, kind: "OrderPlaced", tick: 2, cycle: 0, epoch: 0, payload: {} }] };
    const c = ctx({ "POST /s/1/orders": { status: 200, body: committed } });
    const r = await invoke(act.placeOrder, { instrument: "food", side: "bid", qty: 2, limit_price: 130 }, c);
    expect(r.ok && r.data).toEqual({ clock: committed.clock, first_seq: 900, events: ["OrderPlaced#900"] });
  });
  it("refuse bad input before the network, with the harness's own code", async () => {
    const c = ctx();
    const r = await invoke(act.placeOrder, { instrument: "food", side: "buy", qty: 0, limit_price: -1 }, c);
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.code).toBe(HARNESS_BAD_INPUT);
      expect(r.detail).toContain("side");
    }
    expect(c.turn.actionsUsed).toBe(0);
  });
  it("stop at the cap, accepted or refused", async () => {
    const c = ctx({ "POST /s/1/transfers": { status: 422, body: problem("self_deal", "cannot transfer to yourself") } }, 2);
    const t = { to: { citizen: 1 }, asset: { money: 1 }, memo: "" };
    expect((await invoke(act.transfer, t, c)).ok).toBe(false);
    expect((await invoke(act.transfer, t, c)).ok).toBe(false);
    const r = await invoke(act.transfer, t, c);
    if (!r.ok) expect(r.code).toBe(HARNESS_ACTION_CAP);
    expect(c.turn.actionsUsed).toBe(2);
    expect(c.turn.calls.length).toBe(3);
  });
  it("end the turn and refuse anything after", async () => {
    const c = ctx();
    const r = await invoke(endTurn, { intent: "rest", did_not_understand: ["what is a ration card"] }, c);
    expect(r.ok).toBe(true);
    expect(c.turn.ended).toEqual({ intent: "rest", did_not_understand: ["what is a ration card"] });
    const after = await invoke(read.home, {}, c);
    if (!after.ok) expect(after.code).toBe(HARNESS_TURN_OVER);
  });
  it("send the wire shapes the web client sends", async () => {
    let body: unknown = null;
    const transport = async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
      const req = new Request(input, init);
      body = JSON.parse(await req.text());
      return new Response(JSON.stringify({ clock: {}, events: [], first_seq: null }), { status: 200, headers: { "content-type": "application/json" } });
    };
    const c: ToolContext = { client: makeClient({ baseUrl: BASE, auth: { key: "k" }, transport }), sid: 1, turn: newTurn(9) };
    await invoke(act.offerSale, { asset: { good: ["food", 10] }, price: { money: 1500 } }, c);
    expect(body).toEqual({ asset: { good: ["food", 10] }, price: { money: 1500 }, to: null, on_behalf_of: null });
    await invoke(act.offerEmployment, { org: 3, workplace: 7, pay: { hourly: 800 }, max_hours: 8, places: 2 }, c);
    expect(body).toEqual({ workplace: 7, pay: { hourly: 800 }, max_hours: 8, places: 2, notice_cycles: 1, term_cycles: null });
    await invoke(act.setPlan, { keep_food_at_least: 24, keep_balance_at_least: 500 }, c);
    expect(body).toEqual({
      plan: { labor: "explicit", keep_food_at_least: 24, max_food_price: null, buy_wares_when: null, keep_balance_at_least: 500, standing_orders: [], vote_default: "abstain" },
    });
  });
});
