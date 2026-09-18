// The seam between "what a player is" and "how it decides". A brain takes a
// turn with the tools and nothing else: the scripted brain calls them from
// code, the LLM brain hands them to a model. Both leave the same trace in
// `ToolContext.turn`, so the journal and the report do not care which it was.

import type { Clock, HomeView } from "../api/client.ts";
import type { Usage } from "../journal/journal.ts";
import type { ToolContext } from "../tools/context.ts";

export type Persona = {
  name: string;
  slug: string;
  brain: "scripted" | "llm";
  goals: string[];
  temperament: string;
  risk: "low" | "medium" | "high";
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

export type TurnOutcome = {
  intent: string;
  did_not_understand: string[];
  ended_by: "end_turn" | "text" | "cap" | "budget" | "error" | "script";
  error: string | null;
  usage: Usage;
};

export type ReflectionInput = {
  clock: Clock;
  notes: string;
  /** The cycle's turns, digested: intents, rejections, confusions. */
  digest: string;
};

export type ReflectionOutcome = { notes: string; plan: string; usage: Usage };

export interface Brain {
  readonly kind: "scripted" | "llm";
  readonly model: string | null;
  takeTurn(ctx: ToolContext, input: TurnInput): Promise<TurnOutcome>;
  /** Once per cycle; a scripted brain has nothing to rewrite. */
  reflect?(input: ReflectionInput): Promise<ReflectionOutcome>;
}
