// SB.2 done gate: the Home verdict names the state in the player's words,
// leads with the good, and has one "but" at most — the worst problem by the
// engine's thresholds; the rest stay in their cards (docs/style.md §8.2).

import { describe, expect, it } from "vitest";
import { verdictText } from "../components/Verdict";
import { needStatus } from "./needs";
import { greeting, homeVerdict, needTone } from "./verdict";

const fine = { food: 100, shelter: 100, comfort: 94, hardship: false, housed: true, hours: 8, hired: true };
const t = (k: string) => k;

describe("homeVerdict", () => {
  it("reads as the bible's example when only a dwelling is missing", () => {
    const parts = homeVerdict({ ...fine, housed: false, shelter: 94 });
    expect(verdictText(parts)).toBe("You're well fed and working today, but you don't have a place to live yet.");
    expect(parts).toContainEqual({ text: "don't have a place to live", tone: "attn" });
    expect(parts).toContainEqual({ text: "well fed", tone: "good" });
  });

  it("says nothing needs you when nothing does", () => {
    expect(verdictText(homeVerdict(fine))).toBe("You're well fed, housed and working today. Nothing needs you right now.");
  });

  it("names hardship first, in crit, and keeps one but", () => {
    const parts = homeVerdict({ ...fine, food: 12, hardship: true, housed: false, hours: 0 });
    const text = verdictText(parts);
    expect(text).toBe("You're in hardship — Food has been under the line for a whole day. Eat first.");
    expect(text.split("but").length).toBe(1);
    expect(parts).toContainEqual({ text: "in hardship", tone: "crit" });
  });

  it("ranks low food above a missing dwelling and a missing job, with no good to lead", () => {
    expect(verdictText(homeVerdict({ ...fine, food: 40, housed: false, hours: 0 }))).toBe("You're getting hungry.");
    expect(verdictText(homeVerdict({ ...fine, food: 40 }))).toBe("You're housed and working today, but you're getting hungry.");
  });

  it("tells a hired citizen to set hours and an unhired one to find work", () => {
    expect(verdictText(homeVerdict({ ...fine, hours: 0, hired: true }))).toBe("You're well fed and housed, but you haven't set your hours yet.");
    expect(verdictText(homeVerdict({ ...fine, hours: 0, hired: false }))).toBe("You're well fed and housed, but you don't have work yet.");
  });

  it("is crit under the line and attn under 50", () => {
    expect(needTone(19)).toBe("crit");
    expect(needTone(20)).toBe("attn");
    expect(needTone(50)).toBe("good");
    expect(needTone(24, 25)).toBe("crit");
  });
});

describe("greeting", () => {
  it("follows the society's hour, not the wall clock", () => {
    expect(greeting("chaps", { tick: 6, ticks_per_cycle: 24 })).toBe("Good morning, chaps.");
    expect(greeting("chaps", { tick: 14, ticks_per_cycle: 24 })).toBe("Good afternoon, chaps.");
    expect(greeting("chaps", { tick: 20, ticks_per_cycle: 24 })).toBe("Good evening, chaps.");
    expect(greeting("chaps", { tick: 3, ticks_per_cycle: 24 })).toBe("Hello, chaps.");
    expect(greeting("chaps", { tick: 3, ticks_per_cycle: 12 })).toBe("Hello, chaps.");
  });
});

describe("needStatus", () => {
  const base = { food: 100, shelter: 94, comfort: 94, hardship: false, housed: false, pantryFood: 22, pantryWares: 0 };
  it("pairs every bar with a sentence and lifts unhoused shelter to attn", () => {
    const s = needStatus(t, base);
    expect(s.food).toBe("Full. You eat 1 Food an hour from your pantry.");
    expect(s.shelter).toBe("Falling every hour — you have no dwelling.");
    expect(s.comfort).toBe("Drifting down slowly. Wares would top it up.");
    expect(s.shelterTone).toBe("attn");
  });
  it("leaves the thresholds to speak once the value is under 50", () => {
    expect(needStatus(t, { ...base, shelter: 30 }).shelterTone).toBeUndefined();
    expect(needStatus(t, { ...base, housed: true }).shelterTone).toBeUndefined();
    expect(needStatus(t, { ...base, housed: true }).shelter).toBe("Rising — you have a dwelling.");
  });
  it("says hardship on the Food line", () => {
    expect(needStatus(t, { ...base, food: 10, hardship: true }).food).toMatch(/^In hardship/);
    expect(needStatus(t, { ...base, food: 10, pantryFood: 0 }).food).toBe("Under the hardship line and your pantry is empty.");
  });
});
