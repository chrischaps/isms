// SB.2 done gate: the Home verdict names the state in the player's words,
// leads with the good, and has one "but" at most — the worst problem by the
// engine's thresholds; the rest stay in their cards (docs/style.md §8.2).

import { describe, expect, it } from "vitest";
import { verdictText } from "../components/Verdict";
import { needStatus } from "./needs";
import { greeting, homeVerdict, marketVerdict, needTone, planVerdict, workVerdict } from "./verdict";

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

// SB.3 done gate: the three verdicts, each from the player's side of the screen.
describe("workVerdict", () => {
  const farm = { name: "Legacy Farm No. 1", hours: 8, effort: "normal" };
  it("reads as the bible's example", () => {
    const parts = workVerdict({ hours: 8, budget: 8, fatigue: 0, positions: [farm], byNorm: false });
    expect(verdictText(parts)).toBe("You're working 8 of 8 hours at Legacy Farm No. 1 at normal effort.");
    expect(parts).toContainEqual({ text: "working 8 of 8 hours", tone: "good" });
  });
  it("names every workplace with hours and calls their efforts mixed", () => {
    const mine = { name: "Iron & Sons", hours: 2, effort: "high" };
    expect(verdictText(workVerdict({ hours: 8, budget: 8, fatigue: 0, positions: [{ ...farm, hours: 6 }, mine], byNorm: false }))).toBe(
      "You're working 8 of 8 hours at Legacy Farm No. 1 and Iron & Sons at mixed effort.",
    );
  });
  it("makes fatigue the one but", () => {
    const parts = workVerdict({ hours: 6, budget: 6, fatigue: 2, positions: [{ ...farm, hours: 6 }], byNorm: false });
    expect(verdictText(parts)).toBe("You're working 6 of 6 hours at Legacy Farm No. 1 at normal effort, but fatigue has taken 2 hours off today's budget.");
    expect(parts).toContainEqual({ text: "fatigue", tone: "attn" });
  });
  it("tells a hired citizen to set hours and an unhired one where the work is", () => {
    expect(verdictText(workVerdict({ hours: 0, budget: 8, fatigue: 0, positions: [{ ...farm, hours: 0 }], byNorm: false }))).toBe(
      "You hold a position at Legacy Farm No. 1, but you haven't set your hours yet.",
    );
    expect(verdictText(workVerdict({ hours: 0, budget: 8, fatigue: 0, positions: [], byNorm: false }))).toBe(
      "You don't have work yet — the job board is on the Organizations screen.",
    );
    expect(verdictText(workVerdict({ hours: 0, budget: 8, fatigue: 0, positions: [], byNorm: true }))).toBe("You hold no position yet — take one below.");
  });
});

describe("planVerdict", () => {
  const freeport = { keepFood: 24, keepBalance: 0, wares: false, orders: 0, money: true, store: false, labor: "explicit" as const, vote: null };
  it("reads as the bible's example", () => {
    expect(verdictText(planVerdict(freeport))).toBe("Your plan keeps you fed and spends everything else. It runs every hour, here or not.");
  });
  it("lists every rule that is set", () => {
    const parts = planVerdict({ ...freeport, keepBalance: 1250, wares: true, orders: 2, vote: "follow", followHandle: "ada" });
    expect(verdictText(parts)).toBe(
      "Your plan keeps you fed, keeps 12.50 cr in hand, buys Wares when Comfort dips, places 2 standing orders and votes with ada. It runs every hour, here or not.",
    );
    expect(parts).toContainEqual({ text: "keeps 12.50 cr in hand", tone: "good" });
  });
  it("warns when there is no Food floor, in the Store's words where there is one", () => {
    const parts = planVerdict({ ...freeport, keepFood: 0, keepBalance: 500 });
    expect(verdictText(parts)).toBe("Your plan keeps 5.00 cr in hand, but it won't buy Food for you — set a pantry floor.");
    expect(parts).toContainEqual({ text: "won't buy Food", tone: "attn" });
    expect(verdictText(planVerdict({ ...freeport, keepFood: 0, money: false, store: true }))).toBe("Your plan does nothing on its own and won't draw Food for you — set a pantry floor.");
  });
  it("speaks the Commune's rules without money", () => {
    expect(verdictText(planVerdict({ ...freeport, money: false, store: true, wares: true, labor: "follow_norm", vote: "abstain" }))).toBe(
      "Your plan keeps you fed, draws Wares when Comfort dips, works the norm and abstains for you. It runs every hour, here or not.",
    );
  });
});

describe("marketVerdict", () => {
  const food = { name: "food", isShare: false, last: 131, change: 0.2, days: 3, soldOut: false, noAsks: false, food: { keepFood: 24, pantryFood: 22 }, openOrders: 0 };
  it("reads as the bible's example", () => {
    const parts = marketVerdict(food);
    expect(verdictText(parts)).toBe("Food costs 1.31 cr and hasn't moved in 3 days. Your plan will bid for 2 food next hour.");
    expect(parts).toContainEqual({ text: "Food costs 1.31 cr", tone: "ink" });
  });
  it("says which way the price went and what the plan holds", () => {
    expect(verdictText(marketVerdict({ ...food, change: -4.25, food: { keepFood: 24, pantryFood: 30 }, openOrders: 1 }))).toBe(
      "Food costs 1.31 cr and is down 4.3 % over 3 days. Your plan keeps Food at 24 and has no shortfall to bid for. You have 1 open order here.",
    );
    expect(verdictText(marketVerdict({ ...food, change: null, food: undefined }))).toBe("Food costs 1.31 cr and has too little trading yet to say where it's going.");
  });
  it("keeps the one but for an empty ask side", () => {
    const parts = marketVerdict({ ...food, soldOut: true, food: undefined });
    expect(verdictText(parts)).toBe("Food costs 1.31 cr and hasn't moved in 3 days, but nothing is on offer right now — sellers are met the moment they post.");
    expect(parts).toContainEqual({ text: "nothing is on offer right now", tone: "attn" });
    expect(verdictText(marketVerdict({ ...food, noAsks: true, food: undefined }))).toBe("Food costs 1.31 cr and hasn't moved in 3 days, but no one is selling right now.");
  });
  it("handles a book that has never traded and a share", () => {
    expect(verdictText(marketVerdict({ ...food, last: null, change: null, noAsks: true, food: undefined, wares: null }))).toBe(
      "No one has traded food yet — the first bid and ask to meet will set the price. Your plan has no Wares rule; Comfort holds only while you buy by hand.",
    );
    expect(verdictText(marketVerdict({ name: "shares of Iron & Sons", isShare: true, last: 20000, change: 12, days: 3, soldOut: false, noAsks: true, openOrders: 0 }))).toBe(
      "Shares of Iron & Sons last went for 200.00 cr and is up 12.0 % over 3 days.",
    );
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
