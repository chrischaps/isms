import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { parsePersona, personaText } from "../src/player/persona.ts";
import { strategyFor } from "../src/brain/scripted/index.ts";
import { householderLike } from "../src/brain/scripted/strategies.ts";
import { ruleProber } from "../src/brain/scripted/fuzz.ts";

const DIR = join(import.meta.dirname, "..", "personas");

describe("personas", () => {
  const files = readdirSync(DIR).filter((f) => f.endsWith(".md"));
  it("are the eight the card names", () => {
    expect(files.map((f) => f.replace(/\.md$/, "")).sort()).toEqual(
      ["borrower", "founder", "landlord", "rule-prober", "saver", "slacker", "speculator", "wage-maximiser"],
    );
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
