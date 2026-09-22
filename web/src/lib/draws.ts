// The draw record (GDD 9.1 step 3; S2.7): a `Drew` payload becomes one ledger
// line with its Explain, a draw by need told apart from a surplus share by
// the rule that produced it. Also the words for each rationing rule, which
// the Store screen states beside the shelves.

import type { EventRef } from "../api/client";
import type { LedgerRow } from "../components/Ledger";
import type { Explain } from "../components/Num";
import { whenOf } from "./when";

/** "2 food, 1 wares" for a goods map. */
export function goodsText(goods: Record<string, number> | undefined): string {
  return Object.entries(goods ?? {})
    .map(([g, n]) => `${n} ${g}`)
    .join(", ");
}

/** The rationing rule in force, as a sentence (GDD 6.2, TDD T8). */
export function ruleText(rule: string): string {
  switch (rule) {
    case "need_first":
      return "need first: the largest request is served first, whole, until the shelf is bare";
    case "equal_shortfall":
      return "equal shortfall: everyone gets the same share of what they asked for";
    case "lottery":
      return "lottery: requests are served whole in drawn order until the shelf is bare";
    default:
      return rule.replaceAll("_", " ");
  }
}

/**
 * A shelf as a bar (S2.11; docs/style.md §7.6 by analogy): the stock against
 * what is asked this hour, full when it covers every request, empty when
 * bare. With nothing asked, a stocked shelf is full and a bare one empty.
 * The tone is the engine's own state, not taste (§4.2): bare is crit, short
 * of what is asked is attn, covered is good.
 */
export function stockLevel(stock: number, requested: number): { value: number; tone: "good" | "attn" | "crit" } {
  if (stock <= 0) return { value: 0, tone: "crit" };
  if (requested <= 0) return { value: 100, tone: "good" };
  if (stock < requested) return { value: Math.max(1, Math.round((stock / requested) * 100)), tone: "attn" };
  return { value: 100, tone: "good" };
}

export function drawRows(draws: EventRef[], perDay = 24, limit?: number): LedgerRow[] {
  const recent = limit === undefined ? draws.slice() : draws.slice(-limit);
  return recent.reverse().map((d) => {
    const p = (d.payload.Drew ?? {}) as Record<string, unknown>;
    const explain = (p.explain as Explain | undefined) ?? null;
    const share = explain?.rule === "store_surplus_share";
    return {
      key: String(d.seq),
      epoch: d.epoch,
      when: whenOf(d, perDay),
      what: share ? "Surplus share at the day's end" : "Drew from the Store",
      goods: `+${goodsText(p.goods as Record<string, number> | undefined)}`,
      explain,
    };
  });
}
