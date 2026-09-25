import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { parsePersona, personaText } from "../src/player/persona.ts";
import { personasFor } from "../src/player/cast.ts";
import { loadConfig } from "../src/config.ts";
import { strategyFor } from "../src/brain/scripted/index.ts";
import { householderLike } from "../src/brain/scripted/strategies.ts";
import { ruleProber } from "../src/brain/scripted/fuzz.ts";

const DIR = join(import.meta.dirname, "..", "personas");

describe("personas", () => {
  const files = readdirSync(DIR).filter((f) => f.endsWith(".md"));
  it("are the eight the S1.16 card names, the four the S2.10 card adds, and SJ.3's lender and builder", () => {
    expect(files.map((f) => f.replace(/\.md$/, "")).sort()).toEqual(
      ["borrower", "builder", "chronicler", "founder", "free-rider", "landlord", "lender", "rationer", "rule-prober", "saver", "slacker", "speculator", "steward", "wage-maximiser"],
    );
  });
  it("loads the sixteen-player town by name, cycled to the players (SJ.3)", () => {
    const cfg = loadConfig(join(DIR, "..", "config.toml"), { players: 16 });
    const town = personasFor(cfg, "freeport", "freeport-town");
    expect(town).toHaveLength(16);
    const count = (slug: string) => town.filter((p) => p.slug === slug).length;
    expect(count("founder")).toBe(2);
    expect(count("lender")).toBe(2);
    expect(count("builder")).toBe(1);
    expect(count("landlord")).toBe(1);
    expect(count("rule-prober")).toBe(1);
    expect(new Set(town.map((p) => p.slug)).size).toBe(10);
    expect(personasFor(cfg, "freeport").map((p) => p.slug).slice(0, 8)).toEqual(cfg.run.personas);
    expect(() => personasFor(cfg, "freeport", "nowhere")).toThrow(/no cast named nowhere/);
  });
  for (const f of files) {
    it(`${f} parses, names its slug, and reads as prose`, () => {
      const p = parsePersona(readFileSync(join(DIR, f), "utf8"));
      expect(p.slug).toBe(f.replace(/\.md$/, ""));
      expect(p.goals.length).toBeGreaterThanOrEqual(2);
      expect(p.body.length).toBeGreaterThan(100);
      const text = personaText(p);
      expect(text).toContain(`You are ${p.name}.`);
      expect(text).toContain("1. ");
    });
  }
  it("gives the rule-prober the fuzzer and unknown slugs the householder floor", () => {
    expect(strategyFor("rule-prober")).toBe(ruleProber);
    expect(strategyFor("nobody")).toBe(householderLike);
    expect(strategyFor("founder")).not.toBe(householderLike);
  });
  it("rejects a persona without a brain", () => {
    expect(() => parsePersona("---\nname: x\nslug: x\ngoals: [a]\ntemperament: t\nrisk: low\n---\nbody")).toThrow();
  });
});
