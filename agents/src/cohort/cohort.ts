// The cohort: every player takes a turn when the hour moves (a few at a time),
// reflects when the day ends, and the run stops at the epoch's end or the
// budget's, writing `summary.json` and a `/stats` snapshot either way.

import { appendFileSync, writeFileSync } from "node:fs";
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

/** One good's book at a day's end (SJ.5), in cents; null where nothing has traded or rests. */
export type PriceRow = { good: string; last: number | null; bid: number | null; ask: number | null; bids: number; asks: number };

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
  // The economy day by day (SJ.2): `/stats` at each day's end, its `last_cycle` aggregates being the day just closed, one line per day in `days.jsonl`;
  // and the day's headlines from `/chronicle` (SJ.3), so a day that breaks the pattern can be read, not guessed.
  const daysFile = join(opts.runDir, "days.jsonl");
  writeFileSync(daysFile, "");
  const headlinesOf = async (cycle: number): Promise<string[]> => {
    try {
      const v = unwrap(await opts.client.GET("/s/{id}/chronicle", { params: { path: { id: opts.sid }, query: { cycle } } }));
      return v.headlines.map((h) => h.text);
    } catch (e) {
      log(`could not read /chronicle for day ${cycle}: ${e instanceof Error ? e.message : String(e)}`);
      return [];
    }
  };
  // The books at the day's end (SJ.5): last price, best bid and ask, depth, per good, for the report's price graph.
  const pricesOf = async (): Promise<PriceRow[]> => {
    try {
      const v = unwrap(await opts.client.GET("/s/{id}/books", { params: { path: { id: opts.sid } } }));
      return v.books
        .filter((b) => !b.instrument.startsWith("share:"))
        .map((b) => ({ good: b.instrument, last: b.last_price ?? null, bid: b.best_bid ?? null, ask: b.best_ask ?? null, bids: b.bid_depth, asks: b.ask_depth }));
    } catch (e) {
      log(`could not read /books at the end of the day: ${e instanceof Error ? e.message : String(e)}`);
      return [];
    }
  };
  const snapshotDay = async (cycle: number, stats?: { live?: unknown; last_cycle?: unknown }) => {
    try {
      const v = stats ?? (unwrap(await opts.client.GET("/s/{id}/stats", { params: { path: { id: opts.sid } } })) as { live?: unknown; last_cycle?: unknown });
      const headlines = await headlinesOf(cycle);
      const prices = await pricesOf();
      appendFileSync(daysFile, JSON.stringify({ cycle, live: v.live ?? null, aggregates: v.last_cycle ?? null, headlines, prices }) + "\n");
    } catch (e) {
      log(`could not read /stats at the end of day ${cycle}: ${e instanceof Error ? e.message : String(e)}`);
    }
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
      track(snapshotDay(signal.clock.cycle - 1));
      for (const p of opts.players) {
        // Outside the turn pool: a reflection holds no slot, so the day's turns keep going.
        track(p.reflect(signal.clock).catch((e) => log(`${p.name}: reflection failed: ${e instanceof Error ? e.message : String(e)}`)));
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
    // The epoch's last day closed with the epoch, so no `cycle` signal snapshotted it.
    if (stoppedBy === "epoch_end") {
      const v = stats as { clock?: { cycle?: number }; live?: unknown; last_cycle?: unknown };
      await snapshotDay((v.clock?.cycle ?? cyclesSeen + 1) - 1, v);
    }
  } catch (e) {
    log(`could not read /stats at the end: ${e instanceof Error ? e.message : String(e)}`);
  }
  writeFileSync(join(opts.runDir, "stats.json"), JSON.stringify(stats, null, 2) + "\n");
  // Who ended where (SJ.2): the scoreboard, for the report's standings table.
  try {
    const board = unwrap(await opts.client.GET("/s/{id}/scoreboard", { params: { path: { id: opts.sid } } }));
    writeFileSync(join(opts.runDir, "scoreboard.json"), JSON.stringify(board, null, 2) + "\n");
  } catch (e) {
    log(`could not read /scoreboard at the end: ${e instanceof Error ? e.message : String(e)}`);
  }
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
