// Verdicts (docs/style.md §7.5, §8.2): one sentence per screen, in the shell
// voice, written from the player's side of the screen. Shape: [good thing]
// and [good thing], but [the one thing to act on] — one "but" at most, so a
// verdict names the worst problem and the rest appear in their own cards.
// States are named in the player's words (fed, housed, working), not the
// meter's, and each carries its tone by the engine's thresholds (§4.2).
// Everything here is pure: the screen hands in what the API gave it.

import type { Tone, VerdictPart } from "../components/Verdict";

export type HomeVerdictInput = {
  food: number;
  shelter: number;
  comfort: number;
  /** The engine's hardship flag: Food under the line for a whole day. */
  hardship: boolean;
  housed: boolean;
  /** Hours allocated today, over every position. */
  hours: number;
  /** Holds a position or a contract, hours set or not. */
  hired: boolean;
  /** The hardship line, where the meter draws it. */
  threshold?: number;
};

/** A need meter's tone by the bible's thresholds: crit under the line, attn under 50. */
export function needTone(value: number, threshold = 20): Tone {
  if (value < threshold) return "crit";
  if (value < 50) return "attn";
  return "good";
}

type Problem = { pre: string; text: string; post: string; tone: Tone };

/** The one thing to act on, worst first; `null` when nothing needs you. */
function worst(i: HomeVerdictInput): Problem | null {
  const line = i.threshold ?? 20;
  if (i.hardship) return { pre: "you're ", text: "in hardship", post: " — Food has been under the line for a whole day. Eat first.", tone: "crit" };
  if (i.food < line) return { pre: "you're ", text: "running out of food", post: ". Eat first.", tone: "crit" };
  if (i.shelter < line && !i.housed) return { pre: "you're ", text: "out in the cold", post: " — rent or buy a dwelling.", tone: "crit" };
  if (i.food < 50) return { pre: "you're ", text: "getting hungry", post: ".", tone: "attn" };
  if (!i.housed) return { pre: "you ", text: "don't have a place to live", post: " yet.", tone: "attn" };
  if (i.hours === 0) {
    return i.hired
      ? { pre: "you ", text: "haven't set your hours", post: " yet.", tone: "attn" }
      : { pre: "you ", text: "don't have work", post: " yet.", tone: "attn" };
  }
  if (i.comfort < line) return { pre: "you're ", text: "short on comfort", post: " — a few Wares would help.", tone: "crit" };
  if (i.comfort < 50) return { pre: "you're ", text: "low on comfort", post: ".", tone: "attn" };
  return null;
}

export function homeVerdict(i: HomeVerdictInput): VerdictPart[] {
  const goods: string[] = [];
  if (i.food >= 50 && !i.hardship) goods.push("well fed");
  if (i.housed) goods.push("housed");
  if (i.hours > 0) goods.push("working today");
  const p = worst(i);
  const good = (text: string): VerdictPart => ({ text, tone: "good" });
  const parts: VerdictPart[] = [];
  if (goods.length > 0) {
    parts.push("You're ");
    goods.forEach((g, n) => {
      if (n > 0) parts.push(n === goods.length - 1 ? " and " : ", ");
      parts.push(good(g));
    });
  }
  if (!p) {
    parts.push(goods.length > 0 ? ". Nothing needs you right now." : "Nothing needs you right now.");
    return parts;
  }
  const pre = goods.length > 0 ? `, but ${p.pre}` : p.pre.charAt(0).toUpperCase() + p.pre.slice(1);
  parts.push(pre, { text: p.text, tone: p.tone }, p.post);
  return parts;
}

/** A list in prose: "a", "a and b", "a, b and c". */
function join(parts: VerdictPart[][]): VerdictPart[] {
  return parts.flatMap((p, n) => (n === 0 ? p : [n === parts.length - 1 ? " and " : ", ", ...p]));
}

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

export type WorkVerdictInput = {
  /** Hours allocated today, as the engine holds them. */
  hours: number;
  budget: number;
  fatigue: number;
  /** Every position held, with the hours and effort set at it. */
  positions: { name: string; hours: number; effort: string }[];
  /** Positions come without a contract; the picker is on this screen. */
  byNorm: boolean;
};

/** Work (§10): "You're working 8 of 8 hours at Legacy Farm No. 1 at normal effort." */
export function workVerdict(i: WorkVerdictInput): VerdictPart[] {
  if (i.positions.length === 0) {
    return i.byNorm
      ? ["You hold ", { text: "no position", tone: "attn" }, " yet — take one below."]
      : ["You ", { text: "don't have work", tone: "attn" }, " yet — the job board is on the Organizations screen."];
  }
  const names = (ps: typeof i.positions) => join(ps.map((p) => [p.name]));
  if (i.hours === 0) {
    return ["You hold a position at ", ...names(i.positions), ", but you ", { text: "haven't set your hours", tone: "attn" }, " yet."];
  }
  const working = i.positions.filter((p) => p.hours > 0);
  const efforts = new Set(working.map((p) => p.effort));
  const effort = efforts.size === 1 ? `${[...efforts][0]} effort` : "mixed effort";
  const parts: VerdictPart[] = ["You're ", { text: `working ${i.hours} of ${i.budget} hours`, tone: "good" }, " at ", ...names(working), ` at ${effort}`];
  if (i.fatigue > 0) parts.push(", but ", { text: "fatigue", tone: "attn" }, ` has taken ${plural(i.fatigue, "hour")} off today's budget.`);
  else parts.push(".");
  return parts;
}

export type PlanVerdictInput = {
  keepFood: number;
  /** Cents; 0 means the plan may spend everything. */
  keepBalance: number;
  /** A Wares rule is set. */
  wares: boolean;
  orders: number;
  money: boolean;
  /** Food comes from the Common Store, not a market. */
  store: boolean;
  labor: "explicit" | "accept_assignment" | "follow_norm";
  vote: "none" | "abstain" | "follow" | null;
  followHandle?: string;
};

/** Standing plan (§10): "Your plan keeps you fed and spends everything else." */
export function planVerdict(i: PlanVerdictInput): VerdictPart[] {
  const goods: VerdictPart[][] = [];
  if (i.keepFood > 0) goods.push([{ text: "keeps you fed", tone: "good" }]);
  if (i.money && i.keepBalance > 0) goods.push([{ text: `keeps ${(i.keepBalance / 100).toFixed(2)} cr in hand`, tone: "good" }]);
  if (i.wares) goods.push([{ text: `${i.store ? "draws" : "buys"} Wares when Comfort dips`, tone: "good" }]);
  if (i.orders > 0) goods.push([{ text: `places ${plural(i.orders, "standing order")}`, tone: "good" }]);
  if (i.labor === "follow_norm") goods.push([{ text: "works the norm", tone: "good" }]);
  if (i.labor === "accept_assignment") goods.push([{ text: "accepts your assignment", tone: "good" }]);
  if (i.vote === "follow") goods.push([{ text: `votes with ${i.followHandle ?? "a citizen"}`, tone: "good" }]);
  if (i.vote === "abstain") goods.push([{ text: "abstains for you", tone: "good" }]);
  if (i.money && i.keepBalance === 0 && i.keepFood > 0) goods.push([{ text: "spends everything else", tone: "ink" }]);
  const parts: VerdictPart[] = goods.length > 0 ? ["Your plan ", ...join(goods)] : ["Your plan does nothing on its own"];
  if (i.keepFood === 0) {
    parts.push(goods.length > 0 ? ", but it " : " and ", { text: `won't ${i.store ? "draw" : "buy"} Food`, tone: "attn" }, " for you — set a pantry floor.");
  } else {
    parts.push(". It runs every hour, here or not.");
  }
  return parts;
}

export type MarketVerdictInput = {
  /** The instrument's name as the screen shows it ("food", "shares of Iron & Sons"). */
  name: string;
  isShare: boolean;
  /** Last trade in cents; null when nothing has traded. */
  last: number | null;
  /** Signed percentage over the window; null with too few points. */
  change: number | null;
  days: number;
  /** An empty ask side with a busy tape: the good sells as it is posted. */
  soldOut: boolean;
  noAsks: boolean;
  /** For food: the plan's pantry floor against the pantry now. */
  food?: { keepFood: number; pantryFood: number };
  /** For wares: the plan's Comfort threshold, or null with no rule. */
  wares?: { comfortBelow: number } | null;
  openOrders: number;
};

/** Market (§10): "Food costs 1.31 cr and hasn't moved in 3 days. Your plan will buy 2 food next hour." */
export function marketVerdict(i: MarketVerdictInput): VerdictPart[] {
  const name = i.name.charAt(0).toUpperCase() + i.name.slice(1);
  const parts: VerdictPart[] = [];
  if (i.last === null) {
    parts.push({ text: `No one has traded ${i.name} yet`, tone: "ink" }, " — the first bid and ask to meet will set the price");
  } else {
    parts.push({ text: `${name} ${i.isShare ? "last went for" : "costs"} ${(i.last / 100).toFixed(2)} cr`, tone: "ink" });
    if (i.change === null) parts.push(` and has too little trading yet to say where it's going`);
    else if (Math.abs(i.change) < 0.5) parts.push(" and ", { text: "hasn't moved", tone: "ink" }, ` in ${plural(i.days, "day")}`);
    else parts.push(" and is ", { text: `${i.change > 0 ? "up" : "down"} ${Math.abs(i.change).toFixed(1)} %`, tone: "ink" }, ` over ${plural(i.days, "day")}`);
  }
  if (i.soldOut) parts.push(", but ", { text: "nothing is on offer right now", tone: "attn" }, " — sellers are met the moment they post.");
  else if (i.noAsks && !i.isShare && i.last !== null) parts.push(", but ", { text: "no one is selling", tone: "attn" }, " right now.");
  else parts.push(".");
  if (i.food) {
    const short = Math.max(0, i.food.keepFood - i.food.pantryFood);
    parts.push(short > 0 ? ` Your plan will bid for ${short} food next hour.` : i.food.keepFood > 0 ? ` Your plan keeps Food at ${i.food.keepFood} and has no shortfall to bid for.` : " Your plan has no Food floor, so it never bids here.");
  }
  if (i.wares !== undefined) {
    parts.push(i.wares ? ` Your plan buys Wares when Comfort falls below ${i.wares.comfortBelow}.` : " Your plan has no Wares rule; Comfort holds only while you buy by hand.");
  }
  if (i.openOrders > 0) parts.push(` You have ${plural(i.openOrders, "open order")} here.`);
  return parts;
}

/**
 * Home's greeting (§5): the h1 is a greeting because the screen's real title
 * is the Verdict. The hour is the society's, not the wall clock's — it is
 * always morning somewhere, but here it is the morning the player is in.
 */
export function greeting(handle: string, clock: { tick: number; ticks_per_cycle: number }): string {
  if (clock.ticks_per_cycle !== 24) return `Hello, ${handle}.`;
  const hour = (Math.max(1, clock.tick) - 1) % 24;
  const word =
    hour < 5 ? "Hello" : hour < 12 ? "Good morning" : hour < 17 ? "Good afternoon" : hour < 22 ? "Good evening" : "Hello";
  return `${word}, ${handle}.`;
}
