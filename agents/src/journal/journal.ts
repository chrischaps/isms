// One line per turn, per player: what it saw, what it meant to do, every call
// with its result, every rejection verbatim, and what it did not understand.
// Both brains write the same shape, so the report reads them alike.

import { appendFileSync, mkdirSync, readFileSync } from "node:fs";
import { dirname } from "node:path";
import { z } from "zod";

export const CallRecordSchema = z.object({
  tool: z.string(),
  input: z.unknown(),
  ok: z.boolean(),
  status: z.number().optional(),
  code: z.string().optional(),
  detail: z.string().optional(),
  ms: z.number(),
});

export const UsageSchema = z.object({
  input: z.number(),
  output: z.number(),
  cache_read: z.number(),
  cache_write: z.number(),
  usd: z.number(),
});
export type Usage = z.infer<typeof UsageSchema>;

export const ZERO_USAGE: Usage = { input: 0, output: 0, cache_read: 0, cache_write: 0, usd: 0 };

/** A small extract of `/home`: enough for the arc, never the whole view. */
export const SituationSchema = z.object({
  balance: z.number(),
  food: z.number(),
  shelter: z.number(),
  comfort: z.number(),
  housed: z.boolean(),
  jobs: z.number(),
  pantry_food: z.number(),
});
export type Situation = z.infer<typeof SituationSchema>;

export const TurnRecordSchema = z.object({
  kind: z.literal("turn"),
  run: z.string(),
  player: z.string(),
  persona: z.string(),
  brain: z.enum(["scripted", "llm"]),
  model: z.string().nullable(),
  epoch: z.number(),
  cycle: z.number(),
  tick: z.number(),
  ms: z.number(),
  situation: SituationSchema,
  intent: z.string(),
  calls: z.array(CallRecordSchema),
  rejections: z.array(z.object({ tool: z.string(), code: z.string(), detail: z.string() })),
  did_not_understand: z.array(z.string()),
  ended_by: z.enum(["end_turn", "text", "cap", "budget", "error", "script"]),
  error: z.string().nullable(),
  usage: UsageSchema,
});
export type TurnRecord = z.infer<typeof TurnRecordSchema>;

export const ReflectionRecordSchema = z.object({
  kind: z.literal("reflection"),
  run: z.string(),
  player: z.string(),
  persona: z.string(),
  model: z.string(),
  epoch: z.number(),
  cycle: z.number(),
  ms: z.number(),
  notes_before: z.string(),
  notes_after: z.string(),
  plan: z.string(),
  usage: UsageSchema,
});
export type ReflectionRecord = z.infer<typeof ReflectionRecordSchema>;

export const SkipRecordSchema = z.object({
  kind: z.literal("skip"),
  run: z.string(),
  player: z.string(),
  epoch: z.number(),
  cycle: z.number(),
  tick: z.number(),
  reason: z.enum(["overrun", "budget"]),
});
export type SkipRecord = z.infer<typeof SkipRecordSchema>;

export const RecordSchema = z.discriminatedUnion("kind", [TurnRecordSchema, ReflectionRecordSchema, SkipRecordSchema]);
export type Record = z.infer<typeof RecordSchema>;

export class Journal {
  private readonly file: string;
  constructor(file: string) {
    this.file = file;
    mkdirSync(dirname(file), { recursive: true });
  }
  append(r: Record) {
    appendFileSync(this.file, JSON.stringify(RecordSchema.parse(r)) + "\n");
  }
}

export function readJournal(file: string): Record[] {
  return readFileSync(file, "utf8")
    .split("\n")
    .filter((l) => l.trim() !== "")
    .map((l) => RecordSchema.parse(JSON.parse(l)));
}
