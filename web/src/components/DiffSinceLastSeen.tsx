// What the plan did while you were away (GDD 9.3): the digest's events as
// one line each, in the society's words, newest last.

import { credits, type EventRef } from "../api/client";
import { Ledger, type LedgerRow } from "./Ledger";

type T = (key: string) => string;

function payload(e: EventRef): Record<string, unknown> {
  const p = e.payload as Record<string, unknown>;
  const inner = p[e.kind];
  return (inner && typeof inner === "object" ? inner : p) as Record<string, unknown>;
}

function money(v: unknown): number | undefined {
  return typeof v === "number" ? v : undefined;
}

/** One ledger row per event kind we know how to narrate; others by name. */
export function describe(e: EventRef, t: T, me?: number): LedgerRow {
  const p = payload(e);
  const when = `c${e.cycle + 1} t${(e.tick % 24) + 1}`;
  const base = { key: `${e.seq}`, when, explain: (p.explain as LedgerRow["explain"]) ?? null };
  switch (e.kind) {
    case "Paid":
      return { ...base, what: t("compensation"), cents: money(p.amount) };
    case "RentPaid":
      return { ...base, what: "Rent", cents: -(money(p.amount) ?? 0) };
    case "Trade": {
      const bought = me !== undefined && JSON.stringify(p.buyer) === JSON.stringify({ citizen: me });
      const qty = money(p.qty) ?? 0;
      const price = money(p.price) ?? 0;
      const inst = typeof p.instrument === "object" && p.instrument ? Object.values(p.instrument as object)[0] : p.instrument;
      return {
        ...base,
        what: `${bought ? "Bought" : "Sold"} ${qty} ${String(inst)} @ ${credits(price)}`,
        cents: bought ? -qty * price : qty * price,
      };
    }
    case "Transferred":
      return { ...base, what: `Transfer: ${String(p.memo ?? "")}`, cents: money(p.amount) };
    case "DividendPaid":
      return { ...base, what: "Dividend", cents: money(p.amount) };
    case "CreditInstallment":
      return { ...base, what: "Loan installment", cents: -(money(p.amount) ?? 0) };
    case "HardshipBegan":
      return { ...base, what: "Hardship began" };
    case "HardshipEnded":
      return { ...base, what: "Hardship ended" };
    case "EmploymentAccepted":
      return { ...base, what: `${t("job")} accepted` };
    case "EmploymentTerminated":
      return { ...base, what: `${t("job")} ended` };
    case "OrderPlaced":
      return { ...base, what: "Standing order placed" };
    case "OrderExpired":
      return { ...base, what: "Order expired" };
    case "OrderCancelled":
      return { ...base, what: "Order cancelled" };
    default:
      return { ...base, what: e.kind.replace(/([a-z])([A-Z])/g, "$1 $2") };
  }
}

export function DiffSinceLastSeen({ events, t, me }: { events: EventRef[]; t: T; me?: number }) {
  const rows = events
    .filter((e) => e.kind !== "TickResolved" && e.kind !== "CitizenSeen")
    .map((e) => describe(e, t, me));
  return <Ledger rows={rows} empty="Nothing happened to you while you were away." />;
}
