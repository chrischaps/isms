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
