// What a turn starts from: a small extract of `/home` for the journal (the arc
// is drawn from it) and a plain-text rendering for a model to read. Both are
// deterministic functions of the view, so a replay reads the same words.

import type { HomeView } from "../api/client.ts";
import type { Situation } from "../journal/journal.ts";

export function extract(home: HomeView): Situation {
  const pantry = home.household.pantry as Record<string, number>;
  return {
    balance: home.household.balance,
    food: home.needs.food,
    shelter: home.needs.shelter,
    comfort: home.needs.comfort,
    housed: home.household.dwelling != null,
    // Contract-less positions under a norm count as jobs (S2.10); in Freeport every position has a contract.
    jobs: home.labor.employment.length + (home.labor.positions ?? []).filter((p) => p.contract == null).length,
    pantry_food: pantry.food ?? 0,
  };
}

const credits = (cents: number) => (cents / 100).toFixed(2);

/** The situation as the model reads it: numbers with units, nothing the tools would not also say. */
export function situationText(home: HomeView): string {
  const c = home.clock;
  const h = home.household;
  const pantry = Object.entries(h.pantry as Record<string, number>)
    .filter(([, n]) => n > 0)
    .map(([g, n]) => `${g} ${n}`)
    .join(", ");
  const jobs = home.labor.employment.map((e) => {
    const b = (e.body as { employment?: Record<string, unknown> }).employment ?? {};
    const pay = (b.pay ?? {}) as { hourly?: number; piece_rate?: number };
    const rate = pay.hourly !== undefined ? `${credits(pay.hourly)}/h` : pay.piece_rate !== undefined ? `${credits(pay.piece_rate)} a unit` : "?";
    return `contract ${e.id}: org ${String(b.org)} workplace ${String(b.workplace)} at ${rate}, up to ${String(b.max_hours)} h/day (${e.status})`;
  });
  const positions = (home.labor.positions ?? []).filter((p) => p.contract == null).map((p) => `workplace ${p.workplace} (${p.org_name}, ${p.kind})`);
  const alloc = home.labor.allocations.map((a) => `${a.hours} h at workplace ${a.workplace} (${a.org_name}, ${a.kind}) effort ${a.effort}`);
  const dwelling = h.dwelling
    ? `dwelling ${h.dwelling.id}${h.dwelling.rent_per_cycle != null ? ` renting at ${credits(h.dwelling.rent_per_cycle)} a day` : ", yours"}`
    : "unhoused";
  const flags = Object.entries(home.citizen.flags as Record<string, unknown>)
    .filter(([, v]) => v === true)
    .map(([k]) => k);
  const since = home.since_last_seen.events;
  const kinds = new Map<string, number>();
  for (const e of since) kinds.set(e.kind, (kinds.get(e.kind) ?? 0) + 1);
  const touched = [...kinds].map(([k, n]) => (n > 1 ? `${k} x${n}` : k)).join(", ");
  return [
    `Epoch ${c.epoch}, day ${c.cycle}, hour ${c.tick} of ${c.ticks_per_cycle}${c.epoch_ended ? " (the epoch has ended)" : ""}.`,
    `You are ${home.citizen.handle} (citizen ${home.citizen.id})${flags.length ? `; flags: ${flags.join(", ")}` : ""}.`,
    `Needs: food ${home.needs.food.toFixed(0)}, shelter ${home.needs.shelter.toFixed(0)}, comfort ${home.needs.comfort.toFixed(0)} (each 0-100; below 20 food for a whole day is hardship).`,
    `Balance ${credits(h.balance)} credits. Pantry: ${pantry || "empty"}. Housing: ${dwelling}.`,
    positions.length ? `Positions without a contract (under a norm, or at your own firm): ${positions.join("; ")}.` : `Jobs: ${jobs.length ? jobs.join("; ") : "none"}.`,
    `Labor this day: ${alloc.length ? alloc.join("; ") : "nothing allocated"}; budget ${home.labor.budget} h; output multiplier ${home.labor.output_mult.toFixed(2)}${home.labor.fatigue_debt ? `; fatigue debt ${home.labor.fatigue_debt} h` : ""}.`,
    `Society: ${home.society.population} citizens, ${home.society.active_humans} people, ${home.society.unemployed} unemployed; food last ${home.society.food_last_price != null ? credits(home.society.food_last_price) : "?"}; price index ${home.society.price_index?.toFixed(2) ?? "?"}.`,
    home.headlines.length ? `Headlines: ${home.headlines.map((x) => x.text).join(" | ")}` : "No headlines yet.",
    since.length ? `Since you last looked (${since.length} events): ${touched}.` : "Nothing touched you since you last looked.",
  ].join("\n");
}
