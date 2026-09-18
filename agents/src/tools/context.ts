// What a tool is, to both brains: a name, a description the model reads, a zod
// schema, and a `run` against the public API. Every call passes through
// `invoke`, which validates the input, counts actions against the turn's cap,
// and records the call for the journal. A refusal from the server is a normal
// result here (`ok: false` with the problem document verbatim), never a throw:
// the refusal is what the player is meant to read.

import type { z } from "zod";
import { ApiError, type Client } from "../api/client.ts";

export type ToolKind = "read" | "act" | "end";

export type Refusal = {
  ok: false;
  status: number;
  code: string;
  title: string;
  detail: string;
};
export type ToolResult = { ok: true; data: unknown } | Refusal;

export type ToolDef<I = unknown> = {
  name: string;
  description: string;
  kind: ToolKind;
  schema: z.ZodType<I>;
  run: (input: I, ctx: ToolContext) => Promise<ToolResult>;
};

/** A tool of any input type, for lists and dispatch. */
// oxlint-disable-next-line no-explicit-any
export type AnyTool = ToolDef<any>;

export type CallRecord = {
  tool: string;
  input: unknown;
  ok: boolean;
  status?: number;
  code?: string;
  detail?: string;
  ms: number;
};

export type EndTurn = { intent: string; did_not_understand: string[] };

export type TurnState = {
  actionsUsed: number;
  maxActions: number;
  calls: CallRecord[];
  ended: EndTurn | null;
};

export type ToolContext = {
  client: Client;
  sid: number;
  turn: TurnState;
  /** Injected in tests; defaults to `Date.now`. */
  now?: () => number;
};

export function newTurn(maxActions: number): TurnState {
  return { actionsUsed: 0, maxActions, calls: [], ended: null };
}

export const HARNESS_ACTION_CAP = "HARNESS_ACTION_CAP";
export const HARNESS_BAD_INPUT = "HARNESS_BAD_INPUT";
export const HARNESS_TURN_OVER = "HARNESS_TURN_OVER";

export function refusal(status: number, code: string, title: string, detail: string): Refusal {
  return { ok: false, status, code, title, detail };
}

/** An ApiError as the player reads it: the server's words, untouched. */
export function refusalOf(e: ApiError): Refusal {
  return refusal(e.status, e.code, e.problem?.title ?? `HTTP ${e.status}`, e.problem?.detail ?? "");
}

/** Run a tool the way both brains must: validated, capped, recorded. */
export async function invoke(tool: AnyTool, rawInput: unknown, ctx: ToolContext): Promise<ToolResult> {
  const now = ctx.now ?? Date.now;
  const started = now();
  const parsed = tool.schema.safeParse(rawInput);
  const record = (r: ToolResult): ToolResult => {
    ctx.turn.calls.push({
      tool: tool.name,
      input: rawInput,
      ok: r.ok,
      ...(r.ok ? {} : { status: r.status, code: r.code, detail: r.detail }),
      ms: now() - started,
    });
    return r;
  };
  if (!parsed.success) {
    return record(refusal(400, HARNESS_BAD_INPUT, "bad tool input", parsed.error.issues.map((i) => `${i.path.join(".")}: ${i.message}`).join("; ")));
  }
  if (ctx.turn.ended) {
    return record(refusal(409, HARNESS_TURN_OVER, "the turn is over", "end_turn was already called; nothing more happens until the next tick"));
  }
  if (tool.kind === "act") {
    if (ctx.turn.actionsUsed >= ctx.turn.maxActions) {
      return record(
        refusal(429, HARNESS_ACTION_CAP, "no actions left this turn", `you may take at most ${ctx.turn.maxActions} actions per turn; read, then end_turn`),
      );
    }
    ctx.turn.actionsUsed += 1;
  }
  try {
    return record(await tool.run(parsed.data, ctx));
  } catch (e) {
    if (e instanceof ApiError) {
      if (e.status === 429) {
        // The society's per-citizen bucket refilled within a second; one retry is honest, more would hide a real limit.
        await new Promise((r) => setTimeout(r, 1000));
        try {
          return record(await tool.run(parsed.data, ctx));
        } catch (e2) {
          if (e2 instanceof ApiError) return record(refusalOf(e2));
          throw e2;
        }
      }
      return record(refusalOf(e));
    }
    throw e;
  }
}

/** Rows a read tool returns at most; the model is told when a list was cut. */
export const ROW_CAP = 40;

export function capRows<T>(rows: T[], cap = ROW_CAP): { rows: T[]; note?: string } {
  if (rows.length <= cap) return { rows };
  return { rows: rows.slice(-cap), note: `showing the last ${cap} of ${rows.length}` };
}
