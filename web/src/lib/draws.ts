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
