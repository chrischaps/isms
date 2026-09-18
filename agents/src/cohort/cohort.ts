// The cohort: every player takes a turn when the hour moves (a few at a time),
// reflects when the day ends, and the run stops at the epoch's end or the
// budget's, writing `summary.json` and a `/stats` snapshot either way.

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Client, unwrap } from "../api/client.ts";
import type { Budget } from "../budget/budget.ts";
import type { TickSignal } from "../stream/ticks.ts";
import type { Player } from "../player/player.ts";

export type CohortOpts = {
  run: string;
  runDir: string;
  players: Player[];
  budget: Budget;
  concurrency: number;
  overrun: "skip" | "queue";
  /** Any player's client, for the closing `/stats`. */
  client: Client;
  sid: number;
  log?: (line: string) => void;
};

export type Summary = {
  run: string;
  stopped_by: "epoch_end" | "budget" | "signal";
  turns: number;
  skipped: number;
  ticks_seen: number;
  cycles_seen: number;
  players: { name: string; persona: string; brain: string; model: string | null; skipped: number }[];
  budget: ReturnType<Budget["summary"]>;
};

class Pool {
  private active = 0;
  private waiting: (() => void)[] = [];
  private readonly size: number;
  constructor(size: number) {
    this.size = size;
  }
  async run<T>(f: () => Promise<T>): Promise<T> {
    if (this.active >= this.size) await new Promise<void>((r) => this.waiting.push(r));
    this.active += 1;
    try {
      return await f();
    } finally {
      this.active -= 1;
      this.waiting.shift()?.();
    }
  }
}

export async function runCohort(opts: CohortOpts, ticks: AsyncIterable<TickSignal>): Promise<Summary> {
  const log = opts.log ?? (() => undefined);
  const pool = new Pool(opts.concurrency);
  let turns = 0;
  let ticksSeen = 0;
  let cyclesSeen = 0;
  let stoppedBy: Summary["stopped_by"] = "signal";
  const inFlight = new Set<Promise<unknown>>();
  const track = (p: Promise<unknown>) => {
    inFlight.add(p);
    p.finally(() => inFlight.delete(p)).catch(() => undefined);
  };

  for await (const signal of ticks) {
    if (opts.budget.exhausted()) {
      stoppedBy = "budget";
      log(`budget exhausted at day ${signal.clock.cycle} hour ${signal.clock.tick}`);
      break;
    }
    if (signal.kind === "epoch_end") {
      stoppedBy = "epoch_end";
      log("the epoch ended");
      break;
    }
    if (signal.kind === "cycle") {
      cyclesSeen += 1;
      log(`day ${signal.clock.cycle - 1} ended; players reflect`);
      for (const p of opts.players) {
        track(pool.run(() => p.reflect(signal.clock)).catch((e) => log(`${p.name}: reflection failed: ${e instanceof Error ? e.message : String(e)}`)));
      }
      continue;
    }
    ticksSeen += 1;
    log(`hour ${signal.clock.tick} of day ${signal.clock.cycle} (${signal.via})`);
    for (const p of opts.players) {
      if (p.isBusy && opts.overrun === "skip") {
        p.skip(signal.clock, "overrun");
        continue;
      }
      if (opts.budget.exhausted()) {
        p.skip(signal.clock, "budget");
        continue;
      }
      track(
        pool
          .run(() => p.takeTurn(signal.clock))
          .then((t) => {
            turns += 1;
            log(`  ${p.name}: ${t.intent}${t.rejections.length ? ` [${t.rejections.length} refused]` : ""}${t.usage.usd ? ` $${t.usage.usd.toFixed(3)}` : ""}`);
          })
          .catch((e) => log(`  ${p.name}: turn failed: ${e instanceof Error ? e.message : String(e)}`)),
      );
    }
  }
  await Promise.allSettled([...inFlight]);

  let stats: unknown = null;
  try {
    stats = unwrap(await opts.client.GET("/s/{id}/stats", { params: { path: { id: opts.sid } } }));
  } catch (e) {
    log(`could not read /stats at the end: ${e instanceof Error ? e.message : String(e)}`);
  }
  writeFileSync(join(opts.runDir, "stats.json"), JSON.stringify(stats, null, 2) + "\n");
  const summary: Summary = {
    run: opts.run,
    stopped_by: stoppedBy,
    turns,
    skipped: opts.players.reduce((n, p) => n + p.skipped, 0),
    ticks_seen: ticksSeen,
    cycles_seen: cyclesSeen,
    players: opts.players.map((p) => ({ name: p.name, persona: p.persona.slug, brain: p.brain.kind, model: p.brain.model, skipped: p.skipped })),
    budget: opts.budget.summary(),
  };
  writeFileSync(join(opts.runDir, "summary.json"), JSON.stringify(summary, null, 2) + "\n");
  return summary;
}
