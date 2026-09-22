// SB.2 done gate: the Home verdict names the state in the player's words,
// leads with the good, and has one "but" at most — the worst problem by the
// engine's thresholds; the rest stay in their cards (docs/style.md §8.2).

import { describe, expect, it } from "vitest";
import { verdictText } from "../components/Verdict";
import { needStatus } from "./needs";
import { archiveVerdict, assemblyVerdict, contractsVerdict, coordinatorVerdict, greeting, homeVerdict, ledgerVerdict, marketVerdict, needTone, orgVerdict, orgsVerdict, planVerdict, societyVerdict, storeVerdict, workVerdict } from "./verdict";

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

// SB.4: the five verdicts of Organizations, Org, Contracts, Society and Archive.
describe("orgsVerdict", () => {
  it("reads as the bible's example", () => {
    const parts = orgsVerdict({ hiring: 18, places: 18, workAt: ["Legacy Farm No. 1"], manage: [], byNorm: false });
    expect(verdictText(parts)).toBe("18 firms are hiring; you work at Legacy Farm No. 1.");
    expect(parts).toContainEqual({ text: "Legacy Farm No. 1", tone: "good" });
  });
  it("names the firms you manage beside the ones you work at", () => {
    expect(verdictText(orgsVerdict({ hiring: 1, places: 3, workAt: ["Legacy Farm No. 1"], manage: ["Iron & Sons"], byNorm: false }))).toBe(
      "1 firm is hiring (3 places); you work at Legacy Farm No. 1 and manage Iron & Sons.",
    );
  });
  it("makes no work the one but, and points at the board or the founding form", () => {
    const some = orgsVerdict({ hiring: 2, places: 2, workAt: [], manage: [], byNorm: false });
    expect(verdictText(some)).toBe("2 firms are hiring, but you don't have work yet — take a job below.");
    expect(some).toContainEqual({ text: "don't have work", tone: "attn" });
    expect(verdictText(orgsVerdict({ hiring: 0, places: 0, workAt: [], manage: [], byNorm: false }))).toBe("Nobody is hiring this hour and you don't have work yet — found a firm, or look again next hour.");
  });
  it("sends a norm citizen to the Work screen", () => {
    expect(verdictText(orgsVerdict({ hiring: 0, places: 0, workAt: [], manage: [], byNorm: true }))).toBe("Positions here come by the norm, not by hire; you hold no position yet — take one on the Work screen.");
  });
});

describe("orgVerdict", () => {
  const mine = { name: "Iron & Sons", share: 1, controlling: true, manage: true, employed: false, member: true, employees: 1, money: true, paymentMissed: false, hiring: 0 };
  it("names a shortfall in crit as the one but", () => {
    const parts = orgVerdict({ ...mine, payroll: { due: 6800, shortfall: 6800, workers: 1 } });
    expect(verdictText(parts)).toBe("Iron & Sons is yours: you hold 100 % and manage it, but the treasury is short by 68.00 cr for tonight's payroll.");
    expect(parts).toContainEqual({ text: "short by 68.00 cr", tone: "crit" });
  });
  it("says a covered payroll is covered, and an empty one empty", () => {
    expect(verdictText(orgVerdict({ ...mine, payroll: { due: 6800, shortfall: 0, workers: 1 } }))).toBe("Iron & Sons is yours: you hold 100 % and manage it. Tonight's payroll of 68.00 cr is covered.");
    expect(verdictText(orgVerdict({ ...mine, payroll: { due: 0, shortfall: 0, workers: 0 }, hiring: 2 }))).toBe("Iron & Sons is yours: you hold 100 % and manage it. Nobody is on the payroll; 2 places on the board.");
  });
  it("reads from an employee's and a stranger's side", () => {
    expect(verdictText(orgVerdict({ ...mine, share: 0, controlling: false, manage: false, employed: true, member: false }))).toBe("You work at Iron & Sons.");
    expect(verdictText(orgVerdict({ ...mine, share: 0, controlling: false, manage: false, member: false, employees: 4, paymentMissed: true }))).toBe("Iron & Sons employs 4; you have no part in it, but it missed a payday.");
    expect(verdictText(orgVerdict({ ...mine, share: 0.2, controlling: false, manage: false, employed: true, member: false }))).toBe("You hold 20 % of Iron & Sons, and you work here.");
  });
});

describe("contractsVerdict", () => {
  it("reads as the bible's example", () => {
    const parts = contractsVerdict({ housed: false, rent: null, shelter: 94, toLet: 0, active: 0, overdue: false });
    expect(verdictText(parts)).toBe("You have no dwelling and nothing to let is on the board.");
    expect(parts).toContainEqual({ text: "have no dwelling", tone: "attn" });
  });
  it("turns crit under the line and points at the board when something is to let", () => {
    const parts = contractsVerdict({ housed: false, rent: null, shelter: 12, toLet: 3, active: 1, overdue: false });
    expect(verdictText(parts)).toBe("You have no dwelling — 3 dwellings to let are on the board, rent one below.");
    expect(parts).toContainEqual({ text: "have no dwelling", tone: "crit" });
  });
  it("counts the running contracts when housed, with an overdue payment as the one but", () => {
    expect(verdictText(contractsVerdict({ housed: true, rent: 800, shelter: 90, toLet: 2, active: 2, overdue: false }))).toBe("You're housed at 8.00 cr a day and 2 contracts are running. Nothing needs you right now.");
    expect(verdictText(contractsVerdict({ housed: true, rent: null, shelter: 90, toLet: 0, active: 1, overdue: true }))).toBe("You're housed in a dwelling of your own and 1 contract is running, but a payment is overdue.");
  });
});

describe("societyVerdict", () => {
  it("reads as the bible's example", () => {
    const parts = societyVerdict({ name: "Freeport", fed: 0.98, hardship: 1, population: 41, price: { now: 1.0, yesterday: 1.0 } });
    expect(verdictText(parts)).toBe("Freeport is fed (98 %), one citizen is in hardship, prices are steady.");
    expect(parts).toContainEqual({ text: "fed (98 %)", tone: "good" });
    expect(parts).toContainEqual({ text: "one citizen is in hardship", tone: "attn" });
  });
  it("moves prices, drops them without money, and goes crit when a tenth are in hardship", () => {
    expect(verdictText(societyVerdict({ name: "Freeport", fed: 0.6, hardship: 5, population: 40, price: { now: 1.1, yesterday: 1.0 } }))).toBe("Freeport is partly fed (60 %), 5 citizens are in hardship, prices are up 10 %.");
    const commune = societyVerdict({ name: "The Commune", fed: 1, hardship: 0, population: 30, price: null });
    expect(verdictText(commune)).toBe("The Commune is fed (100 %), nobody is in hardship.");
    expect(societyVerdict({ name: "F", fed: 0.4, hardship: 4, population: 40, price: null })).toContainEqual({ text: "4 citizens are in hardship", tone: "crit" });
  });
  it("waits for the first day to close", () => {
    expect(verdictText(societyVerdict({ name: "Freeport", fed: null, hardship: null, population: 41, price: null }))).toBe("Freeport is on its first day; the numbers come when it ends.");
  });
});

describe("archiveVerdict", () => {
  it("says how the last epoch ended and where this one is", () => {
    expect(verdictText(archiveVerdict({ latest: { epoch: 1, reason: "scheduled", final_cycle: 41, open: false }, clock: { epoch: 2, cycle: 3 } }))).toBe("Epoch 1 ran its course after 41 days; epoch 2 is on Day 3.");
    const fell = archiveVerdict({ latest: { epoch: 1, reason: "collapse", final_cycle: 12, open: true }, clock: { epoch: 2, cycle: 1 } });
    expect(verdictText(fell)).toBe("Epoch 1 collapsed on Day 12, and closing statements are still open.");
    expect(fell).toContainEqual({ text: "collapsed", tone: "crit" });
  });
  it("says so when nothing has closed", () => {
    expect(verdictText(archiveVerdict({ latest: null, clock: { epoch: 1, cycle: 5 } }))).toBe("No epoch has closed here yet; this is epoch 1, Day 5.");
  });
});

// SB.5 done gate: the four screens of the last card speak the same way.
const office = { kind: "coordinator", seats: 1, holders: 1, iHold: false, election: false, iStand: false };

describe("assemblyVerdict", () => {
  it("counts the proposals and owes a ballot as the one but", () => {
    const parts = assemblyVerdict({ open: 2, uncast: 1, mine: 0, offices: [office] });
    expect(verdictText(parts)).toBe("2 proposals are before the assembly tonight, but you haven't cast on one of them yet.");
    expect(parts).toContainEqual({ text: "haven't cast on one of them", tone: "attn" });
    expect(verdictText(assemblyVerdict({ open: 1, uncast: 1, mine: 1, offices: [office] }))).toBe("One proposal is before the assembly tonight, but you haven't cast on it yet.");
  });
  it("leads with the office you hold and rests when every ballot is cast", () => {
    const parts = assemblyVerdict({ open: 1, uncast: 0, mine: 0, offices: [{ ...office, iHold: true }] });
    expect(verdictText(parts)).toBe("One proposal is before the assembly tonight, and you hold coordinator, and your ballot is cast. Nothing needs you right now.");
    expect(parts).toContainEqual({ text: "hold coordinator", tone: "good" });
  });
  it("points at an open election you are not in, then at an empty seat", () => {
    expect(verdictText(assemblyVerdict({ open: 0, uncast: 0, mine: 0, offices: [{ ...office, holders: 0, election: true }] }))).toBe("Nothing is before the assembly tonight, but an election for coordinator is open — approve a candidate or stand.");
    expect(verdictText(assemblyVerdict({ open: 0, uncast: 0, mine: 0, offices: [{ ...office, holders: 0, election: true, iStand: true }] }))).toBe("Nothing is before the assembly tonight. Nothing needs you right now.");
    expect(verdictText(assemblyVerdict({ open: 0, uncast: 0, mine: 0, offices: [{ ...office, holders: 0 }] }))).toBe("Nothing is before the assembly tonight, but the coordinator seat is empty.");
  });
});

describe("coordinatorVerdict", () => {
  const sits = { office: "coordinator", termEnds: 12, planPublished: true, targets: { set: 3, met: 2, measured: 3 }, materials: { held: 40, cost: 20 }, freeSlots: 2 };
  it("says the seat, the term, the Plan and rests", () => {
    const parts = coordinatorVerdict(sits);
    expect(verdictText(parts)).toBe("You sit as coordinator through Day 12, and the Plan is published (2 of 3 targets met yesterday). Nothing needs you right now.");
    expect(parts).toContainEqual({ text: "sit as coordinator", tone: "good" });
  });
  it("asks for a Plan first, then Materials, then a slot", () => {
    expect(verdictText(coordinatorVerdict({ ...sits, planPublished: false, materials: { held: 0, cost: 20 } }))).toBe("You sit as coordinator through Day 12, but no Plan is published yet — set the targets below.");
    expect(verdictText(coordinatorVerdict({ ...sits, targets: { set: 0, met: 0, measured: 0 }, materials: { held: 5, cost: 20 } }))).toBe("You sit as coordinator through Day 12, and the Plan is published, but the Store is short of Materials to open a workplace (5 of 20).");
    expect(verdictText(coordinatorVerdict({ ...sits, targets: { set: 0, met: 0, measured: 0 }, freeSlots: 0 }))).toBe("You sit as coordinator through Day 12, and the Plan is published, but every slot on the land is taken.");
  });
});

describe("storeVerdict", () => {
  it("says the shelves are stocked and what you may draw", () => {
    const parts = storeVerdict({ bare: [], food: { stock: 40, entitlement: 3, pending: 0 }, rule: "need_first" });
    expect(verdictText(parts)).toBe("The shelves are stocked, and you may draw 3 Food this hour; your plan asks at the next.");
    expect(parts).toContainEqual({ text: "shelves are stocked", tone: "good" });
    expect(verdictText(storeVerdict({ bare: [], food: { stock: 40, entitlement: 0, pending: 0 }, rule: "need_first" }))).toBe("The shelves are stocked, and your pantry is full.");
    expect(verdictText(storeVerdict({ bare: [], food: { stock: 40, entitlement: 2, pending: 2 }, rule: "need_first" }))).toBe("The shelves are stocked, and your plan has asked for 2 Food this hour.");
  });
  it("names a bare shelf in crit", () => {
    const parts = storeVerdict({ bare: ["food", "wares"], food: { stock: 0, entitlement: 3, pending: 0 }, rule: "need_first" });
    expect(verdictText(parts)).toBe("The shelf is bare of food and wares — what the workplaces make this hour is served this hour, by the rule, and you have room for 3 Food with none to draw.");
    expect(parts).toContainEqual({ text: "bare of food and wares", tone: "crit" });
  });
});

describe("ledgerVerdict", () => {
  it("keeps the spec's opening words and the norm", () => {
    const parts = ledgerVerdict({ norm: 6, hoursToday: 4, normMet: false, workplaces: ["workshop"], days: 2, metDays: 1, leastStaffed: null });
    expect(verdictText(parts)).toBe("You have given 4 of the norm's 6 hours today at the workshop. Over 2 days on the record you met the norm once.");
    const met = ledgerVerdict({ norm: 6, hoursToday: 6.5, normMet: true, workplaces: ["farm", "workshop"], days: 0, metDays: 0, leastStaffed: null });
    expect(verdictText(met)).toBe("You have given 6.5 hours today, the norm met at the farm and workshop.");
    expect(met).toContainEqual({ text: "6.5 hours today, the norm met", tone: "good" });
    expect(verdictText(ledgerVerdict({ norm: null, hoursToday: 1, normMet: false, workplaces: ["farm"], days: 0, metDays: 0, leastStaffed: null }))).toBe("You have given 1 hour today at the farm.");
  });
  it("sends someone with no position to Work and names where labor is scarcest", () => {
    const parts = ledgerVerdict({ norm: 6, hoursToday: 0, normMet: false, workplaces: [], days: 0, metDays: 0, leastStaffed: "farm" });
    expect(verdictText(parts)).toBe("You hold no position today — take one on the Work screen; labor is scarcest at the farm.");
    expect(parts).toContainEqual({ text: "hold no position", tone: "attn" });
  });
});
