// Any list of money, goods or hours movements (TDD 11; docs/style.md §7.14),
// each line with its Explain. Header in caps, one line between rows, figures
// right-aligned and tabular, 44px rows on touch. Days restart with every
// epoch, so a list that reaches back past an epoch's start is cut into
// labelled groups: "Day 42" alone would not say which. A row carries one of
// `cents`, `goods` or `hours`: a payslip, a draw from the Store, a
// contribution to the norm (S2.11) — the same table in every society, and
// the currency appears only where a row has one.

import { Fragment } from "react";
import { credits } from "../api/client";
import { Num, type Explain } from "./Num";

export type LedgerRow = {
  key: string;
  /** The event's 0-based epoch; rows without one are never grouped. */
  epoch?: number;
  when: string;
  what: string;
  cents?: number;
  goods?: string;
  /** Hours given (the Ledger of Contribution's unit); shown as "+4 h". */
  hours?: number;
  explain?: Explain | null;
};

/** Table header cell style (§7.14): 13px caps, muted, a line beneath. */
export const TH = "text-muted border-line border-b px-2 py-1.5 text-left text-xs font-bold tracking-caps uppercase";
export const TH_NUM = `${TH} text-right`;
/** Body cell style: a line between rows, 44px minimum on touch. */
export const TD = "border-line border-b px-2 py-2.5 align-top";
export const TD_NUM = `${TD} text-right tabular-nums whitespace-nowrap`;

/** The line between two epochs in any table of events; `epoch` is 0-based, shown 1-based like the clock. */
export function EpochDivider({ epoch, current, span }: { epoch: number; current: boolean; span: number }) {
  return (
    <tr data-testid="epoch-divider">
      <td colSpan={span} className="text-muted border-line border-t-2 pt-3 pb-1 text-xs font-bold tracking-caps uppercase">
        Epoch {epoch + 1}
        {current ? " · this epoch" : " · ended"}
      </td>
    </tr>
  );
}

export function Ledger({
  rows,
  empty = "Nothing yet.",
  amountLabel = "Amount",
}: {
  rows: LedgerRow[];
  empty?: string;
  /** The last column's header: "Amount" for money, "Drew" for a draw record, "Hours" for the norm. */
  amountLabel?: string;
}) {
  if (rows.length === 0) {
    return <p className="text-muted m-0">{empty}</p>;
  }
  const epochs = new Set(rows.flatMap((r) => (r.epoch === undefined ? [] : [r.epoch])));
  const latest = Math.max(...epochs);
  return (
    <table className="w-full border-collapse text-[15px]">
      <thead>
        <tr>
          <th className={TH}>When</th>
          <th className={TH}>What</th>
          <th className={TH_NUM}>{amountLabel}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <Fragment key={r.key}>
            {epochs.size > 1 && r.epoch !== undefined && r.epoch !== rows[i - 1]?.epoch ? (
              <EpochDivider epoch={r.epoch} current={r.epoch === latest} span={3} />
            ) : null}
            <tr className="hover:bg-surface-2">
              <td className={`${TD} tabular-nums whitespace-nowrap`}>{r.when}</td>
              <td className={TD}>{r.what}</td>
              <td className={TD_NUM}>
                {r.cents !== undefined ? (
                  <Num value={`${r.cents >= 0 ? "+" : "−"}${credits(Math.abs(r.cents))}`} unit="cr" explain={r.explain} />
                ) : r.goods ? (
                  <Num value={r.goods} explain={r.explain} />
                ) : r.hours !== undefined ? (
                  <Num value={`${r.hours >= 0 ? "+" : "−"}${Number.isInteger(r.hours) ? Math.abs(r.hours) : Math.abs(r.hours).toFixed(1)}`} unit="h" explain={r.explain} />
                ) : null}
              </td>
            </tr>
          </Fragment>
        ))}
      </tbody>
    </table>
  );
}
