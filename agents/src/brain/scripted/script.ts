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
import { FREEPORT, type SocietyFacts } from "../../society.ts";

export type Offer = Schemas["OfferView"];
export type OrgView = Schemas["OrgView"];
export type OrgsView = Schemas["OrgsView"];
export type BooksView = Schemas["BooksView"];
export type ProposalsView = Schemas["ProposalsView"];
export type ProposalView = Schemas["ProposalView"];
export type OfficesView = Schemas["OfficesView"];
export type OfficeView = Schemas["OfficeView"];
export type StoreView = Schemas["StoreView"];
export type LedgerView = Schemas["ContributionView"];
export type PublishedPlanView = Schemas["PublishedPlanView"];
export type PositionView = Schemas["PositionView"];
export type ScoreboardView = Schemas["ScoreboardView"];

/** Per-player memory that survives between turns (a scripted brain's "notes"). */
export type Memory = Record<string, unknown>;

export class Script {
  readonly ctx: ToolContext;
  readonly input: TurnInput;
  readonly memory: Memory;
  readonly notes: string[] = [];
  readonly handle: string;
  /** The society's capabilities; Freeport's when none were read. */
  readonly facts: SocietyFacts;
  constructor(ctx: ToolContext, input: TurnInput, memory: Memory, handle: string, facts: SocietyFacts = FREEPORT) {
    this.ctx = ctx;
    this.input = input;
    this.memory = memory;
    this.handle = handle;
    this.facts = facts;
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
  get me(): number {
    return this.home.citizen.id;
  }
  /** Contract-less positions under a work norm (S2.7). */
  get positions(): PositionView[] {
    return (this.home.labor.positions ?? []).filter((p) => p.contract == null);
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
  async proposals(): Promise<ProposalsView | null> {
    const v = await this.get<ProposalsView>(read.proposals);
    return isRefusal(v) ? null : v;
  }
  async offices(): Promise<OfficesView | null> {
    const v = await this.get<OfficesView>(read.offices);
    return isRefusal(v) ? null : v;
  }
  async store(): Promise<StoreView | null> {
    const v = await this.get<StoreView>(read.store);
    return isRefusal(v) ? null : v;
  }
  async ledger(): Promise<LedgerView | null> {
    const v = await this.get<LedgerView>(read.ledger);
    return isRefusal(v) ? null : v;
  }
  async scoreboard(): Promise<ScoreboardView | null> {
    const v = await this.get<ScoreboardView>(read.scoreboard);
    return isRefusal(v) ? null : v;
  }
  async publishedPlan(): Promise<PublishedPlanView | null> {
    const v = await this.get<PublishedPlanView>(read.publishedPlan);
    return isRefusal(v) ? null : v;
  }
  /** The citizens' roll (the read tool caps it at 80 rows): handles and flags, for a lender reading defaults (SJ.3). */
  async citizens(): Promise<CitizenPublic[]> {
    const v = await this.get<{ rows: CitizenPublic[] }>(read.citizens);
    return isRefusal(v) ? [] : v.rows;
  }
  /** My contracts and those of the orgs I manage (SJ.3: a manager naming a worker's contract to end it). */
  async contracts(): Promise<ContractView[]> {
    const v = await this.get<ContractsView>(read.contracts);
    return isRefusal(v) ? [] : v.contracts;
  }
}

export type CitizenPublic = Schemas["CitizenPublic"];
export type ContractView = Schemas["ContractView"];
export type ContractsView = Schemas["ContractsView"];

/** The active employment contracts an org holds, by worker: what a manager may terminate on its behalf (SJ.3). */
export function workerContracts(contracts: ContractView[], org: number): { contract: number; citizen: number; hourly: number }[] {
  const out: { contract: number; citizen: number; hourly: number }[] = [];
  for (const k of contracts) {
    if (k.status !== "active") continue;
    const e = (k.body as { employment?: Record<string, unknown> }).employment;
    if (!e || Number(e.org) !== org) continue;
    // The engine's `(Party, Party)`: the employer first, the worker second.
    const parties = k.parties as unknown as { citizen?: number; org?: number }[] | undefined;
    const worker = Array.isArray(parties) ? parties.find((p) => p && typeof p.citizen === "number")?.citizen : undefined;
    if (worker === undefined) continue;
    const pay = (e.pay ?? {}) as { hourly?: number; piece_rate?: number };
    out.push({ contract: k.id, citizen: worker, hourly: pay.hourly ?? pay.piece_rate ?? 0 });
  }
  return out;
}

/** The cheapest rent asked on the board, else the preset's legacy rent (8.00 a day, TDD 9.3). */
export function goingRent(offers: Offer[]): number {
  const rents = offers
    .filter((o) => o.kind === "lease")
    .map((o) => (o.body as { lease?: { rent_per_cycle?: number } }).lease?.rent_per_cycle)
    .filter((r): r is number => typeof r === "number");
  return rents.length ? Math.min(...rents) : 800;
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

// -- the Commune: the norm, the assembly, the Store (S2.10) ----------------------

/** Work under the norm: take a position where labor is scarcest if none is held, then set the hours. */
export async function workTheNorm(s: Script, hours?: number, effort: "low" | "normal" | "high" = "normal"): Promise<string> {
  let position = s.positions[0];
  if (!position) {
    const ledger = await s.ledger();
    const wid = ledger?.least_staffed;
    if (wid == null) return "no workplace has room";
    if (!(await s.do(act.takePosition, { workplace: wid }))) return `could not take a position at workplace ${wid}`;
    const norm = ledger?.norm_hours ?? 6;
    const h = Math.min(hours ?? norm, s.home.labor.budget);
    await s.do(act.setLabor, { allocations: [{ workplace: wid, hours: h, effort }] });
    return `took a position at workplace ${wid} and set ${h} h`;
  }
  const norm = (s.memory.norm as number | undefined) ?? 6;
  const h = Math.min(hours ?? norm, s.home.labor.budget);
  const current = s.home.labor.allocations;
  const same = current.length === 1 && current[0]!.workplace === position.workplace && current[0]!.hours === h && current[0]!.effort === effort;
  if (same) return `working ${h} h at workplace ${position.workplace}`;
  const ok = await s.do(act.setLabor, { allocations: [{ workplace: position.workplace, hours: h, effort }] });
  return ok ? `set ${h} h at workplace ${position.workplace}` : `could not set ${h} h at workplace ${position.workplace}`;
}

/** What a policy-change proposal does to the work norm: the new hours, or null when it leaves them. */
export function normOf(p: ProposalView): number | null {
  const k = p.kind as { policy_change?: { patch?: { work_norm_hours?: number | null } } };
  const h = k.policy_change?.patch?.work_norm_hours;
  return typeof h === "number" ? h : null;
}

/** Ballots on every open proposal without one yet, as `decide` says; returns what was cast. */
export async function voteOnOpen(s: Script, decide: (p: ProposalView) => "yes" | "no" | "abstain" | null): Promise<string[]> {
  const v = await s.proposals();
  if (!v) return [];
  const cast: string[] = [];
  for (const p of v.open) {
    if (p.my_ballot != null || s.actionsLeft <= 0) continue;
    const ballot = decide(p);
    if (!ballot) continue;
    if (await s.do(act.vote, { proposal: p.id, ballot })) cast.push(`${ballot} on #${p.id} "${p.title}"`);
  }
  return cast;
}

/** The office's open election, if any, and whether I stand or sit. */
export function electionFor(offices: OfficesView | null, kind: string): { office: OfficeView; open: boolean; iStand: boolean; iHold: boolean } | null {
  const office = offices?.offices.find((o) => o.kind === kind);
  if (!office) return null;
  return { office, open: office.election != null, iStand: office.election?.i_stand ?? false, iHold: office.i_hold };
}
