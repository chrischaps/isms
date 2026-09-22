// What the plan did while you were away (GDD 9.3): the digest's events as
// one line each, in the society's words, newest last. The digest carries the
// assembly's news too (S2.5 `concerns`): a ballot the plan cast by default,
// a proposal closed, an honor, a seat taken or lost, an election opened — each
// its own line here rather than a bare event name (S2.11).

import { credits, type EventRef } from "../api/client";
import { fieldName } from "../lib/policy";
import { whenOf } from "../lib/when";
import { Ledger, type LedgerRow } from "./Ledger";

type T = (key: string) => string;

/** "#3" for a 0-based proposal id, as the Assembly numbers them. */
function motion(p: unknown): string {
  return `#${String(p)}`;
}

/** "term ended", "absence", "recalled" for a `VacancyReason`. */
function vacancy(reason: unknown): string {
  switch (reason) {
    case "term_ended":
      return "the term ended";
    case "absence":
      return "absence";
    case "recalled":
      return "recalled";
    default:
      return String(reason ?? "");
  }
}

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
  const when = whenOf(e);
  const base = { key: `${e.seq}`, epoch: e.epoch, when, explain: (p.explain as LedgerRow["explain"]) ?? null };
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
    case "Drew": {
      const share = (p.explain as { rule?: string } | undefined)?.rule === "store_surplus_share";
      const goods = Object.entries((p.goods ?? {}) as Record<string, number>)
        .map(([g, n]) => `${n} ${g}`)
        .join(", ");
      return { ...base, what: share ? "Surplus share from the Store" : "Drew from the Store", goods: `+${goods}` };
    }
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
    // The assembly's news (S2.5 `concerns`, S2.11).
    case "Voted": {
      const how = String(p.ballot ?? "");
      return { ...base, what: `${p.by_default ? "Your plan voted" : "You voted"} ${how} on ${t("proposal").toLowerCase()} ${motion(p.proposal)}${p.by_default ? " by default" : ""}` };
    }
    case "ProposalClosed": {
      const tally = (p.tally ?? {}) as Record<string, number>;
      const count = tally.yes !== undefined ? ` (${tally.yes} yes, ${tally.no} no, ${tally.cast} of ${tally.eligible} cast)` : "";
      return { ...base, what: `${t("proposal")} ${motion(p.proposal)} ${p.passed ? "carried" : "failed"}${count}` };
    }
    case "Honored":
      return { ...base, what: `The assembly honored you (${t("proposal").toLowerCase()} ${motion(p.proposal)})` };
    case "OfficeTaken":
      return { ...base, what: `You took a seat as ${fieldName(String(p.office))}, through Day ${Number(p.term_ends_cycle) + 1}` };
    case "OfficeVacated":
      return { ...base, what: `Your seat as ${fieldName(String(p.office))} emptied: ${vacancy(p.reason)}` };
    case "ElectionOpened":
      return { ...base, what: `An election opened for ${fieldName(String(p.office))}, ${String(p.seats)} ${Number(p.seats) === 1 ? "seat" : "seats"}, closing at the end of Day ${Number(p.closes_cycle) + 1}` };
    case "Disbursed": {
      const asset = (p.asset ?? {}) as Record<string, unknown>;
      if (typeof asset.money === "number") return { ...base, what: "Disbursed to you by a members' vote", cents: asset.money };
      // The engine's `Asset::Good(good, qty)` on the wire: `{"good": ["food", 3]}`.
      const good = asset.good as [string, number] | undefined;
      return { ...base, what: "Disbursed to you by a members' vote", goods: good ? `+${good[1]} ${good[0]}` : undefined };
    }
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
