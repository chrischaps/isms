import { describe, expect, it } from "vitest";
import { Budget, priceOf, tokensOf } from "../src/budget/budget.ts";

describe("priceOf", () => {
  it("prices cache reads at a tenth and writes at 1.25x of input", () => {
    const u = priceOf("claude-sonnet-5", { input_tokens: 1_000_000, output_tokens: 0, cache_read_input_tokens: 1_000_000, cache_creation_input_tokens: 1_000_000 });
    expect(u.usd).toBeCloseTo(2 + 0.2 + 2.5, 6);
    expect(tokensOf(u)).toBe(3_000_000);
  });
  it("refuses a model it has no price for", () => {
    expect(() => priceOf("claude-3", { input_tokens: 1, output_tokens: 1 })).toThrow(/no price/);
  });
});

describe("Budget", () => {
  it("accumulates by model and by player and stops at the cap", () => {
    const b = new Budget(10_000, 0.05);
    b.charge("claude-haiku-4-5", "a", { input_tokens: 1000, output_tokens: 1000 });
    expect(b.exhausted()).toBe(false);
    b.charge("claude-opus-5", "b", { input_tokens: 1000, output_tokens: 1000 });
    // 1000*5 + 1000*25 = $0.03 plus haiku $0.006: still under $0.05, under 10k tokens
    expect(b.exhausted()).toBe(false);
    b.charge("claude-opus-5", "b", { input_tokens: 0, output_tokens: 1000 });
    expect(b.total.usd).toBeCloseTo(0.006 + 0.03 + 0.025, 6);
    expect(b.exhausted()).toBe(true);
    const s = b.summary();
    expect(Object.keys(s.by_model).sort()).toEqual(["claude-haiku-4-5", "claude-opus-5"]);
    expect(s.by_player.b!.output).toBe(2000);
  });
  it("stops on tokens too", () => {
    const b = new Budget(100, 1000);
    b.charge("claude-haiku-4-5", "a", { input_tokens: 60, output_tokens: 50 });
    expect(b.exhausted()).toBe(true);
  });
});
