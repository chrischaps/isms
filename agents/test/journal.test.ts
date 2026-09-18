import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Journal, readJournal, ZERO_USAGE, type TurnRecord } from "../src/journal/journal.ts";

const turn: TurnRecord = {
  kind: "turn",
  run: "t",
  player: "p",
  persona: "saver",
  brain: "scripted",
  model: null,
  epoch: 1,
  cycle: 1,
  tick: 1,
  ms: 3,
  situation: { balance: 100, food: 50, shelter: 50, comfort: 50, housed: false, jobs: 0, pantry_food: 0 },
  intent: "wait",
  calls: [{ tool: "home", input: {}, ok: true, ms: 1 }],
  rejections: [],
  did_not_understand: [],
  ended_by: "script",
  error: null,
  usage: ZERO_USAGE,
};

describe("Journal", () => {
  it("round-trips turns, reflections and skips", () => {
    const dir = mkdtempSync(join(tmpdir(), "j-"));
    const j = new Journal(join(dir, "a.jsonl"));
    j.append(turn);
    j.append({ kind: "skip", run: "t", player: "p", epoch: 1, cycle: 1, tick: 2, reason: "overrun" });
    j.append({ kind: "reflection", run: "t", player: "p", persona: "saver", model: "m", epoch: 1, cycle: 1, ms: 1, notes_before: "", notes_after: "n", plan: "1. x", usage: ZERO_USAGE });
    const back = readJournal(join(dir, "a.jsonl"));
    expect(back.map((r) => r.kind)).toEqual(["turn", "skip", "reflection"]);
    expect(back[0]).toEqual(turn);
  });
  it("refuses a record that is not a record", () => {
    const dir = mkdtempSync(join(tmpdir(), "j-"));
    const j = new Journal(join(dir, "b.jsonl"));
    expect(() => j.append({ ...turn, ended_by: "whatever" } as unknown as TurnRecord)).toThrow();
  });
});
