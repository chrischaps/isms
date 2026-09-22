// The scripted brain: a strategy keyed by persona slug, the fuzzer for the
// rule-prober, and the householder-like floor for any slug without one.

import { ZERO_USAGE } from "../../journal/journal.ts";
import type { ToolContext } from "../../tools/context.ts";
import { FREEPORT, type SocietyFacts } from "../../society.ts";
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
  private readonly facts: SocietyFacts;
  /** `facts` are the society's capabilities (S2.10); a brain built without them plays Freeport. */
  constructor(persona: Persona, handle: string, facts: SocietyFacts = FREEPORT) {
    this.strategy = strategyFor(persona.slug);
    this.handle = handle;
    this.facts = facts;
  }
  async takeTurn(ctx: ToolContext, input: TurnInput): Promise<TurnOutcome> {
    const s = new Script(ctx, input, this.memory, this.handle, this.facts);
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
