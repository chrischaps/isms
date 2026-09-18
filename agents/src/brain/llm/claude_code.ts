// The daily reflection through the Claude Code CLI in headless mode
// (`claude -p`), which runs on the account's Claude subscription instead of
// API credit. Only the once-a-day rewrite of a player's notes goes this way:
// it is one prompt in, one JSON object out, no tools, and the strongest model
// is wanted for it. The hourly turns stay on the SDK, where the tool runner is.
//
// The CLI is given the same rules and persona as the SDK call would be, as its
// whole system prompt, and a JSON schema to answer in. Its usage comes back in
// tokens; the budget records them at no dollar cost.

import { execFile } from "node:child_process";
import type { RawUsage } from "../../budget/budget.ts";

export type ClaudeCodeResult = {
  structured: unknown;
  usage: RawUsage;
  /** The CLI's own idea of what the call would have cost at list price. */
  listUsd: number;
};

/** The arguments for one structured, tool-less, non-persisted call. */
export function claudeCodeArgs(model: string, system: string, schema: object): string[] {
  // The CLI validates the schema itself and rejects a `$schema` draft reference (zod emits one).
  const { $schema: _draft, ...plain } = schema as Record<string, unknown>;
  return [
    "-p",
    "--output-format",
    "json",
    "--json-schema",
    JSON.stringify(plain),
    "--model",
    model,
    "--system-prompt",
    system,
    "--tools",
    "",
    "--no-session-persistence",
  ];
}

/** The `--output-format json` document: the structured answer and the usage, or the error it reports. */
export function parseClaudeCodeResult(stdout: string): ClaudeCodeResult {
  let doc: Record<string, unknown>;
  try {
    doc = JSON.parse(stdout) as Record<string, unknown>;
  } catch {
    throw new Error(`claude -p did not answer with JSON: ${stdout.slice(0, 200)}`);
  }
  if (doc.is_error === true || doc.type !== "result") {
    throw new Error(`claude -p failed: ${String(doc.result ?? doc.error ?? stdout.slice(0, 200))}`);
  }
  const structured = doc.structured_output;
  if (structured === undefined || structured === null) {
    throw new Error(`claude -p returned no structured output: ${String(doc.result ?? "").slice(0, 200)}`);
  }
  const u = (doc.usage ?? {}) as Record<string, unknown>;
  const n = (v: unknown) => (typeof v === "number" ? v : 0);
  return {
    structured,
    usage: {
      input_tokens: n(u.input_tokens),
      output_tokens: n(u.output_tokens),
      cache_read_input_tokens: n(u.cache_read_input_tokens),
      cache_creation_input_tokens: n(u.cache_creation_input_tokens),
    },
    listUsd: n(doc.total_cost_usd),
  };
}

export type ClaudeCodeCall = {
  /** How many CLI processes may run at once; each is a whole Claude Code runtime, and five at once froze a 16 GB machine. */
  concurrency?: number;
  model: string;
  system: string;
  prompt: string;
  schema: object;
  /** The executable; `claude` on the PATH by default. */
  bin?: string;
  timeoutMs?: number;
};

let active = 0;
const waiting: (() => void)[] = [];

/** Wait for a slot under `limit`, and release it when `f` settles. */
async function withSlot<T>(limit: number, f: () => Promise<T>): Promise<T> {
  if (active >= limit) await new Promise<void>((r) => waiting.push(r));
  active += 1;
  try {
    return await f();
  } finally {
    active -= 1;
    waiting.shift()?.();
  }
}

/** One call: the prompt on stdin, the JSON document on stdout; at most `concurrency` at once. */
export function callClaudeCode(call: ClaudeCodeCall): Promise<ClaudeCodeResult> {
  return withSlot(call.concurrency ?? 1, () => spawnClaudeCode(call));
}

function spawnClaudeCode(call: ClaudeCodeCall): Promise<ClaudeCodeResult> {
  return new Promise((resolve, reject) => {
    const child = execFile(
      call.bin ?? "claude",
      claudeCodeArgs(call.model, call.system, call.schema),
      { timeout: call.timeoutMs ?? 300_000, maxBuffer: 16 * 1024 * 1024, windowsHide: true },
      (err, stdout, stderr) => {
        if (err && !stdout) {
          reject(new Error(`claude -p: ${err.message}${stderr ? `\n${stderr.slice(0, 500)}` : ""}`));
          return;
        }
        try {
          resolve(parseClaudeCodeResult(String(stdout)));
        } catch (e) {
          reject(e instanceof Error ? e : new Error(String(e)));
        }
      },
    );
    child.stdin?.end(call.prompt);
  });
}
