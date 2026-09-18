// What a scripted strategy gets to work with: the same tools, through `invoke`,
// so every call is capped and journaled exactly like a model's. A strategy is
// a function of the turn input that returns its intent in one line; `s.note`
// gathers anything it wants the journal to keep as "did not understand" (for
// the fuzzer, a defect).

import type { Clock, HomeView, Schemas } from "../../api/client.ts";
import { invoke, type AnyTool, type Refusal, type ToolContext } from "../../tools/context.ts";
import * as act from "../../tools/act.ts";
import * as read from "../../tools/read.ts";
import type { TurnInput } from "../brain.ts";

export type Offer = Schemas["OfferView"];
export type OrgView = Schemas["OrgView"];
export type OrgsView = Schemas["OrgsView"];
export type BooksView = Schemas["BooksView"];

/** Per-player memory that survives between turns (a scripted brain's "notes"). */
export type Memory = Record<string, unknown>;

export class Script {
  readonly ctx: ToolContext;
  readonly input: TurnInput;
  readonly memory: Memory;
  readonly notes: string[] = [];
  readonly handle: string;
  constructor(ctx: ToolContext, input: TurnInput, memory: Memory, handle: string) {
    this.ctx = ctx;
    this.input = input;
    this.memory = memory;
    this.handle = handle;
  }

  get home(): HomeView {
    return this.input.home;
  }
  get clock(): Clock {
    return this.input.clock;
  }
  get balance(): number {
    return this.home.household.balance;
  }
  get pantry(): Record<string, number> {
    return this.home.household.pantry as Record<string, number>;
  }
  get food(): number {
    return this.home.needs.food;
  }
  get employed(): boolean {
    return this.home.labor.employment.length > 0;
  }
  get actionsLeft(): number {
    return this.ctx.turn.maxActions - this.ctx.turn.actionsUsed;
  }

  note(s: string) {
    this.notes.push(s);
  }

  /** A read: the data, or a refusal. */
  async get<T>(tool: AnyTool, input: unknown = {}): Promise<T | Refusal> {
    const r = await invoke(tool, input, this.ctx);
    return r.ok ? (r.data as T) : r;
  }

  /** An act: `true` when accepted; the refusal is left in the journal either way. */
  async do(tool: AnyTool, input: unknown): Promise<boolean> {
    if (this.actionsLeft <= 0) return false;
    const r = await invoke(tool, input, this.ctx);
    return r.ok;
  }

  async board(): Promise<Offer[]> {
    const v = await this.get<Schemas["NoticeBoardView"]>(read.noticeBoard);
    return isRefusal(v) ? [] : v.offers;
  }
  async orgs(): Promise<OrgsView | null> {
    const v = await this.get<OrgsView>(read.orgs);
    return isRefusal(v) ? null : v;
  }
  async books(): Promise<BooksView | null> {
    const v = await this.get<BooksView>(read.books);
    return isRefusal(v) ? null : v;
  }
}

export function isRefusal(v: unknown): v is Refusal {
  return typeof v === "object" && v !== null && (v as { ok?: unknown }).ok === false;
}

// -- offers as the scripts read them -------------------------------------------

export type JobOffer = {
  id: number;
  org: number;
  workplace: number;
  hourly: number;
  maxHours: number;
  places: number;
};

/** Employment offers with an hourly-equivalent pay (a piece rate is taken as one unit an hour). */
export function jobOffers(offers: Offer[]): JobOffer[] {
  const out: JobOffer[] = [];
  for (const o of offers) {
    if (o.kind !== "employment") continue;
    const e = (o.body as { employment?: Record<string, unknown> }).employment;
    if (!e) continue;
    const pay = e.pay as { hourly?: number; piece_rate?: number };
    const hourly = pay.hourly ?? pay.piece_rate ?? 0;
    out.push({
      id: o.id,
      org: Number(e.org),
      workplace: Number(e.workplace),
      hourly,
      maxHours: Number(e.max_hours),
      places: Number(e.places),
    });
  }
  return out.sort((a, b) => b.hourly - a.hourly);
}

/** Contracts the citizen may still work: one in its notice period stays listed but labor there is refused (`not_assigned`). */
export function currentJobs(home: HomeView): { contract: number; org: number; workplace: number; hourly: number; maxHours: number }[] {
  return home.labor.employment.filter((c) => c.status === "active").map((c) => {
    const e = (c.body as { employment?: Record<string, unknown> }).employment ?? {};
    const pay = (e.pay ?? {}) as { hourly?: number; piece_rate?: number };
    return {
      contract: c.id,
      org: Number(e.org),
      workplace: Number(e.workplace),
      hourly: pay.hourly ?? pay.piece_rate ?? 0,
      maxHours: Number(e.max_hours ?? 8),
    };
  });
}

/** Work every job to its contract hours, within the day's budget. */
export async function workFullHours(s: Script, effort: "low" | "normal" | "high" = "normal", hoursCap?: number): Promise<boolean> {
  const jobs = currentJobs(s.home);
  if (jobs.length === 0) return false;
  let budget = Math.min(s.home.labor.budget, hoursCap ?? s.home.labor.budget);
  const allocations = [];
  for (const j of jobs.slice(0, 2)) {
    const hours = Math.max(0, Math.min(j.maxHours, budget));
    budget -= hours;
    allocations.push({ workplace: j.workplace, hours, effort });
  }
  const current = s.home.labor.allocations;
  const same =
    current.length === allocations.length &&
    allocations.every((a) => current.some((c) => c.workplace === a.workplace && c.hours === a.hours && c.effort === a.effort));
  if (same) return true;
  return s.do(act.setLabor, { allocations });
}

/** Take the best open job when unemployed: what a householder does, and the floor under every persona. */
export async function takeAJob(s: Script): Promise<JobOffer | null> {
  if (s.employed) return null;
  const jobs = jobOffers(await s.board());
  for (const j of jobs) {
    if (await s.do(act.acceptOffer, { offer: j.id })) return j;
  }
  return null;
}

/** Keep Food coming: a plan that buys the shortfall, once. */
export async function ensurePlan(s: Script, keepFood = 24, keepBalance = 0): Promise<void> {
  if (s.memory.planSet) return;
  const ok = await s.do(act.setPlan, {
    labor: "explicit",
    keep_food_at_least: keepFood,
    max_food_price: null,
    buy_wares_when: null,
    keep_balance_at_least: keepBalance,
    standing_orders: [],
  });
  if (ok) s.memory.planSet = true;
}

export function lastPrice(books: BooksView | null, instrument: string): number | null {
  const b = books?.books.find((x) => x.instrument === instrument);
  return b?.last_price ?? b?.best_ask ?? null;
}

export function median(xs: number[]): number {
  if (xs.length === 0) return 0;
  const s = [...xs].sort((a, b) => a - b);
  const m = Math.floor(s.length / 2);
  return s.length % 2 ? s[m]! : Math.round((s[m - 1]! + s[m]!) / 2);
}
