// Any list of money or goods movements (TDD 11), each line with its Explain.

import { credits } from "../api/client";
import { Num, type Explain } from "./Num";

export type LedgerRow = {
  key: string;
  when: string;
  what: string;
  cents?: number;
  goods?: string;
  explain?: Explain | null;
};

export function Ledger({ rows, empty = "Nothing yet." }: { rows: LedgerRow[]; empty?: string }) {
  if (rows.length === 0) {
    return <p className="text-muted text-sm">{empty}</p>;
  }
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
        {rows.map((r) => (
          <tr key={r.key} className="rule">
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
        ))}
      </tbody>
    </table>
  );
}
