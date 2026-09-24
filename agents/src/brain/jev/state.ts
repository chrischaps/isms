// The state a Jev turn is decided against (SJ.1): a small object built from
// `/home` and the slots' own facts. Deterministic in its input, so a replay
// sends the same bytes; no timestamps, no ids, no prose that could steer the
// answer, and every comparison precomputed, because Jev cannot count.

import type { HomeView } from "../../api/client.ts";
import type { TurnInput } from "../brain.ts";
import type { Memory } from "../scripted/script.ts";
import type { Slot } from "./candidates.ts";

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
  slots: Record<string, Record<string, unknown>>;
};

const credits = (cents: number) => (cents / 100).toFixed(2);

export function foodTrend(now: number, before: unknown): JevState["me"]["food_trend"] {
  if (typeof before !== "number") return "unknown";
  const d = now - before;
  return d < -1 ? "falling" : d > 1 ? "rising" : "steady";
}

export function buildState(input: TurnInput, slots: Slot[], memory: Memory): JevState {
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
    slots: slotFacts,
  };
}
