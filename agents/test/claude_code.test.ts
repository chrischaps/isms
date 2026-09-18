import { describe, expect, it } from "vitest";
import { Budget } from "../src/budget/budget.ts";
import { claudeCodeArgs, parseClaudeCodeResult } from "../src/brain/llm/claude_code.ts";

describe("claude -p as the reflection provider", () => {
  it("asks for one structured, tool-less, non-persisted answer", () => {
    const args = claudeCodeArgs("claude-opus-5", "rules and persona", { type: "object" });
    expect(args.slice(0, 3)).toEqual(["-p", "--output-format", "json"]);
    expect(args[args.indexOf("--json-schema") + 1]).toBe('{"type":"object"}');
    expect(args[args.indexOf("--model") + 1]).toBe("claude-opus-5");
    expect(args[args.indexOf("--system-prompt") + 1]).toBe("rules and persona");
    expect(args[args.indexOf("--tools") + 1]).toBe("");
    expect(args).toContain("--no-session-persistence");
  });

  it("reads the structured output and the usage from the result document", () => {
    const doc = {
      type: "result",
      is_error: false,
      result: '{"notes":"n","plan":"p"}',
      structured_output: { notes: "n", plan: "p" },
      total_cost_usd: 0.53,
      usage: { input_tokens: 4, output_tokens: 444, cache_read_input_tokens: 49130, cache_creation_input_tokens: 49365 },
    };
    const r = parseClaudeCodeResult(JSON.stringify(doc));
    expect(r.structured).toEqual({ notes: "n", plan: "p" });
    expect(r.usage).toEqual({ input_tokens: 4, output_tokens: 444, cache_read_input_tokens: 49130, cache_creation_input_tokens: 49365 });
    expect(r.listUsd).toBe(0.53);
  });

  it("turns a signed-out CLI or a non-JSON answer into an error", () => {
    const out = JSON.stringify({ type: "result", is_error: true, result: "Not logged in · Please run /login" });
    expect(() => parseClaudeCodeResult(out)).toThrow(/Not logged in/);
    expect(() => parseClaudeCodeResult("not json")).toThrow(/did not answer with JSON/);
    expect(() => parseClaudeCodeResult(JSON.stringify({ type: "result", result: "plain text" }))).toThrow(/no structured output/);
  });

  it("is charged in tokens but not in dollars", () => {
    const b = new Budget(1_000_000, 1);
    const u = b.charge("claude-opus-5", "landlord-1", { input_tokens: 4, output_tokens: 444, cache_read_input_tokens: 49130, cache_creation_input_tokens: 49365 }, false);
    expect(u.usd).toBe(0);
    expect(b.total.usd).toBe(0);
    expect(b.total.cache_read).toBe(49130);
    expect(Object.keys(b.summary().by_model)).toEqual(["claude-opus-5 via claude-code"]);
  });
});
