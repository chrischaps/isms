// Payslip events as ledger rows (S1.8, shared by Home and Work in S1.9): a
// `Paid` payload becomes one line with its Explain, the org by its name
// (lib/names) and the payday by its day (lib/when).

import type { EventRef } from "../api/client";
import type { LedgerRow } from "../components/Ledger";
import type { Explain } from "../components/Num";
import { dayOf } from "./when";

export type OrgNamer = (org: number) => string;

export function payslipRows(slips: EventRef[], name: OrgNamer, limit?: number): LedgerRow[] {
  const recent = limit === undefined ? slips.slice() : slips.slice(-limit);
  return recent.reverse().map((s) => {
    const p = (s.payload.Paid ?? {}) as Record<string, unknown>;
    return {
      key: String(s.seq),
      when: dayOf(s.cycle),
      what: `Payslip, ${name(Number(p.org))}`,
      cents: Number(p.amount ?? 0),
      explain: (p.explain as Explain | undefined) ?? null,
    };
  });
}
