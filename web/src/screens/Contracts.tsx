// Contracts, housing, transfers and the notice board (GDD 7.2, 7.3, 6.1;
// S1.12). Housing first (a dwelling or the leases on offer), then the
// board's typed ads with accept and withdraw, the forms to post credit, a
// sale, a wanted ad, a lease, or a transfer, and your contracts with their
// status and termination. Money forms mount only where money exists;
// each contract kind mounts on `capabilities.contracts`.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits, type OfferView } from "../api/client";
import { useBoard, useCapabilities, useHome, useLexicon } from "../api/hooks";
import { useCitizens } from "../api/civic";
import { useOrgs } from "../api/market";
import { useCancelOffer, useOfferSale } from "../api/orgs";
import { useAcceptOffer } from "../api/society";
import {
  useContracts,
  useMoveOut,
  useOfferCredit,
  useOfferLease,
  usePostWanted,
  useTerminate,
  useTransfer,
  type ContractView,
} from "../api/contracts";
import { Meter } from "../components/Meter";
import { useNames, type Names } from "../lib/names";
import { needHints } from "../lib/needs";
import { jobLine } from "../lib/offers";
import { whenOfTick } from "../lib/when";

const GOODS = ["grain", "ore", "materials", "food", "wares", "machines"];
type Party = { citizen: number } | { org: number };

function isMe(p: unknown, me: number): boolean {
  return typeof (p as Record<string, unknown> | undefined)?.citizen === "number" && (p as { citizen: number }).citizen === me;
}

const toCents = (s: string) => Math.round(Number(s) * 100) || 0;

/** The engine's schedule: total = principal + principal x rate x term; installment = floor(total / term). */
function schedule(principal: number, bp: number, term: number) {
  const interest = Math.floor((principal * bp * term) / 10_000);
  const total = principal + interest;
  const n = Math.max(1, term);
  const installment = Math.floor(total / n);
  return { total, installment, last: total - installment * (n - 1) };
}

function assetText(a: unknown, names: Names): string {
  const o = (a ?? {}) as Record<string, unknown>;
  if (Array.isArray(o.good)) return `${String(o.good[1])} ${String(o.good[0])}`;
  if (Array.isArray(o.shares)) return `${String(o.shares[1])} shares of ${names.org(Number(o.shares[0]))}`;
  if (o.dwelling !== undefined) return `dwelling no. ${String(o.dwelling)}`;
  if (o.workplace !== undefined) return `the ${names.workplace(Number(o.workplace))}`;
  if (typeof o.money === "number") return `${credits(o.money)} cr`;
  return JSON.stringify(a);
}

function priceText(p: unknown): string {
  const o = (p ?? {}) as Record<string, unknown>;
  if (typeof o.money === "number") return `${credits(o.money)} cr`;
  if (Array.isArray(o.good)) return `${String(o.good[1])} ${String(o.good[0])}`;
  return JSON.stringify(p);
}

function contractLine(k: ContractView, me: number, names: Names) {
  const body = k.body as Record<string, Record<string, unknown>>;
  const parties = k.parties as unknown as [Party, Party];
  const other = isMe(parties[0], me) ? parties[1] : parties[0];
  const who = names.party(other);
  if (body.employment) {
    const e = body.employment;
    const pay = e.pay as Record<string, number>;
    return {
      kind: "Employment",
      text: `with ${who}: ${pay.hourly !== undefined ? `${credits(pay.hourly)} cr/h` : `${credits(pay.piece_rate ?? 0)} cr/unit`}, up to ${String(e.max_hours)} h a day, notice ${String(e.notice_cycles)} day(s)`,
      endable: true,
    };
  }
  if (body.credit) {
    const c = body.credit;
    const lender = isMe(parties[0], me);
    return {
      kind: lender ? "Loan out" : "Loan",
      text: `${lender ? "to" : "from"} ${who}: ${credits(Number(c.principal))} cr at ${(Number(c.rate_per_cycle_bp) / 100).toFixed(2)}% a day, ${credits(Number(c.installment))} cr a day, ${String(c.installments_left)} installment(s) left${c.collateral ? `, collateral ${assetText(c.collateral, names)}` : ""}${c.missed ? ", an installment missed" : ""}`,
      endable: false,
    };
  }
  if (body.lease) {
    const l = body.lease;
    const owner = isMe(parties[0], me);
    return {
      kind: owner ? "Lease out" : "Lease",
      text: `${owner ? "to" : "from"} ${who}: ${assetText(l.asset, names)} at ${credits(Number(l.rent_per_cycle))} cr a day${Number(l.missed_cycles) > 0 ? `, ${String(l.missed_cycles)} day(s) unpaid` : ""}`,
      endable: true,
    };
  }
  const kind = Object.keys(body)[0] ?? "contract";
  return { kind: kind.replace("_", " "), text: `with ${who}: ${JSON.stringify(body[kind])}`, endable: false };
}

export function Contracts({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const board = useBoard(id);
  const contracts = useContracts(id);
  const orgs = useOrgs(id);
  const citizens = useCitizens(id);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const accept = useAcceptOffer(id);
  const withdraw = useCancelOffer(id);
  const offerCredit = useOfferCredit(id);
  const offerSale = useOfferSale(id);
  const offerLease = useOfferLease(id);
  const postWanted = usePostWanted(id);
  const transfer = useTransfer(id);
  const terminate = useTerminate(id);
  const moveOut = useMoveOut(id);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [credit, setCredit] = useState({ principal: "100.00", rate: "1.00", term: 4, to: "", collateralKind: "none", collateralId: "", collateralQty: 1 });
  const [sale, setSale] = useState({ good: "food", qty: 1, price: "", to: "" });
  const [wanted, setWanted] = useState({ good: "food", qty: 1, max: "" });
  const [lease, setLease] = useState({ dwelling: "", rent: "8.00", term: "" });
  const [gift, setGift] = useState({ toKind: "citizen", to: "", kind: "money", amount: "", good: "food", qty: 1, memo: "" });

  if (home.isPending || board.isPending || contracts.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }} className="underline">
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error || board.error || contracts.error || caps.error) {
    return <p className="text-bad">Could not load: {String(home.error ?? board.error ?? contracts.error ?? caps.error)}</p>;
  }
  const h = home.data!;
  const c = caps.data!;
  const me = h.citizen.id;
  const others = (citizens.data?.citizens ?? []).filter((z) => z.id !== me).sort((a, b) => a.handle.localeCompare(b.handle));
  const offers = board.data!.offers;
  const enabled = new Set(c.contracts);
  const fail = (e: Error) => {
    setNote(null);
    setError(e.message);
  };
  const ok = (what: string) => () => {
    setError(null);
    setNote(what);
  };
  const byMe = (o: OfferView) => isMe(o.by, me);
  const leases = offers.filter((o) => o.kind === "lease");
  const ads = offers.filter((o) => o.kind !== "employment" && o.kind !== "lease" && o.kind !== "membership");
  const dwelling = h.household.dwelling;
  const myLease = dwelling?.lease != null ? contracts.data!.contracts.find((k) => k.id === dwelling.lease) : undefined;
  const sched = schedule(toCents(credit.principal), Math.round(Number(credit.rate) * 100), credit.term);

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">Contracts</h2>
        <p className="text-muted text-sm">The engine enforces every one of these; nobody else does.</p>
      </header>
      {error ? (
        <p className="text-bad text-sm" role="alert">
          {error}
        </p>
      ) : null}
      {note ? <p className="text-muted text-sm">{note}</p> : null}

      <section data-testid="housing">
        <h3 className="text-lg">{t("dwelling")}</h3>
        <div className="mt-2 max-w-md">
          <Meter label="Shelter" value={h.needs.shelter} hint={needHints(t, c).shelter} />
        </div>
        {dwelling ? (
          <p className="mt-2 text-sm" data-testid="my-dwelling">
            Dwelling no. {dwelling.id}, owned by {names.party(dwelling.owner)}
            {dwelling.rent_per_cycle != null ? `, ${credits(dwelling.rent_per_cycle)} cr a day` : ", yours"}.{" "}
            {myLease ? (
              <button type="button" className="text-muted underline" onClick={() => terminate.mutate(myLease.id, { onSuccess: ok("Lease ended."), onError: fail })}>
                End the lease
              </button>
            ) : (
              <button type="button" className="text-muted underline" onClick={() => moveOut.mutate(dwelling.id, { onSuccess: ok("Moved out."), onError: fail })}>
                Move out
              </button>
            )}
          </p>
        ) : (
          <div className="mt-2 text-sm">
            <p className="text-muted">No dwelling: Shelter falls every hour until you rent or buy one.</p>
            {leases.length === 0 ? (
              <p className="text-muted mt-1">Nothing to let on the board.</p>
            ) : (
              <table className="mt-2 w-full" data-testid="leases">
                <thead className="text-muted text-left text-xs uppercase tracking-wide">
                  <tr>
                    <th className="py-1 font-normal">Dwelling</th>
                    <th className="py-1 font-normal">Landlord</th>
                    <th className="py-1 text-right font-normal">Rent a day</th>
                    <th className="py-1 font-normal">Term</th>
                    <th className="py-1 font-normal" />
                  </tr>
                </thead>
                <tbody>
                  {leases.map((o) => {
                    const l = (o.body as Record<string, unknown>).lease as Record<string, unknown>;
                    return (
                      <tr key={o.id} className="rule">
                        <td className="py-1 pr-2">{assetText(l.asset, names)}</td>
                        <td className="py-1 pr-2">{names.party(o.by)}</td>
                        <td className="num py-1 pr-2 text-right">{credits(Number(l.rent_per_cycle))} cr</td>
                        <td className="py-1 pr-2">{l.term_cycles == null ? "open" : `${String(l.term_cycles)} cycles`}</td>
                        <td className="py-1 text-right">
                          {byMe(o) ? (
                            <button type="button" className="text-muted text-xs underline" onClick={() => withdraw.mutate(o.id, { onError: fail })}>
                              withdraw
                            </button>
                          ) : (
                            <button type="button" disabled={accept.isPending} className="border-line rounded-sm border px-2 py-0.5 text-xs" onClick={() => accept.mutate(o.id, { onSuccess: ok("Rented. Shelter recovers from the next hour."), onError: fail })}>
                              Rent it
                            </button>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </div>
        )}
      </section>

      <section>
        <h3 className="text-lg">Notice board</h3>
        <p className="text-muted mt-1 text-xs">
          Typed ads. {t("job")}s are on the{" "}
          <Link to="/s/$id/orgs" params={{ id: String(id) }} className="underline">
            Organizations
          </Link>{" "}
          screen; dwellings to let are above.
        </p>
        {ads.length === 0 ? (
          <p className="text-muted mt-2 text-sm">Nothing posted.</p>
        ) : (
          <table className="mt-2 w-full text-sm" data-testid="ads">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">Kind</th>
                <th className="py-1 font-normal">Terms</th>
                <th className="py-1 font-normal">By</th>
                <th className="py-1 font-normal" />
              </tr>
            </thead>
            <tbody>
              {ads.map((o) => {
                const b = o.body as Record<string, Record<string, unknown>>;
                let terms = "";
                let action: string | null = null;
                if (b.sale) {
                  terms = `${assetText(b.sale.asset, names)} for ${priceText(b.sale.price)}${b.sale.to ? `, for ${names.party(b.sale.to)} only` : ""}`;
                  action = "Buy";
                } else if (b.wanted) {
                  terms = `${String(b.wanted.qty)} ${String(b.wanted.good)} at up to ${credits(Number(b.wanted.max_price))} cr each; answer it with a sale offer addressed to the poster`;
                } else if (b.credit) {
                  const s = schedule(Number(b.credit.principal), Number(b.credit.rate_per_cycle_bp), Number(b.credit.term_cycles));
                  terms = `${credits(Number(b.credit.principal))} cr at ${(Number(b.credit.rate_per_cycle_bp) / 100).toFixed(2)}% a day over ${String(b.credit.term_cycles)} days: ${credits(s.installment)} cr a day, ${credits(s.total)} cr in all${b.credit.collateral ? `, against ${assetText(b.credit.collateral, names)}` : ""}${b.credit.to ? `, for ${names.party(b.credit.to)} only` : ""}`;
                  action = "Borrow";
                } else {
                  terms = JSON.stringify(o.body);
                }
                const jl = jobLine(o);
                if (jl) terms = `${jl.pay}, up to ${jl.hours} h`;
                return (
                  <tr key={o.id} className="rule align-top">
                    <td className="py-1 pr-2">{o.kind.replace("_", " ")}</td>
                    <td className="py-1 pr-2">{terms}</td>
                    <td className="py-1 pr-2">{names.party(o.by)}</td>
                    <td className="py-1 text-right whitespace-nowrap">
                      {byMe(o) ? (
                        <button type="button" className="text-muted text-xs underline" onClick={() => withdraw.mutate(o.id, { onSuccess: ok("Withdrawn."), onError: fail })}>
                          withdraw
                        </button>
                      ) : action ? (
                        <button type="button" disabled={accept.isPending} className="border-line rounded-sm border px-2 py-0.5 text-xs" onClick={() => accept.mutate(o.id, { onSuccess: ok(`${action === "Buy" ? "Bought" : "Borrowed"}.`), onError: fail })}>
                          {action}
                        </button>
                      ) : null}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>

      <section className="grid gap-8 md:grid-cols-2 text-sm">
        {c.money && enabled.has("credit") ? (
          <form
            className="flex flex-col gap-2"
            data-testid="credit-form"
            onSubmit={(e) => {
              e.preventDefault();
              const collateral =
                credit.collateralKind === "dwelling"
                  ? { dwelling: Number(credit.collateralId) }
                  : credit.collateralKind === "shares"
                    ? { shares: [Number(credit.collateralId), credit.collateralQty] }
                    : null;
              offerCredit.mutate(
                {
                  principal: toCents(credit.principal),
                  rate_per_cycle_bp: Math.round(Number(credit.rate) * 100),
                  term_cycles: credit.term,
                  to: credit.to.trim() === "" ? null : ({ citizen: Number(credit.to) } as never),
                  collateral: collateral as never,
                },
                { onSuccess: ok("Loan offered; the principal is in escrow until someone takes it."), onError: fail },
              );
            }}
          >
            <h3 className="text-lg">Offer a loan</h3>
            <label className="flex items-center gap-2">
              <span className="w-28">Principal</span>
              <input aria-label="Principal" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={credit.principal} onChange={(e) => setCredit({ ...credit, principal: e.target.value })} />
              <span className="text-muted text-xs">cr</span>
            </label>
            <label className="flex items-center gap-2">
              <span className="w-28">Rate a day</span>
              <input aria-label="Rate" type="number" step="0.01" min={0} className="border-line num w-20 rounded-sm border px-1" value={credit.rate} onChange={(e) => setCredit({ ...credit, rate: e.target.value })} />
              <span className="text-muted text-xs">%</span>
            </label>
            <label className="flex items-center gap-2">
              <span className="w-28">Term</span>
              <input aria-label="Term" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={credit.term} onChange={(e) => setCredit({ ...credit, term: Math.max(1, Number(e.target.value)) })} />
              <span className="text-muted text-xs">days</span>
            </label>
            <label className="flex items-center gap-2">
              <span className="w-28">To</span>
              <select aria-label="Borrower" className="border-line rounded-sm border px-1" value={credit.to} onChange={(e) => setCredit({ ...credit, to: e.target.value })}>
                <option value="">anyone</option>
                {others.map((z) => (
                  <option key={z.id} value={z.id}>
                    {z.handle}
                  </option>
                ))}
              </select>
              <span className="text-muted text-xs">one citizen, or an open offer</span>
            </label>
            <label className="flex items-center gap-2">
              <span className="w-28">Collateral</span>
              <select aria-label="Collateral" className="border-line rounded-sm border px-1" value={credit.collateralKind} onChange={(e) => setCredit({ ...credit, collateralKind: e.target.value, collateralId: "" })}>
                <option value="none">none</option>
                <option value="dwelling">a dwelling</option>
                <option value="shares">shares</option>
              </select>
              {credit.collateralKind !== "none" ? (
                credit.collateralKind === "dwelling" ? (
                  <input aria-label="Collateral id" type="number" min={1} placeholder="dwelling no." className="border-line num w-28 rounded-sm border px-1" value={credit.collateralId} onChange={(e) => setCredit({ ...credit, collateralId: e.target.value })} />
                ) : (
                  <select aria-label="Collateral id" className="border-line rounded-sm border px-1" value={credit.collateralId} onChange={(e) => setCredit({ ...credit, collateralId: e.target.value })}>
                    <option value="">which organization</option>
                    {(orgs.data ?? []).map((o) => (
                      <option key={o.id} value={o.id}>
                        {o.name}
                      </option>
                    ))}
                  </select>
                )
              ) : null}
              {credit.collateralKind === "shares" ? (
                <input aria-label="Collateral shares" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={credit.collateralQty} onChange={(e) => setCredit({ ...credit, collateralQty: Number(e.target.value) })} />
              ) : null}
            </label>
            <p className="bg-paper-2 rounded-sm p-2 text-xs" data-testid="schedule">
              Repaid as {credit.term} installment(s) of {credits(sched.installment)} cr{sched.last !== sched.installment ? ` (the last ${credits(sched.last)} cr)` : ""} at the end of each day, {credits(sched.total)} cr in all. A missed installment seizes the collateral, then flags the borrower.
            </p>
            <button type="submit" disabled={offerCredit.isPending || toCents(credit.principal) < 1} className="bg-ink text-paper self-start rounded-sm px-3 py-1 disabled:opacity-50">
              Offer loan
            </button>
          </form>
        ) : null}

        <div className="flex flex-col gap-6">
          <form
            className="flex flex-col gap-2"
            data-testid="sale-form"
            onSubmit={(e) => {
              e.preventDefault();
              offerSale.mutate(
                { asset: { good: [sale.good, sale.qty] }, price: { money: toCents(sale.price) }, ...(sale.to.trim() === "" ? {} : { to: { citizen: Number(sale.to) } }) } as never,
                { onSuccess: ok("Sale offered; the goods are in escrow."), onError: fail },
              );
            }}
          >
            <h3 className="text-lg">Offer goods for sale</h3>
            <div className="flex flex-wrap items-center gap-2">
              <input aria-label="Sale quantity" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={sale.qty} onChange={(e) => setSale({ ...sale, qty: Number(e.target.value) })} />
              <select aria-label="Sale good" className="border-line rounded-sm border px-1" value={sale.good} onChange={(e) => setSale({ ...sale, good: e.target.value })}>
                {GOODS.map((g) => (
                  <option key={g} value={g}>
                    {g} ({(h.household.pantry as Record<string, number>)[g] ?? 0} held)
                  </option>
                ))}
              </select>
              <span>for</span>
              <input aria-label="Sale price" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={sale.price} onChange={(e) => setSale({ ...sale, price: e.target.value })} />
              <span className="text-muted text-xs">cr the lot</span>
              <span>to</span>
              <select aria-label="Sale to" className="border-line rounded-sm border px-1" value={sale.to} onChange={(e) => setSale({ ...sale, to: e.target.value })}>
                <option value="">anyone</option>
                {others.map((z) => (
                  <option key={z.id} value={z.id}>
                    {z.handle}
                  </option>
                ))}
              </select>
              <button type="submit" disabled={offerSale.isPending || toCents(sale.price) < 1} className="border-line rounded-sm border px-2 py-0.5">
                Offer
              </button>
            </div>
          </form>

          <form
            className="flex flex-col gap-2"
            data-testid="wanted-form"
            onSubmit={(e) => {
              e.preventDefault();
              postWanted.mutate({ good: wanted.good, qty: wanted.qty, max_price: toCents(wanted.max) }, { onSuccess: ok("Wanted ad posted."), onError: fail });
            }}
          >
            <h3 className="text-lg">Post a wanted ad</h3>
            <div className="flex flex-wrap items-center gap-2">
              <input aria-label="Wanted quantity" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={wanted.qty} onChange={(e) => setWanted({ ...wanted, qty: Number(e.target.value) })} />
              <select aria-label="Wanted good" className="border-line rounded-sm border px-1" value={wanted.good} onChange={(e) => setWanted({ ...wanted, good: e.target.value })}>
                {GOODS.map((g) => (
                  <option key={g} value={g}>
                    {g}
                  </option>
                ))}
              </select>
              <span>at up to</span>
              <input aria-label="Wanted price" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={wanted.max} onChange={(e) => setWanted({ ...wanted, max: e.target.value })} />
              <span className="text-muted text-xs">cr each</span>
              <button type="submit" disabled={postWanted.isPending || toCents(wanted.max) < 1} className="border-line rounded-sm border px-2 py-0.5">
                Post
              </button>
            </div>
          </form>

          {enabled.has("lease") ? (
            <form
              className="flex flex-col gap-2"
              data-testid="lease-form"
              onSubmit={(e) => {
                e.preventDefault();
                offerLease.mutate(
                  { asset: { dwelling: Number(lease.dwelling) } as never, rent_per_cycle: toCents(lease.rent), term_cycles: lease.term.trim() === "" ? null : Number(lease.term) },
                  { onSuccess: ok("Dwelling offered to let."), onError: fail },
                );
              }}
            >
              <h3 className="text-lg">Let a dwelling you own</h3>
              <div className="flex flex-wrap items-center gap-2">
                <span>Dwelling no.</span>
                <input aria-label="Lease dwelling" type="number" min={1} className="border-line num w-20 rounded-sm border px-1" value={lease.dwelling} onChange={(e) => setLease({ ...lease, dwelling: e.target.value })} />
                <span>at</span>
                <input aria-label="Lease rent" type="number" step="0.01" min={0} className="border-line num w-24 rounded-sm border px-1" value={lease.rent} onChange={(e) => setLease({ ...lease, rent: e.target.value })} />
                <span className="text-muted text-xs">cr a day, term in days</span>
                <input aria-label="Lease term" type="number" min={1} placeholder="open" className="border-line num w-16 rounded-sm border px-1" value={lease.term} onChange={(e) => setLease({ ...lease, term: e.target.value })} />
                <button type="submit" disabled={offerLease.isPending || lease.dwelling.trim() === ""} className="border-line rounded-sm border px-2 py-0.5">
                  Offer
                </button>
              </div>
            </form>
          ) : null}

          <form
            className="flex flex-col gap-2"
            data-testid="transfer-form"
            onSubmit={(e) => {
              e.preventDefault();
              const to = gift.toKind === "citizen" ? { citizen: Number(gift.to) } : { org: Number(gift.to) };
              const asset = gift.kind === "money" ? { money: toCents(gift.amount) } : { good: [gift.good, gift.qty] };
              transfer.mutate({ to: to as never, asset: asset as never, memo: gift.memo }, { onSuccess: ok("Sent."), onError: fail });
            }}
          >
            <h3 className="text-lg">Transfer</h3>
            <p className="text-muted text-xs">A gift, dues, alms, or a side-payment: all transfers, told apart by the memo.</p>
            <div className="flex flex-wrap items-center gap-2">
              <span>To</span>
              <select aria-label="Transfer to kind" className="border-line rounded-sm border px-1" value={gift.toKind} onChange={(e) => setGift({ ...gift, toKind: e.target.value, to: "" })}>
                <option value="citizen">a citizen</option>
                <option value="org">an organization</option>
              </select>
              <select aria-label="Transfer to" className="border-line rounded-sm border px-1" value={gift.to} onChange={(e) => setGift({ ...gift, to: e.target.value })}>
                <option value="">who</option>
                {(gift.toKind === "citizen" ? others.map((z) => ({ id: z.id, name: z.handle })) : (orgs.data ?? [])).map((z) => (
                  <option key={z.id} value={z.id}>
                    {z.name}
                  </option>
                ))}
              </select>
              {c.money ? (
                <select aria-label="Transfer kind" className="border-line rounded-sm border px-1" value={gift.kind} onChange={(e) => setGift({ ...gift, kind: e.target.value })}>
                  <option value="money">money</option>
                  <option value="good">goods</option>
                </select>
              ) : null}
              {gift.kind === "money" && c.money ? (
                <>
                  <input aria-label="Transfer amount" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={gift.amount} onChange={(e) => setGift({ ...gift, amount: e.target.value })} />
                  <span className="text-muted text-xs">cr</span>
                </>
              ) : (
                <>
                  <input aria-label="Transfer quantity" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={gift.qty} onChange={(e) => setGift({ ...gift, qty: Number(e.target.value) })} />
                  <select aria-label="Transfer good" className="border-line rounded-sm border px-1" value={gift.good} onChange={(e) => setGift({ ...gift, good: e.target.value })}>
                    {GOODS.map((g) => (
                      <option key={g} value={g}>
                        {g}
                      </option>
                    ))}
                  </select>
                </>
              )}
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <input aria-label="Memo" placeholder="memo" className="border-line w-64 rounded-sm border px-1" value={gift.memo} onChange={(e) => setGift({ ...gift, memo: e.target.value })} maxLength={80} />
              <button type="submit" disabled={transfer.isPending || gift.to.trim() === ""} className="border-line rounded-sm border px-2 py-0.5">
                Send
              </button>
            </div>
          </form>
        </div>
      </section>

      <section>
        <h3 className="text-lg">Your contracts</h3>
        {contracts.data!.contracts.length === 0 ? (
          <p className="text-muted mt-2 text-sm">None yet.</p>
        ) : (
          <table className="mt-2 w-full text-sm" data-testid="contracts">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">Kind</th>
                <th className="py-1 font-normal">Terms</th>
                <th className="py-1 font-normal">Status</th>
                <th className="py-1 font-normal" />
              </tr>
            </thead>
            <tbody>
              {contracts.data!.contracts.map((k) => {
                const line = contractLine(k, me, names);
                return (
                  <tr key={k.id} className="rule align-top" data-testid={`contract-${k.id}`}>
                    <td className="py-1 pr-2 whitespace-nowrap">
                      {line.kind}
                      {k.role === "manager" ? <span className="text-muted block text-xs">for your org</span> : null}
                    </td>
                    <td className="py-1 pr-2">
                      {line.text}
                      <span className="text-muted block text-xs">
                        since {whenOfTick(k.created_tick, h.clock.ticks_per_cycle)}
                        {k.term_cycles != null ? ` · term ${k.term_cycles} days` : ""}
                      </span>
                    </td>
                    <td className="py-1 pr-2">{k.status}</td>
                    <td className="py-1 text-right">
                      {line.endable && k.status === "active" ? (
                        <button type="button" disabled={terminate.isPending} className="text-muted text-xs underline" onClick={() => terminate.mutate(k.id, { onSuccess: ok("Ended; the notice rules apply."), onError: fail })}>
                          end
                        </button>
                      ) : null}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
