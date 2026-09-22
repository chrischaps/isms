// Your own line of the Ledger of Contribution as ledger rows (S2.11; GDD 6.2):
// the hours given today and yesterday, each against the norm. The wire
// carries two days of a citizen's record, not a history (Q157), so the list
// is two rows at most and says so in the words, not in a gap.

import type { LedgerRow } from "../components/Ledger";
import { dayOf } from "./when";

export type MyContribution = {
  hours_today: number;
  hours_yesterday: number;
  norm_met_today: boolean;
};

/** Today's and yesterday's rows, newest first; yesterday only once a day has closed. `cycle` and `epoch` 0-based. */
export function contributionRows(me: MyContribution, norm: number | null, cycle: number, epoch: number, workplace?: string): LedgerRow[] {
  const line = (hours: number, met: boolean | null): string => {
    const where = workplace ? ` at ${workplace}` : "";
    if (norm == null) return `Gave hours${where}`;
    if (met === true) return `Met the norm of ${norm}${where}`;
    if (hours === 0) return `No hours given${where}`;
    return `Short of the norm of ${norm}${where}`;
  };
  const rows: LedgerRow[] = [
    { key: `today-${cycle}`, epoch, when: `${dayOf(cycle)}, so far`, what: line(me.hours_today, norm == null ? null : me.norm_met_today), hours: me.hours_today },
  ];
  if (cycle > 0) {
    const met = norm == null ? null : me.hours_yesterday >= norm;
    rows.push({ key: `yesterday-${cycle - 1}`, epoch, when: dayOf(cycle - 1), what: line(me.hours_yesterday, met), hours: me.hours_yesterday });
  }
  return rows;
}
