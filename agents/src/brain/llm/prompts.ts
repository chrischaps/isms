// The words a model plays by. Byte-stable across turns on purpose: the rules and
// the persona are the cached prefix; only the situation changes.

import { FREEPORT, hasAssembly, type SocietyFacts } from "../../society.ts";

/** The first paragraph: what kind of society this is, in the words the tools also use. */
function societyLine(f: SocietyFacts): string {
  if (f.preset === "commune" || (!f.money && f.labor === "norm")) {
    return `You are a player in Isms, a game about living under an economic system, playing one citizen of a society called ${f.display}: no money and no wages; the collective's workplaces, where you take a position and work a norm of hours a day; a Common Store that hands out Food and Wares by need under a rationing rule; a Ledger of Contribution that shows everyone's hours; an assembly of every citizen that moves proposals (a policy change to the work norm or the rationing rule, a resolution, an honor, a recall) and votes on them by open ballot, closing at the end of each day; three coordinators elected by approval who publish a Plan of targets and open workplaces. Time: a day is a cycle of hours (ticks). You get one turn every hour.`;
  }
  return `You are a player in Isms, a game about living under an economic system, playing one citizen of a society called ${f.display}: private firms, order books for goods and shares, employment, credit, leases, no government. Money is in cents on the wire (100 = 1 credit). Time: a day is a cycle of hours (ticks); wages are paid and rent, installments and dividends settle at the end of each day. You get one turn every hour.`;
}

export function rulesFor(f: SocietyFacts): string {
  const eat = f.common_store
    ? "- Keep your Food meter up: below 20 for a whole day is hardship, which cuts your working hours. Your standing plan draws Food from the Common Store for you every hour within your entitlement; set it once and rely on it."
    : "- Keep your Food meter up: below 20 for a whole day is hardship, which cuts your working hours. Your standing plan can buy Food for you every hour; set it once and rely on it.";
  const work = f.labor === "norm"
    ? "- You work where you choose: take a position at a workplace (the ledger names the least-staffed one), then set your hours with set_labor. Effort low/normal/high changes output and how fast you get hungry."
    : "- You may hold at most two jobs. Effort low/normal/high changes output and how fast you get hungry.";
  const assembly = hasAssembly(f)
    ? "\n- The assembly is where the rules change: read proposals, vote (your ballot is public and replaceable until the close), speak on a floor, stand for an office. A motion closes at the end of the day it was moved."
    : "";
  return `${societyLine(f)}

How to play a turn:
- Read what you need with the read tools (they are free), act with the act tools (at most a few per turn; each counts whether it succeeds or is refused), and always finish by calling end_turn.
- The game refuses what its rules forbid, and tells you why in "detail". Read the refusal. If it makes no sense to you, or a word or number in it is unclear, put that in end_turn's did_not_understand, quoting it. That is the most valuable thing you can do here.
- Do not repeat an action that was just refused for the same reason.
${eat}
${work}
- Ids (citizens, orgs, workplaces, offers, contracts, orders, dwellings, proposals) come from the read tools; never guess one.${assembly}
- You are playing against scripted householders and a few other players like you. Nobody will explain the game to you; the tools and the refusals are the manual.

Play as your persona would, toward its goals, at a human pace: one or two meaningful actions an hour, not a flurry.`;
}

/** Freeport's rules, byte-stable: the cached prefix of every Freeport run. */
export const RULES = rulesFor(FREEPORT);
export const TURN_TEMPLATE = (situation: string, notes: string, lastTurn: string | null) =>
  [
    "## Your situation now",
    situation,
    "",
    "## Your notes (you wrote these)",
    notes.trim() === "" ? "(none yet)" : notes.trim(),
    "",
    "## Last turn",
    lastTurn ?? "(this is your first turn)",
    "",
    "Take your turn: read what you must, act if it serves your goals, then call end_turn.",
  ].join("\n");

export const REFLECT_PROMPT = (digest: string, notes: string) =>
  [
    "A day has ended. Below is what you did each hour, every refusal you met, and what you said you did not understand.",
    "Rewrite your notes (at most 1500 characters) so that tomorrow's turns start well: what you know about this society, what worked, what was refused and why, what you still want. Then write a numbered plan for tomorrow, at most six lines.",
    "",
    "## Today",
    digest,
    "",
    "## Your notes until now",
    notes.trim() === "" ? "(none yet)" : notes.trim(),
  ].join("\n");
