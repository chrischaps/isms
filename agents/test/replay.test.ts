// The done gate's replay: one player's recorded epoch, played again from the
// recording alone (no server, no network), must produce the same journal line
// for line. The recording holds every HTTP exchange the player made and the
// tick signals it was given; timings and the run's name are the only fields
// allowed to differ.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeClient } from "../src/api/client.ts";
import { readRecording, replayTransport, type Transport } from "../src/api/transport.ts";
import type { Brain, Persona } from "../src/brain/brain.ts";
import { JevBrain } from "../src/brain/jev/index.ts";
import { ScriptedBrain } from "../src/brain/scripted/index.ts";
import { Budget } from "../src/budget/budget.ts";
import { ConfigSchema } from "../src/config.ts";
import { Journal, readJournal, type Record as JournalRecord } from "../src/journal/journal.ts";
import { Notes } from "../src/player/notes.ts";
import { loadPersona } from "../src/player/persona.ts";
import { Player } from "../src/player/player.ts";
import type { TickSignal } from "../src/stream/ticks.ts";

const T = join(import.meta.dirname, "fixtures", "transcripts");

/** The recorded players: a scripted one (S1.16) and a Jev one (SJ.1), whose recording holds the decision calls too. */
const PLAYERS: { player: string; persona: string; brain: (p: Persona, name: string, transport: Transport) => Brain }[] = [
  { player: "rule-prober-1", persona: "rule-prober", brain: (p, name) => new ScriptedBrain(p, name) },
  {
    player: "founder-1",
    persona: "founder",
    brain: (p, name, transport) =>
      new JevBrain({ persona: p, player: name, handle: name, cfg: ConfigSchema.parse({}), budget: new Budget(1_000_000, 1), key: "isms_replay.key", transport }),
  },
];

function strip(r: JournalRecord): unknown {
  const { run: _run, ...rest } = r as JournalRecord & { run: string };
  if ("ms" in rest) (rest as { ms?: number }).ms = 0;
  if ("calls" in rest) (rest as { calls: { ms: number }[] }).calls = rest.calls.map((c) => ({ ...c, ms: 0 }));
  return rest;
}

/** A regenerated record with the recorded run label and timings kept, so the fixture's diff is only the new fields. */
function keepTiming(r: JournalRecord, was: JournalRecord | undefined): JournalRecord {
  if (!was) return r;
  const out = { ...r, run: was.run } as JournalRecord & { ms?: number; calls?: { ms: number }[] };
  const before = was as JournalRecord & { ms?: number; calls?: { ms: number }[] };
  if (out.ms !== undefined && before.ms !== undefined) out.ms = before.ms;
  if (out.calls && before.calls) out.calls = out.calls.map((c, i) => ({ ...c, ms: before.calls![i]?.ms ?? c.ms }));
  return out;
}

describe.each(PLAYERS)("replaying $player's recorded epoch", ({ player: PLAYER, persona: slug, brain }) => {
  it("produces the recorded journal without the network", async () => {
    const recording = readRecording(join(T, `${PLAYER}.recording.jsonl`));
    const signals = readFileSync(join(T, `${PLAYER}.signals.jsonl`), "utf8")
      .split("\n")
      .filter((l) => l.trim() !== "")
      .map((l) => JSON.parse(l) as TickSignal);
    const expected = readJournal(join(T, `${PLAYER}.journal.jsonl`));
    const game = recording.find((x) => x.target === "game")!;
    const sid = Number(new URL(game.url).pathname.split("/")[2]);
    const baseUrl = new URL(game.url).origin;

    const transport = replayTransport(recording);
    const dir = mkdtempSync(join(tmpdir(), "replay-"));
    const persona = loadPersona(join(import.meta.dirname, "..", "personas", `${slug}.md`));
    const player = new Player({
      run: "replay",
      name: PLAYER,
      persona,
      brain: brain(persona, PLAYER, transport),
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
    // `UPDATE_TRANSCRIPTS=1 pnpm test replay` rewrites the expected journal from the same recording, as the engine's goldens do,
    // when a journal field is added (E-6's `offered`); a change in what the brain sends or does still needs a re-recording.
    if (process.env.UPDATE_TRANSCRIPTS) writeFileSync(join(T, `${PLAYER}.journal.jsonl`), got.map((r, i) => JSON.stringify(keepTiming(r, expected[i]))).join("\n") + "\n");
    expect(got.length).toBe(expected.length);
    for (let i = 0; i < expected.length; i++) expect(strip(got[i]!)).toEqual(strip(expected[i]!));
    expect(transport.remaining()).toBe(0);
  });
});
