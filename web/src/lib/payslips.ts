// Payslip events as ledger rows (S1.8, shared by Home and Work in S1.9): a
// `Paid` payload becomes one line with its Explain; the org is named when
// the caller knows it (allocations and employment contracts carry names).

import type { EventRef } from "../api/client";
import type { LedgerRow } from "../components/Ledger";
import type { Explain } from "../components/Num";

export type OrgNamer = (org: number) => string;

/** Org names from what the labor view already carries; S1.11 adds the org screens. */
export function orgNamer(
  allocations: { org: number; org_name: string }[],
  employment: { body: Record<string, unknown> }[],
): OrgNamer {
  const names = new Map<number, string>();
  for (const a of allocations) names.set(a.org, a.org_name);
  return (org) => {
    const known = names.get(org);
    if (known) return known;
    const held = employment.some((k) => (k.body.employment as Record<string, unknown> | undefined)?.org === org);
    return held ? `your employer, org #${org}` : `org #${org}`;
  };
}

export function payslipRows(slips: EventRef[], name: OrgNamer, limit?: number): LedgerRow[] {
  const recent = limit === undefined ? slips.slice() : slips.slice(-limit);
  return recent.reverse().map((s) => {
    const p = (s.payload.Paid ?? {}) as Record<string, unknown>;
    return {
      key: String(s.seq),
      when: `c${s.cycle + 1} end`,
      what: `Payslip, ${name(Number(p.org))}`,
      cents: Number(p.amount ?? 0),
      explain: (p.explain as Explain | undefined) ?? null,
    };
  });
}
