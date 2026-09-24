// The state a Jev turn is decided against (SJ.1): a small object built from
// `/home` and the slots' own facts. Deterministic in its input, so a replay
// sends the same bytes; no timestamps, no ids, no prose that could steer the
// answer, and every comparison precomputed, because Jev cannot count.

import type { HomeView } from "../../api/client.ts";
import type { TurnInput } from "../brain.ts";
import type { Memory, ScoreboardView } from "../scripted/script.ts";
import type { Slot } from "./candidates.ts";

/** Where the citizen stands on the society's scoreboard (SJ.2): rank by net worth, and the gaps, precomputed. */
export type Standing = {
  net_worth_credits: string;
  self_made_credits: string;
  rank: number;
  of: number;
  trend_since_yesterday: "rising" | "falling" | "steady" | "unknown";
  gap_to_above_credits: string | null;
  gap_to_below_credits: string | null;
};

/** The standing from a scoreboard read; `yesterday` is the net worth at the last day's end, when known. */
export function standingOf(board: ScoreboardView | null, me: number, yesterday: unknown): Standing | null {
  if (!board) return null;
  const rows = [...board.rows].sort((a, b) => Number(b.net_worth) - Number(a.net_worth) || a.citizen - b.citizen);
  const i = rows.findIndex((r) => r.citizen === me);
  if (i < 0) return null;
  const mine = Number(rows[i]!.net_worth);
  const d = typeof yesterday === "number" ? mine - yesterday : null;
  return {
    net_worth_credits: credits(mine),
    self_made_credits: credits(Number(rows[i]!.self_made)),
    rank: i + 1,
    of: rows.length,
    trend_since_yesterday: d === null ? "unknown" : d > 100 ? "rising" : d < -100 ? "falling" : "steady",
    gap_to_above_credits: i > 0 ? credits(Number(rows[i - 1]!.net_worth) - mine) : null,
    gap_to_below_credits: i < rows.length - 1 ? credits(mine - Number(rows[i + 1]!.net_worth)) : null,
  };
}

/** One sentence on the standing, for the questions' instructions. */
export function standingText(st: Standing | null): string | null {
  if (!st) return null;
  const parts = [`You stand ${st.rank} of ${st.of} by net worth (${st.net_worth_credits} credits), ${st.trend_since_yesterday === "unknown" ? "on the first day" : st.trend_since_yesterday + " since yesterday"}`];
  if (st.gap_to_above_credits !== null) parts.push(`${st.gap_to_above_credits} behind the one above`);
  if (st.gap_to_below_credits !== null) parts.push(`${st.gap_to_below_credits} ahead of the one below`);
  return parts.join("; ") + ".";
}

export type JevState = {
  clock: { day: number; hour: number; hours_left_today: number; epoch_ended: boolean };
  me: {
    food: number;
    shelter: number;
    comfort: number;
    food_trend: "falling" | "steady" | "rising" | "unknown";
    balance_credits: string;
    pantry: Record<string, number>;
    housed: boolean;
    jobs: number;
    hours_set_today: number;
    hour_budget: number;
    fatigue_debt: number;
  };
  society: { population: number; unemployed: number; food_price_credits: string | null; price_index: string | null; headlines: string[] };
  since_last_look: Record<string, number>;
  last_turn: { chosen: Record<string, string>; refusals: string[] } | null;
  standing: Standing | null;
  slots: Record<string, Record<string, unknown>>;
};

const credits = (cents: number) => (cents / 100).toFixed(2);

export function foodTrend(now: number, before: unknown): JevState["me"]["food_trend"] {
  if (typeof before !== "number") return "unknown";
  const d = now - before;
  return d < -1 ? "falling" : d > 1 ? "rising" : "steady";
}

export function buildState(input: TurnInput, slots: Slot[], memory: Memory, standing: Standing | null = null): JevState {
  const home: HomeView = input.home;
  const c = home.clock;
  const h = home.household;
  const pantry = Object.fromEntries(Object.entries(h.pantry as Record<string, number>).filter(([, n]) => n > 0));
  const since: Record<string, number> = {};
  for (const e of home.since_last_seen.events) since[e.kind] = (since[e.kind] ?? 0) + 1;
  const slotFacts: Record<string, Record<string, unknown>> = {};
  for (const s of slots) slotFacts[s.key] = s.facts;
  return {
    clock: { day: c.cycle, hour: c.tick, hours_left_today: Math.max(0, c.ticks_per_cycle - c.tick), epoch_ended: c.epoch_ended },
    me: {
      food: Math.round(home.needs.food),
      shelter: Math.round(home.needs.shelter),
      comfort: Math.round(home.needs.comfort),
      food_trend: foodTrend(home.needs.food, memory.lastFood),
      balance_credits: credits(h.balance),
      pantry,
      housed: h.dwelling != null,
      jobs: home.labor.employment.length,
      hours_set_today: home.labor.allocations.reduce((n, a) => n + a.hours, 0),
      hour_budget: home.labor.budget,
      fatigue_debt: home.labor.fatigue_debt ?? 0,
    },
    society: {
      population: home.society.population,
      unemployed: home.society.unemployed,
      food_price_credits: home.society.food_last_price != null ? credits(home.society.food_last_price) : null,
      price_index: home.society.price_index != null ? home.society.price_index.toFixed(2) : null,
      headlines: home.headlines.slice(0, 3).map((x) => x.text),
    },
    since_last_look: since,
    last_turn: input.lastTurn
      ? {
          chosen: (memory.lastChosen as Record<string, string> | undefined) ?? {},
          refusals: input.lastTurn.rejections.map((r) => `${r.tool}: ${r.detail}`),
        }
      : null,
    standing,
    slots: slotFacts,
  };
}
