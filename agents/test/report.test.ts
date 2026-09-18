import { copyFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { buildReport, endpointOf, fold, normalise } from "../src/report/report.ts";

const T = join(import.meta.dirname, "fixtures", "transcripts");

describe("normalise and endpointOf", () => {
  it("blanks ids and numbers so one bug is one row", () => {
    expect(normalise("c41 already works at w19")).toBe("c# already works at w#");
    expect(normalise("Citizen(c47) has 965.94")).toBe("Citizen(c#) has #");
    expect(normalise("no offer 999999")).toBe("no offer #");
  });
  it("files a confusion under the endpoint it names", () => {
    expect(endpointOf("I do not know what a piece rate on a job offer means")).toBe("offers and contracts");
    expect(endpointOf("the bid was refused with 'escrow' but I had the money")).toBe("market");
    expect(endpointOf("why did my lease end")).toBe("housing");
    expect(endpointOf("???")).toBe("general");
  });
});

describe("buildReport", () => {
  it("folds a run into the sections the card asks for", () => {
    const dir = mkdtempSync(join(tmpdir(), "run-"));
    copyFileSync(join(T, "rule-prober-1.journal.jsonl"), join(dir, "rule-prober-1.journal.jsonl"));
    writeFileSync(
      join(dir, "summary.json"),
      JSON.stringify({
        run: "t",
        stopped_by: "epoch_end",
        turns: 20,
        skipped: 0,
        ticks_seen: 20,
        cycles_seen: 1,
        players: [{ name: "rule-prober-1", persona: "rule-prober", brain: "scripted", model: null, skipped: 0 }],
        budget: { total: { input: 0, output: 0, cache_read: 0, cache_write: 0, usd: 0 }, tokens: 0, max_tokens: 1, max_usd: 1, by_model: {}, by_player: {} },
      }),
    );
    writeFileSync(join(dir, "stats.json"), JSON.stringify({ clock: { epoch: 1, cycle: 2, tick: 1 }, live: { population: 48, active_humans: 8, unemployed: 0 }, firm_count: 20, credit_outstanding: 0, last_cycle: null }));
    const md = buildReport(fold(dir, "t"));
    for (const h of ["## Run", "## The economy at the end", "## Rejections", "## Fuzzer findings", "## What players did not understand", "## Each player's arc", "## Harness notes"]) {
      expect(md).toContain(h);
    }
    expect(md).toMatch(/### [a-z_]+ \(\d+\)/);
    expect(md).toContain("Refusals that name things by raw id");
    expect(md).toContain("**rule-prober-1** (rule-prober, scripted) took 20 turns");
  });
});
