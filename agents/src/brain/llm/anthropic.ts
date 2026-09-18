// The LLM brain on the Anthropic SDK. A turn is one fresh conversation: the
// rules and the persona as a cached system prefix, the situation as the user
// message, the tools as tools, the runner doing the loop. `end_turn` aborts
// the runner from inside, which spares the request that would otherwise only
// say goodbye. Once a day the cycle model rewrites the notes and the plan as
// structured output.

import Anthropic from "@anthropic-ai/sdk";
import { betaZodTool } from "@anthropic-ai/sdk/helpers/beta/zod";
import { zodOutputFormat } from "@anthropic-ai/sdk/helpers/zod";
import { z } from "zod";
import type { Budget } from "../../budget/budget.ts";
import { MODELS, type Config } from "../../config.ts";
import { addUsage, priceOf } from "../../budget/budget.ts";
import { ZERO_USAGE, type Usage } from "../../journal/journal.ts";
import { situationText } from "../../player/situation.ts";
import { personaText } from "../../player/persona.ts";
import { ACT_TOOLS } from "../../tools/act.ts";
import { invoke, type AnyTool, type ToolContext } from "../../tools/context.ts";
import { endTurn } from "../../tools/end_turn.ts";
import { READ_TOOLS } from "../../tools/read.ts";
import type { Brain, Persona, ReflectionInput, ReflectionOutcome, TurnInput, TurnOutcome } from "../brain.ts";
import { REFLECT_PROMPT, RULES, TURN_TEMPLATE } from "./prompts.ts";

export type LlmOpts = {
  client: Anthropic;
  persona: Persona;
  player: string;
  cfg: Config;
  budget: Budget;
};

const Reflection = z.object({
  notes: z.string().max(1600).describe("Your rewritten notes: what you know, what worked, what was refused and why, what you want."),
  plan: z.string().max(800).describe("A numbered plan for tomorrow, at most six lines."),
});

/** The thinking parameters a model takes (Haiku 4.5 wants a budget; the rest run adaptive with an effort). */
function thinkingFor(model: string, cfg: Config) {
  const info = MODELS[model];
  if (!info) throw new Error(`unknown model ${model}`);
  return info.thinking === "budget"
    ? { thinking: { type: "enabled" as const, budget_tokens: cfg.models.haiku_thinking_budget } }
    : { thinking: { type: "adaptive" as const }, output_config: { effort: cfg.models.turn_effort } };
}

export class AnthropicBrain implements Brain {
  readonly kind = "llm" as const;
  readonly model: string;
  private readonly cycleModel: string;
  private readonly opts: LlmOpts;
  private readonly system: Anthropic.Beta.Messages.BetaTextBlockParam[];
  constructor(opts: LlmOpts) {
    this.opts = opts;
    this.model = opts.persona.model.turn ?? opts.cfg.models.turn;
    this.cycleModel = opts.persona.model.cycle ?? opts.cfg.models.cycle;
    this.system = [
      { type: "text", text: RULES },
      { type: "text", text: personaText(opts.persona), cache_control: { type: "ephemeral" } },
    ];
  }

  private tools(ctx: ToolContext, aborter: AbortController) {
    const wrap = (t: AnyTool) =>
      betaZodTool({
        name: t.name,
        description: t.description,
        inputSchema: t.schema,
        run: async (args: unknown) => {
          const r = await invoke(t, args, ctx);
          if (t.kind === "end" && ctx.turn.ended) aborter.abort();
          return JSON.stringify(r);
        },
      });
    return [...READ_TOOLS, ...ACT_TOOLS, endTurn].map(wrap);
  }

  async takeTurn(ctx: ToolContext, input: TurnInput): Promise<TurnOutcome> {
    const { cfg, budget, player } = this.opts;
    const lastTurn = input.lastTurn
      ? [`You meant to: ${input.lastTurn.intent}`, ...input.lastTurn.rejections.map((r) => `Refused by ${r.tool} (${r.code}): "${r.detail}"`)].join("\n")
      : null;
    const aborter = new AbortController();
    let usage: Usage = { ...ZERO_USAGE };
    let lastText = "";
    let iterations = 0;
    const maxIterations = ctx.turn.maxActions + 8;
    const runner = this.opts.client.beta.messages.toolRunner(
      {
        model: this.model,
        max_tokens: cfg.turn.max_tokens,
        system: this.system,
        tools: this.tools(ctx, aborter),
        messages: [{ role: "user", content: TURN_TEMPLATE(situationText(input.home), input.notes, lastTurn) }],
        max_iterations: maxIterations,
        ...thinkingFor(this.model, cfg),
      },
      { signal: aborter.signal },
    );
    let endedBy: TurnOutcome["ended_by"] = "text";
    let error: string | null = null;
    try {
      for await (const message of runner) {
        iterations += 1;
        usage = addUsage(usage, budget.charge(this.model, player, message.usage));
        for (const block of message.content) if (block.type === "text") lastText = block.text;
        if (budget.exhausted()) {
          endedBy = "budget";
          aborter.abort();
          break;
        }
        if (message.stop_reason === "refusal") {
          endedBy = "error";
          error = "the model refused the turn";
          break;
        }
      }
      if (ctx.turn.ended) endedBy = "end_turn";
      else if (endedBy === "text" && iterations >= maxIterations) endedBy = "cap";
    } catch (e) {
      if (ctx.turn.ended) endedBy = "end_turn";
      else if (endedBy === "budget") {
        // aborted on purpose
      } else if (e instanceof Anthropic.APIUserAbortError) {
        endedBy = "cap";
      } else {
        endedBy = "error";
        error = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      }
    }
    const ended = ctx.turn.ended;
    return {
      intent: ended?.intent ?? (lastText.trim() || "(no intent stated)"),
      did_not_understand: ended?.did_not_understand ?? [],
      ended_by: endedBy,
      error,
      usage,
    };
  }

  async reflect(input: ReflectionInput): Promise<ReflectionOutcome> {
    const { budget, player } = this.opts;
    const response = await this.opts.client.messages.parse({
      model: this.cycleModel,
      max_tokens: 4096,
      system: [
        { type: "text", text: RULES },
        { type: "text", text: personaText(this.opts.persona), cache_control: { type: "ephemeral" } },
      ],
      messages: [{ role: "user", content: REFLECT_PROMPT(input.digest, input.notes) }],
      output_config: { format: zodOutputFormat(Reflection) },
    });
    const usage = budget.charge(this.cycleModel, player, response.usage);
    const parsed = response.parsed_output;
    if (!parsed) return { notes: input.notes, plan: "", usage };
    return { notes: parsed.notes, plan: parsed.plan, usage };
  }
}

/** For tests and cost checks: the price of one message on a model. */
export const price = priceOf;
