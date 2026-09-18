// The done gate's replay: one player's recorded epoch, played again from the
// recording alone (no server, no network), must produce the same journal line
// for line. The recording holds every HTTP exchange the player made and the
// tick signals it was given; timings and the run's name are the only fields
// allowed to differ.

import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeClient } from "../src/api/client.ts";
import { readRecording, replayTransport } from "../src/api/transport.ts";
import { ScriptedBrain } from "../src/brain/scripted/index.ts";
import { Journal, readJournal, type Record as JournalRecord } from "../src/journal/journal.ts";
import { Notes } from "../src/player/notes.ts";
import { loadPersona } from "../src/player/persona.ts";
import { Player } from "../src/player/player.ts";
import type { TickSignal } from "../src/stream/ticks.ts";

const T = join(import.meta.dirname, "fixtures", "transcripts");
const PLAYER = "rule-prober-1";

function strip(r: JournalRecord): unknown {
  const { run: _run, ...rest } = r as JournalRecord & { run: string };
  if ("ms" in rest) (rest as { ms?: number }).ms = 0;
  if ("calls" in rest) (rest as { calls: { ms: number }[] }).calls = rest.calls.map((c) => ({ ...c, ms: 0 }));
  return rest;
}

describe("replaying a recorded epoch", () => {
  it("produces the recorded journal without the network", async () => {
    const recording = readRecording(join(T, `${PLAYER}.recording.jsonl`));
    const signals = readFileSync(join(T, `${PLAYER}.signals.jsonl`), "utf8")
      .split("\n")
      .filter((l) => l.trim() !== "")
      .map((l) => JSON.parse(l) as TickSignal);
    const expected = readJournal(join(T, `${PLAYER}.journal.jsonl`));
    const sid = Number(new URL(recording[0]!.url).pathname.split("/")[2]);
    const baseUrl = new URL(recording[0]!.url).origin;

    const transport = replayTransport(recording);
    const dir = mkdtempSync(join(tmpdir(), "replay-"));
    const persona = loadPersona(join(import.meta.dirname, "..", "personas", "rule-prober.md"));
    const player = new Player({
      run: "replay",
      name: PLAYER,
      persona,
      brain: new ScriptedBrain(persona, PLAYER),
      client: makeClient({ baseUrl, auth: { key: "isms_replay.key" }, transport }),
      sid,
      maxActions: 4,
      journal: new Journal(join(dir, "j.jsonl")),
      notes: new Notes(join(dir, "n.md")),
      now: () => 0,
    });
    for (const s of signals) {
      if (s.kind === "tick") await player.takeTurn(s.clock);
      else if (s.kind === "cycle") await player.reflect(s.clock);
    }
    const got = readJournal(join(dir, "j.jsonl"));
    expect(got.length).toBe(expected.length);
    for (let i = 0; i < expected.length; i++) expect(strip(got[i]!)).toEqual(strip(expected[i]!));
    expect(transport.remaining()).toBe(0);
  });
});
