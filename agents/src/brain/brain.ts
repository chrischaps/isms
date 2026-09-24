// The seam between "what a player is" and "how it decides". A brain takes a
// turn with the tools and nothing else: the scripted brain calls them from
// code, the LLM brain hands them to a model, the Jev brain has code lay out
// the choices and a typed-decision model pick (SJ.1). All leave the same trace
// in `ToolContext.turn`, so the journal and the report do not care which it was.

import type { Clock, HomeView } from "../api/client.ts";
import type { Usage } from "../journal/journal.ts";
import type { ToolContext } from "../tools/context.ts";

export type BrainKind = "scripted" | "llm" | "jev";

export type Persona = {
  name: string;
  slug: string;
  brain: BrainKind;
  goals: string[];
  temperament: string;
  risk: "low" | "medium" | "high";
  /** How much the persona cares where it stands on the scoreboard, in its own words; absent when it does not (SJ.2). */
  ambition: string | null;
  model: { turn?: string; cycle?: string };
  max_actions_per_turn: number | null;
  /** The prose the LLM brain reads; the scripted brain is keyed by slug. */
  body: string;
};

/** What a turn begins with: the player's own view and its running notes. */
export type TurnInput = {
  clock: Clock;
  home: HomeView;
  notes: string;
  lastTurn: { intent: string; rejections: { tool: string; code: string; detail: string }[] } | null;
};

/** One typed decision a Jev turn took: the slot asked, the option chosen, and whether it ran (SJ.1). */
export type Decision = { slot: string; option: string; confidence: number; none: boolean; acted: boolean };

export type TurnOutcome = {
  intent: string;
  did_not_understand: string[];
  ended_by: "end_turn" | "text" | "cap" | "budget" | "error" | "script";
  error: string | null;
  usage: Usage;
  /** Only a Jev turn has these; the report measures the brain by them. */
  decisions?: Decision[];
};

export type ReflectionInput = {
  clock: Clock;
  notes: string;
  /** The cycle's turns, digested: intents, rejections, confusions. */
  digest: string;
};

export type ReflectionOutcome = { notes: string; plan: string; usage: Usage };

export interface Brain {
  readonly kind: BrainKind;
  readonly model: string | null;
  takeTurn(ctx: ToolContext, input: TurnInput): Promise<TurnOutcome>;
  /** Once per cycle; a scripted brain has nothing to rewrite. */
  reflect?(input: ReflectionInput): Promise<ReflectionOutcome>;
}
