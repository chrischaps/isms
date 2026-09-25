// The cohort's day-end snapshot (SJ.2, SJ.3): `/stats` and the day's headlines
// from `/chronicle` land in `days.jsonl`, one line per closed day, the epoch's
// last day from the closing read.

import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import type { Client, Clock } from "../src/api/client.ts";
import { Budget } from "../src/budget/budget.ts";
import { runCohort } from "../src/cohort/cohort.ts";
import type { TickSignal } from "../src/stream/ticks.ts";

const clock = (cycle: number, epochEnded = false): Clock => ({ epoch: 1, cycle, tick: 1, ticks_per_cycle: 24, engine_tick: (cycle - 1) * 24, epoch_ended: epochEnded });

/** A client that answers the three reads the cohort makes, by path. */
function fakeClient(calls: string[]): Client {
  const GET = async (path: string, o: { params?: { query?: { cycle?: number } } }) => {
    calls.push(o.params?.query?.cycle !== undefined ? `${path}?cycle=${o.params.query.cycle}` : path);
    let data: unknown;
    if (path === "/s/{id}/stats") data = { clock: clock(3, true), live: { food_last_price: 131 }, last_cycle: { mean_cycle_wage: 60 } };
    else if (path === "/s/{id}/chronicle") {
      const c = o.params!.query!.cycle!;
      data = { clock: clock(3), cycle: c, headlines: [{ seq: c, ordinal: 0, cycle: c, tick: 0, text: `Day ${c} opens.` }, ...(c === 2 ? [{ seq: 9, ordinal: 1, cycle: 2, tick: 5, text: "A mine runs short of hands." }] : [])] };
    } else data = { clock: clock(3), rows: [] };
    return { data, response: { ok: true, status: 200 } as Response };
  };
  return { GET } as unknown as Client;
}

async function* signals(): AsyncIterable<TickSignal> {
  yield { kind: "tick", clock: clock(1), via: "poll" };
  yield { kind: "cycle", clock: clock(2), via: "poll" };
  yield { kind: "epoch_end", clock: clock(3, true), via: "poll" };
}

describe("the day-end snapshot", () => {
  it("writes each closed day's aggregates and headlines to days.jsonl, the last day from the closing read", async () => {
    const runDir = mkdtempSync(join(tmpdir(), "cohort-"));
    const calls: string[] = [];
    const summary = await runCohort({ run: "t", runDir, players: [], budget: new Budget(1000, 1), concurrency: 1, overrun: "skip", client: fakeClient(calls), sid: 1 }, signals());
    expect(summary.stopped_by).toBe("epoch_end");
    expect(summary.cycles_seen).toBe(1);
    const days = readFileSync(join(runDir, "days.jsonl"), "utf8")
      .split("\n")
      .filter((l) => l.trim() !== "")
      .map((l) => JSON.parse(l) as { cycle: number; aggregates: unknown; headlines: string[] });
    expect(days.map((d) => d.cycle)).toEqual([1, 2]);
    expect(days[0]).toMatchObject({ aggregates: { mean_cycle_wage: 60 }, headlines: ["Day 1 opens."] });
    expect(days[1]!.headlines).toEqual(["Day 2 opens.", "A mine runs short of hands."]);
    expect(calls.filter((c) => c.startsWith("/s/{id}/chronicle"))).toEqual(["/s/{id}/chronicle?cycle=1", "/s/{id}/chronicle?cycle=2"]);
  });
});
