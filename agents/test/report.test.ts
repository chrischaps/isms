import { copyFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { buildReport, economyTable, endpointOf, fold, normalise, standingsTable } from "../src/report/report.ts";

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

describe("the economy day by day and the standings (SJ.2)", () => {
  it("draws one row per day from the /stats snapshots", () => {
    const day = (cycle: number, wage: number, gini: number) => ({
      cycle,
      live: { food_last_price: 131 },
      aggregates: { mean_cycle_wage: wage, price_index: 1.1, unemployed: 2, firm_count: 20, credit_outstanding: 5000, real_output: 1335, materials_produced: 710, materials_to_machines: 36, investment_share: 0.05, consumption_gini: gini, need_fulfillment_rate: 0.975, hardship_count: 0, median_wellbeing: 89.8 },
    });
    const lines = economyTable([day(1, 61.7, 0.04), day(2, 63.2, 0.05)]);
    expect(lines[0]).toBe("## The economy, day by day");
    expect(lines).toContain("| 1 | 61.70 | 1.10 | 1.31 | 2 | 20 | 50.00 | 1335 | 710 | 36 | 5% | 0.040 | 98% | 0 | 89.8 |");
    expect(lines.filter((l) => /^\| \d+ \|/.test(l))).toHaveLength(2);
    expect(economyTable([])).toEqual([]);
  });
  it("prints each day's headlines under the table (SJ.3)", () => {
    const day = (cycle: number, headlines?: string[]) => ({ cycle, live: null, aggregates: null, ...(headlines ? { headlines } : {}) });
    const lines = economyTable([day(1, []), day(2, ["Legacy Foundry No. 1 runs short of ore.", "H-3 | leaves the mine"]), day(3)]);
    expect(lines).toContain("### The days' headlines");
    expect(lines).toContain("**Day 2** (2)");
    expect(lines).toContain("- Legacy Foundry No. 1 runs short of ore.");
    expect(lines).toContain("- H-3 \\| leaves the mine");
    expect(lines.some((l) => l.startsWith("**Day 1**") || l.startsWith("**Day 3**"))).toBe(false);
    expect(economyTable([day(1), day(2)])).not.toContain("### The days' headlines");
  });
  it("marks the cohort's players on the scoreboard and keeps every player's rank", () => {
    const rows = Array.from({ length: 14 }, (_, i) => ({ citizen: i, handle: `h${i}`, net_worth: 100000 - i * 1000, self_made: -i * 1000, firms: [] }));
    rows[12]!.handle = "founder-1";
    const lines = standingsTable({ rows }, new Set(["founder-1", "h0"]));
    expect(lines).toContain("| 1 | **h0** | 1000.00 | 0.00 | 0 |");
    expect(lines).toContain("| 13 | **founder-1** | 880.00 | -120.00 | 0 |");
    expect(lines.filter((l) => l.startsWith("| …"))).toHaveLength(1);
    expect(lines.filter((l) => /^\| \d+ \|/.test(l))).toHaveLength(11);
    expect(standingsTable(null, new Set())).toEqual([]);
  });
});
