// What the run has spent, by model and by player, and when it must stop.
// Scripted turns cost nothing; every LLM message is charged from its `usage`.

import { MODELS } from "../config.ts";
import { ZERO_USAGE, type Usage } from "../journal/journal.ts";

export type RawUsage = {
  input_tokens: number;
  output_tokens: number;
  cache_read_input_tokens?: number | null;
  cache_creation_input_tokens?: number | null;
};

export function priceOf(model: string, u: RawUsage): Usage {
  const m = MODELS[model];
  if (!m) throw new Error(`no price for model ${model}`);
  const cacheRead = u.cache_read_input_tokens ?? 0;
  const cacheWrite = u.cache_creation_input_tokens ?? 0;
  const usd =
    (u.input_tokens * m.input + u.output_tokens * m.output + cacheRead * m.cacheRead + cacheWrite * m.cacheWrite) / 1_000_000;
  return { input: u.input_tokens, output: u.output_tokens, cache_read: cacheRead, cache_write: cacheWrite, usd };
}

export function addUsage(a: Usage, b: Usage): Usage {
  return {
    input: a.input + b.input,
    output: a.output + b.output,
    cache_read: a.cache_read + b.cache_read,
    cache_write: a.cache_write + b.cache_write,
    usd: a.usd + b.usd,
  };
}

export function tokensOf(u: Usage): number {
  return u.input + u.output + u.cache_read + u.cache_write;
}

export class Budget {
  readonly maxTokens: number;
  readonly maxUsd: number;
  total: Usage = { ...ZERO_USAGE };
  byModel = new Map<string, Usage>();
  byPlayer = new Map<string, Usage>();
  constructor(maxTokens: number, maxUsd: number) {
    this.maxTokens = maxTokens;
    this.maxUsd = maxUsd;
  }
  /** `priced: false` records the tokens at no dollar cost: a call that ran on the subscription (Claude Code). */
  charge(model: string, player: string, raw: RawUsage, priced = true): Usage {
    const u = priced ? priceOf(model, raw) : { ...priceOf(model, raw), usd: 0 };
    const key = priced ? model : `${model} via claude-code`;
    this.total = addUsage(this.total, u);
    this.byModel.set(key, addUsage(this.byModel.get(key) ?? ZERO_USAGE, u));
    this.byPlayer.set(player, addUsage(this.byPlayer.get(player) ?? ZERO_USAGE, u));
    return u;
  }
  exhausted(): boolean {
    return tokensOf(this.total) >= this.maxTokens || this.total.usd >= this.maxUsd;
  }
  summary() {
    return {
      total: this.total,
      tokens: tokensOf(this.total),
      max_tokens: this.maxTokens,
      max_usd: this.maxUsd,
      by_model: Object.fromEntries(this.byModel),
      by_player: Object.fromEntries(this.byPlayer),
    };
  }
}
