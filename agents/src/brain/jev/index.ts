// The Jev brain (SJ.1): code reads the game and lays out this hour's choices
// with their numbers worked out; one call to TypeSafe's decision model picks
// an option per slot; code runs what was picked, hours first. Jev writes no
// words, so the intent is templated and `did_not_understand` only ever holds
// an answer that named a slot or an option we never offered.

import type { Transport } from "../../api/transport.ts";
import type { Budget } from "../../budget/budget.ts";
import type { Config } from "../../config.ts";
import { ZERO_USAGE, type Usage } from "../../journal/journal.ts";
import { FREEPORT, type SocietyFacts } from "../../society.ts";
import type { ToolContext } from "../../tools/context.ts";
import type { Brain, Decision, Persona, TurnInput, TurnOutcome } from "../brain.ts";
import { ensurePlan, Script, type Memory } from "../scripted/script.ts";
import { layOut, type Candidate, type Slot } from "./candidates.ts";
import { decide, JevError, type Answer } from "./client.ts";
import { buildQuestions } from "./questions.ts";
import { buildState, standingOf, standingText } from "./state.ts";

export type JevOpts = {
  persona: Persona;
  player: string;
  handle: string;
  cfg: Config;
  budget: Budget;
  facts?: SocietyFacts;
  key: string;
  transport: Transport;
};

/** The share of the probability mass an irreversible choice must carry besides its confidence. */
const IRREVERSIBLE_MASS = 0.4;

export class JevBrain implements Brain {
  readonly kind = "jev" as const;
  readonly model: string;
  private readonly opts: JevOpts;
  private readonly memory: Memory = {};
  private readonly facts: SocietyFacts;
  /** Set when the API refused the key or the credit; every later turn is an error without a request. */
  dead: string | null = null;

  constructor(opts: JevOpts) {
    this.opts = opts;
    this.model = opts.cfg.jev.model;
    this.facts = opts.facts ?? FREEPORT;
  }

  async takeTurn(ctx: ToolContext, input: TurnInput): Promise<TurnOutcome> {
    const { cfg, budget, persona, player } = this.opts;
    if (this.dead) {
      return { intent: "(the decisions API refused this player for the run)", did_not_understand: [], ended_by: "error", error: this.dead, usage: { ...ZERO_USAGE }, decisions: [] };
    }
    const s = new Script(ctx, input, this.memory, this.opts.handle, this.facts);
    let usage: Usage = { ...ZERO_USAGE };
    try {
      await ensurePlan(s);
      const slots = await layOut(persona.slug, s);
      // Where I stand (SJ.2): read once an hour; yesterday's net worth is kept from the first read of each day.
      const standing = standingOf(await s.scoreboard(), s.me, this.memory.netWorthYesterday);
      if (standing && this.memory.netWorthDay !== input.clock.cycle) {
        this.memory.netWorthYesterday = this.memory.netWorthToday;
        this.memory.netWorthToday = Number(standing.net_worth_credits) * 100;
        this.memory.netWorthDay = input.clock.cycle;
      }
      const state = buildState(input, slots, this.memory, standing);
      this.memory.lastFood = input.home.needs.food;
      if (slots.length === 0) {
        this.memory.lastChosen = {};
        return { intent: "nothing to decide this hour; held", did_not_understand: [], ended_by: "script", error: null, usage, decisions: [] };
      }
      const questions = buildQuestions(persona, slots, standingText(standing));
      const res = await decide({ model: this.model, state, questions }, { baseUrl: cfg.jev.base_url, key: this.opts.key, transport: this.opts.transport });
      if (res.usage) usage = budget.charge(this.model, player, { input_tokens: res.usage.input_tokens, output_tokens: res.usage.output_tokens });

      const notUnderstood: string[] = [];
      const picks: { slot: Slot; candidate: Candidate; decision: Decision }[] = [];
      for (const slot of slots) {
        const a = res.answers[slot.key];
        const pick = this.read(slot, a, cfg);
        if (typeof pick === "string") {
          notUnderstood.push(pick);
          continue;
        }
        picks.push(pick);
      }
      for (const key of Object.keys(res.answers)) if (!slots.some((x) => x.key === key)) notUnderstood.push(`answered a slot never asked: ${key}`);

      // Hours first, then the job, then the persona's own business; surer choices first within a tier; a stable order after that.
      picks.sort((x, y) => x.slot.tier - y.slot.tier || y.decision.confidence - x.decision.confidence || x.slot.key.localeCompare(y.slot.key));
      const parts: string[] = [];
      const chosen: Record<string, string> = {};
      for (const p of picks) {
        chosen[p.slot.key] = p.candidate.option;
        const conf = p.decision.confidence.toFixed(2);
        if (!p.decision.acted) {
          parts.push(`${p.slot.key} ${p.candidate.option} (${conf})`);
          continue;
        }
        if (s.actionsLeft <= 0) {
          p.decision.acted = false;
          parts.push(`${p.slot.key} ${p.candidate.option} (${conf}, no actions left)`);
          continue;
        }
        const what = await p.candidate.act!(s);
        parts.push(`${p.slot.key} ${p.candidate.option} (${conf}): ${what}`);
      }
      this.memory.lastChosen = chosen;
      return {
        intent: parts.length ? parts.join("; ") : "considered the choices; held",
        did_not_understand: notUnderstood,
        ended_by: "script",
        error: null,
        usage,
        decisions: picks.map((p) => p.decision),
        // The request as sent, beside what came of it (E-6): what a slot offered and never got chosen is readable from the journal.
        ...(cfg.jev.journal_questions ? { request: { state, questions } } : {}),
      };
    } catch (e) {
      if (e instanceof JevError && e.fatal) this.dead = e.message;
      return {
        intent: "the decision call failed",
        did_not_understand: [],
        ended_by: "error",
        error: e instanceof Error ? `${e.name}: ${e.message}` : String(e),
        usage,
        decisions: [],
      };
    }
  }

  /** One answer against its slot: the candidate and the decision record, or a line for `did_not_understand`. */
  private read(slot: Slot, a: Answer | undefined, cfg: Config): { slot: Slot; candidate: Candidate; decision: Decision } | string {
    if (!a || a.choice === undefined) return `no choice came back for slot ${slot.key}`;
    const candidate = slot.candidates.find((c) => c.option === a.choice);
    if (!candidate) return `slot ${slot.key}: chose "${a.choice}", which was not offered (${slot.candidates.map((c) => c.option).join(", ")})`;
    const confidence = a.confidence ?? 0;
    const mass = a.probabilities?.[a.choice] ?? 0;
    const none = candidate.act === null;
    // A slot may carry its own floor (SJ.3); the run's floor otherwise.
    const floor = cfg.jev.floors[slot.key] ?? cfg.jev.min_confidence;
    let acted = !none && confidence >= floor;
    if (acted && candidate.irreversible) acted = confidence >= cfg.jev.irreversible_confidence && mass >= IRREVERSIBLE_MASS;
    return { slot, candidate, decision: { slot: slot.key, offered: slot.candidates.map((c) => c.option), option: candidate.option, confidence, none, acted } };
  }
}
