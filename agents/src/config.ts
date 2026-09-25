// Run configuration: agents/config.toml, then the environment, then CLI flags
// (most specific wins). Every model the harness may use is listed here with
// its price and the shape of its thinking parameter; an unknown id is refused
// before a token is spent.

import { readFileSync } from "node:fs";
import { parse } from "smol-toml";
import { z } from "zod";

/** USD per million tokens (platform.claude.com/docs/en/about-claude/pricing, 2026-09). */
export type ModelInfo = {
  id: string;
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  /** `adaptive` models take `thinking: {type: "adaptive"}` and `output_config.effort`; Haiku 4.5 takes a budget. A decision model thinks in neither way. */
  thinking?: "adaptive" | "budget";
};

export const MODELS: Record<string, ModelInfo> = {
  "claude-haiku-4-5": { id: "claude-haiku-4-5", input: 1, output: 5, cacheRead: 0.1, cacheWrite: 1.25, thinking: "budget" },
  "claude-sonnet-5": { id: "claude-sonnet-5", input: 2, output: 10, cacheRead: 0.2, cacheWrite: 2.5, thinking: "adaptive" },
  "claude-opus-5": { id: "claude-opus-5", input: 5, output: 25, cacheRead: 0.5, cacheWrite: 6.25, thinking: "adaptive" },
  /** TypeSafe's Jev on OpenRouter (openrouter.ai/typesafe/jev-1.13, 2026-09): input only, output free (SJ.1). */
  "~typesafe/jev-latest": { id: "~typesafe/jev-latest", input: 0.042, output: 0, cacheRead: 0, cacheWrite: 0 },
};

const modelId = z.string().refine((id) => id in MODELS, {
  message: `unknown model; one of ${Object.keys(MODELS).join(", ")}`,
});
/** A turn or cycle model must be one that thinks; the decision model has its own field. */
const llmModelId = modelId.refine((id) => MODELS[id]?.thinking !== undefined, { message: "not a language model" });

export const ConfigSchema = z.object({
  models: z
    .object({
      turn: llmModelId.default("claude-sonnet-5"),
      cycle: llmModelId.default("claude-opus-5"),
      /** `api`: the SDK on API credit. `claude_code`: `claude -p` on the account's subscription. */
      cycle_provider: z.enum(["api", "claude_code"]).default("api"),
      /** CLI reflections at once; each is a whole Claude Code runtime. */
      claude_code_concurrency: z.number().int().min(1).default(1),
      turn_effort: z.enum(["low", "medium", "high"]).default("low"),
      haiku_thinking_budget: z.number().int().min(1024).default(2048),
    })
    .prefault({}),
  turn: z
    .object({
      max_actions_per_turn: z.number().int().min(1).default(4),
      max_tokens: z.number().int().min(256).default(4096),
      concurrency: z.number().int().min(1).default(4),
      overrun: z.enum(["skip", "queue"]).default("skip"),
    })
    .prefault({}),
  budget: z
    .object({
      max_tokens: z.number().int().min(1).default(60_000_000),
      max_usd: z.number().min(0).default(40),
    })
    .prefault({}),
  run: z
    .object({
      players: z.number().int().min(1).default(8),
      poll_ms: z.number().int().min(250).default(3000),
      personas: z
        .array(z.string())
        .default(["founder", "wage-maximiser", "speculator", "saver", "slacker", "borrower", "landlord", "rule-prober"]),
      /** A preset's own persona list (S2.10); `personas` is Freeport's and the fallback. */
      personas_for: z.record(z.string(), z.array(z.string())).default({}),
      brain: z.enum(["scripted", "llm", "mixed", "jev"]).default("mixed"),
    })
    .prefault({}),
  /** The typed-decision brain (SJ.1): code lays out the choices, Jev picks. */
  jev: z
    .object({
      model: modelId.default("~typesafe/jev-latest"),
      base_url: z.string().url().default("https://openrouter.ai/api/alpha/decisions"),
      /** With `run.brain = "jev"`, these personas play on Jev; every other one is scripted. */
      personas: z.array(z.string()).default(["founder", "wage-maximiser", "speculator", "saver", "slacker", "borrower", "landlord"]),
      /** A choice below this confidence is treated as its slot's do-nothing option. */
      min_confidence: z.number().min(0).max(1).default(0.35),
      /** An irreversible choice (switch jobs, found, buy a dwelling) needs at least this, and 0.4 of the probability mass. */
      irreversible_confidence: z.number().min(0).max(1).default(0.5),
      /** A floor for one named slot, over `min_confidence` (SJ.3): `market` is the slot that flips. */
      floors: z.record(z.string(), z.number().min(0).max(1)).default({}),
      /** Keep each turn's request (the state and the questions, verbatim) in the journal beside its decisions (E-6); `--journal-questions`. */
      journal_questions: z.boolean().default(false),
    })
    .prefault({}),
});
export type Config = z.infer<typeof ConfigSchema>;

/** What the CLI may override; every field optional. */
export type Overrides = {
  players?: number;
  brain?: Config["run"]["brain"];
  turnModel?: string;
  cycleModel?: string;
  cycleProvider?: Config["models"]["cycle_provider"];
  maxUsd?: number;
  journalQuestions?: boolean;
};

export function loadConfig(path: string, overrides: Overrides = {}): Config {
  const raw = parse(readFileSync(path, "utf8"));
  return applyOverrides(ConfigSchema.parse(raw), overrides);
}

export function applyOverrides(cfg: Config, overrides: Overrides): Config {
  if (overrides.players !== undefined) cfg.run.players = overrides.players;
  if (overrides.brain !== undefined) cfg.run.brain = overrides.brain;
  if (overrides.turnModel !== undefined) cfg.models.turn = llmModelId.parse(overrides.turnModel);
  if (overrides.cycleModel !== undefined) cfg.models.cycle = llmModelId.parse(overrides.cycleModel);
  if (overrides.cycleProvider !== undefined) cfg.models.cycle_provider = overrides.cycleProvider;
  if (overrides.maxUsd !== undefined) cfg.budget.max_usd = overrides.maxUsd;
  if (overrides.journalQuestions !== undefined) cfg.jev.journal_questions = overrides.journalQuestions;
  return cfg;
}

/** The environment the harness reads; nothing else is consulted. */
export type Env = {
  ismsUrl: string;
  society: number;
  serverBin: string;
  databaseUrl: string | undefined;
  anthropicKey: string | undefined;
  openrouterKey: string | undefined;
};

export function readEnv(env: NodeJS.ProcessEnv = process.env): Env {
  const society = Number(env.ISMS_SOCIETY);
  if (!Number.isInteger(society)) throw new Error("ISMS_SOCIETY must be a society id");
  return {
    ismsUrl: (env.ISMS_URL ?? "http://127.0.0.1:8080").replace(/\/$/, ""),
    society,
    serverBin: env.ISMS_SERVER_BIN ?? "../target/debug/isms-server",
    databaseUrl: env.DATABASE_URL,
    anthropicKey: env.ANTHROPIC_API_KEY,
    openrouterKey: env.OPENROUTER_API_KEY,
  };
}
