// The scripted brain: a strategy keyed by persona slug, the fuzzer for the
// rule-prober, and the householder-like floor for any slug without one.

import { ZERO_USAGE } from "../../journal/journal.ts";
import type { ToolContext } from "../../tools/context.ts";
import type { Brain, Persona, TurnInput, TurnOutcome } from "../brain.ts";
import { ruleProber } from "./fuzz.ts";
import { Script, type Memory } from "./script.ts";
import { STRATEGIES, householderLike, type Strategy } from "./strategies.ts";

export function strategyFor(slug: string): Strategy {
  if (slug === "rule-prober") return ruleProber;
  return STRATEGIES[slug] ?? householderLike;
}

export class ScriptedBrain implements Brain {
  readonly kind = "scripted" as const;
  readonly model = null;
  private readonly strategy: Strategy;
  private readonly memory: Memory = {};
  private readonly handle: string;
  constructor(persona: Persona, handle: string) {
    this.strategy = strategyFor(persona.slug);
    this.handle = handle;
  }
  async takeTurn(ctx: ToolContext, input: TurnInput): Promise<TurnOutcome> {
    const s = new Script(ctx, input, this.memory, this.handle);
    try {
      const intent = await this.strategy(s);
      return { intent, did_not_understand: s.notes, ended_by: "script", error: null, usage: ZERO_USAGE };
    } catch (e) {
      return {
        intent: "the script failed",
        did_not_understand: s.notes,
        ended_by: "error",
        error: e instanceof Error ? e.message : String(e),
        usage: ZERO_USAGE,
      };
    }
  }
}
