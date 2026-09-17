// Any list of money or goods movements (TDD 11), each line with its Explain.
// Days restart with every epoch, so a list that reaches back past an epoch's
// start is cut into labelled groups: "Day 42" alone would not say which.

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
  explain?: Explain | null;
};

/** The line between two epochs in any table of events; `epoch` is 0-based, shown 1-based like the clock. */
export function EpochDivider({ epoch, current, span }: { epoch: number; current: boolean; span: number }) {
  return (
    <tr data-testid="epoch-divider">
      <td colSpan={span} className="text-muted border-line border-t-2 pt-3 pb-1 text-xs uppercase tracking-wide">
        Epoch {epoch + 1}
        {current ? " · this epoch" : " · ended"}
      </td>
    </tr>
  );
}

export function Ledger({ rows, empty = "Nothing yet." }: { rows: LedgerRow[]; empty?: string }) {
  if (rows.length === 0) {
    return <p className="text-muted text-sm">{empty}</p>;
  }
  const epochs = new Set(rows.flatMap((r) => (r.epoch === undefined ? [] : [r.epoch])));
  const latest = Math.max(...epochs);
  return (
    <table className="w-full text-sm">
      <thead className="text-muted text-left text-xs uppercase tracking-wide">
        <tr>
          <th className="py-1 font-normal">When</th>
          <th className="py-1 font-normal">What</th>
          <th className="py-1 text-right font-normal">Amount</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <Fragment key={r.key}>
            {epochs.size > 1 && r.epoch !== undefined && r.epoch !== rows[i - 1]?.epoch ? (
              <EpochDivider epoch={r.epoch} current={r.epoch === latest} span={3} />
            ) : null}
          <tr className="rule">
            <td className="num py-1 pr-3 align-top whitespace-nowrap">{r.when}</td>
            <td className="py-1 pr-3 align-top">{r.what}</td>
            <td className="py-1 text-right align-top whitespace-nowrap">
              {r.cents !== undefined ? (
                <Num
                  value={`${r.cents >= 0 ? "+" : "−"}${credits(Math.abs(r.cents))}`}
                  unit="cr"
                  explain={r.explain}
                />
              ) : r.goods ? (
                <Num value={r.goods} explain={r.explain} />
              ) : null}
            </td>
          </tr>
          </Fragment>
        ))}
      </tbody>
    </table>
  );
}
