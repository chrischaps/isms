// One player: a persona, a brain, an account, a journal, notes. It takes a
// turn when the cohort says the hour moved, reflects when the day ends, and
// never runs two things at once.

import { type Client, type Clock, type HomeView, unwrap } from "../api/client.ts";
import type { Brain, Persona } from "../brain/brain.ts";
import { Journal, type TurnRecord } from "../journal/journal.ts";
import { newTurn, type ToolContext } from "../tools/context.ts";
import { Notes } from "./notes.ts";
import { extract } from "./situation.ts";

export type PlayerOpts = {
  run: string;
  name: string;
  persona: Persona;
  brain: Brain;
  client: Client;
  sid: number;
  maxActions: number;
  journal: Journal;
  notes: Notes;
  now?: () => number;
};

export class Player {
  readonly name: string;
  readonly persona: Persona;
  readonly brain: Brain;
  private readonly o: PlayerOpts;
  private busy: Promise<unknown> = Promise.resolve();
  private lastTurn: TurnRecord | null = null;
  private cycleTurns: TurnRecord[] = [];
  skipped = 0;

  constructor(o: PlayerOpts) {
    this.o = o;
    this.name = o.name;
    this.persona = o.persona;
    this.brain = o.brain;
  }

  get isBusy(): boolean {
    return this.busyFlag;
  }
  private busyFlag = false;

  /** Serialise turns and reflections per player. */
  private async exclusive<T>(f: () => Promise<T>): Promise<T> {
    const prev = this.busy;
    let release!: () => void;
    this.busy = new Promise<void>((r) => (release = r));
    await prev;
    this.busyFlag = true;
    try {
      return await f();
    } finally {
      this.busyFlag = false;
      release();
    }
  }

  async home(): Promise<HomeView> {
    return unwrap(await this.o.client.GET("/s/{id}/home", { params: { path: { id: this.o.sid } } }));
  }

  skip(clock: Clock, reason: "overrun" | "budget") {
    this.skipped += 1;
    this.o.journal.append({ kind: "skip", run: this.o.run, player: this.name, epoch: clock.epoch, cycle: clock.cycle, tick: clock.tick, reason });
  }

  async takeTurn(clock: Clock): Promise<TurnRecord> {
    return this.exclusive(async () => {
      const now = this.o.now ?? Date.now;
      const started = now();
      const home = await this.home();
      const turn = newTurn(this.persona.max_actions_per_turn ?? this.o.maxActions);
      const ctx: ToolContext = { client: this.o.client, sid: this.o.sid, turn, now: this.o.now };
      const outcome = await this.brain.takeTurn(ctx, {
        clock,
        home,
        notes: this.o.notes.get(),
        lastTurn: this.lastTurn ? { intent: this.lastTurn.intent, rejections: this.lastTurn.rejections } : null,
      });
      const record: TurnRecord = {
        kind: "turn",
        run: this.o.run,
        player: this.name,
        persona: this.persona.slug,
        brain: this.brain.kind,
        model: this.brain.model,
        epoch: clock.epoch,
        cycle: clock.cycle,
        tick: clock.tick,
        ms: now() - started,
        situation: extract(home),
        intent: outcome.intent,
        calls: turn.calls,
        rejections: turn.calls.filter((c) => !c.ok).map((c) => ({ tool: c.tool, code: c.code ?? "?", detail: c.detail ?? "" })),
        did_not_understand: outcome.did_not_understand,
        ended_by: outcome.ended_by,
        error: outcome.error,
        usage: outcome.usage,
        ...(outcome.decisions ? { decisions: outcome.decisions } : {}),
      };
      this.o.journal.append(record);
      this.lastTurn = record;
      this.cycleTurns.push(record);
      return record;
    });
  }

  /** The day's turns as the cycle model reads them. */
  digest(): string {
    return this.cycleTurns
      .map((t) => {
        const lines = [`Hour ${t.tick}: ${t.intent}`];
        for (const r of t.rejections) lines.push(`  refused by ${r.tool} (${r.code}): "${r.detail}"`);
        for (const d of t.did_not_understand) lines.push(`  did not understand: ${d}`);
        return lines.join("\n");
      })
      .join("\n");
  }

  async reflect(clock: Clock): Promise<void> {
    if (!this.brain.reflect || this.cycleTurns.length === 0) {
      this.cycleTurns = [];
      return;
    }
    // Snapshot the day under the lock, then think outside it: the next day's
    // turns go on with yesterday's notes until the rewrite lands, instead of
    // skipping every hour a slow reflection takes.
    const { before, digest } = await this.exclusive(async () => {
      const snapshot = { before: this.o.notes.get(), digest: this.digest() };
      this.cycleTurns = [];
      return snapshot;
    });
    {
      const now = this.o.now ?? Date.now;
      const started = now();
      const r = await this.brain.reflect!({ clock, notes: before, digest });
      this.o.notes.set(r.notes, r.plan);
      this.o.journal.append({
        kind: "reflection",
        run: this.o.run,
        player: this.name,
        persona: this.persona.slug,
        model: this.brain.model ?? "?",
        epoch: clock.epoch,
        cycle: clock.cycle,
        ms: now() - started,
        notes_before: before,
        notes_after: r.notes,
        plan: r.plan,
        usage: r.usage,
      });
    }
  }
}
