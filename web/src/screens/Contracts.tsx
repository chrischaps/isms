// Contracts, housing, transfers and the notice board (GDD 7.2, 7.3, 6.1;
// S1.12; docs/style.md §10 Contracts). Housing first: the Verdict, the
// Shelter NeedCard, your dwelling or the dwellings to let; then the board's
// typed ads with accept and withdraw, your contracts with their status and
// termination, the loan form, and the smaller forms — a sale, a wanted ad, a
// lease, a transfer — as disclosures. Money forms mount only where money
// exists; each contract kind mounts on `capabilities.contracts`.

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
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { Field, Input, Select } from "../components/Field";
import { More } from "../components/More";
import { NeedCard } from "../components/NeedCard";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill, type PillTone } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { useNames, type Names } from "../lib/names";
import { needHints, needStatus } from "../lib/needs";
import { jobLine } from "../lib/offers";
import { contractsVerdict } from "../lib/verdict";
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
      overdue: false,
    };
  }
  if (body.credit) {
    const c = body.credit;
    const lender = isMe(parties[0], me);
    return {
      kind: lender ? "Loan out" : "Loan",
      text: `${lender ? "to" : "from"} ${who}: ${credits(Number(c.principal))} cr at ${(Number(c.rate_per_cycle_bp) / 100).toFixed(2)}% a day, ${credits(Number(c.installment))} cr a day, ${String(c.installments_left)} installment(s) left${c.collateral ? `, collateral ${assetText(c.collateral, names)}` : ""}${c.missed ? ", an installment missed" : ""}`,
      endable: false,
      overdue: Boolean(c.missed) && !lender,
    };
  }
  if (body.lease) {
    const l = body.lease;
    const owner = isMe(parties[0], me);
    return {
      kind: owner ? "Lease out" : "Lease",
      text: `${owner ? "to" : "from"} ${who}: ${assetText(l.asset, names)} at ${credits(Number(l.rent_per_cycle))} cr a day${Number(l.missed_cycles) > 0 ? `, ${String(l.missed_cycles)} day(s) unpaid` : ""}`,
      endable: true,
      overdue: Number(l.missed_cycles) > 0 && !owner,
    };
  }
  const kind = Object.keys(body)[0] ?? "contract";
  return { kind: kind.replace("_", " "), text: `with ${who}: ${JSON.stringify(body[kind])}`, endable: false, overdue: false };
}

const STATUS_TONE: Record<string, PillTone> = { active: "good", ended: "neutral", terminated: "neutral", defaulted: "crit" };

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
        <Link to="/s/$id" params={{ id: String(id) }}>
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error || board.error || contracts.error || caps.error) {
    return <p className="text-crit">Could not load: {String(home.error ?? board.error ?? contracts.error ?? caps.error)}</p>;
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
  const mine = contracts.data!.contracts.map((k) => ({ k, line: contractLine(k, me, names) }));
  const active = mine.filter((x) => x.k.status === "active");
  const verdict = contractsVerdict({
    housed: dwelling != null,
    rent: dwelling?.rent_per_cycle ?? null,
    shelter: h.needs.shelter,
        toLet: leases.length,
    active: active.length,
    overdue: active.some((x) => x.line.overdue),
  });
  const pantry = h.household.pantry as Record<string, number>;
  const status = needStatus(t, {
    food: h.needs.food,
    shelter: h.needs.shelter,
    comfort: h.needs.comfort,
    hardship: Boolean((h.citizen.flags as Record<string, boolean>).in_hardship),
    housed: dwelling != null,
    pantryFood: pantry.food ?? 0,
    pantryWares: pantry.wares ?? 0,
  });
  const hints = needHints(t, c);

  return (
    <div>
      <PageHeader title="Contracts" meta={<span>The engine enforces every one of these; nobody else does.</span>} />
      {error ? (
        <p className="text-crit mb-4 text-sm" role="alert">
          {error}
        </p>
      ) : null}
      {note ? <p className="text-muted mb-4 text-sm">{note}</p> : null}

      <Stack>
        <Card title={t("dwelling")} icon="home" testId="housing">
          <Verdict parts={verdict} />
          <div className="grid gap-3 @min-[620px]:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
            <NeedCard label="Shelter" icon="home" value={h.needs.shelter} status={status.shelter} tone={status.shelterTone} note={hints.shelter} />
            <Tile className="grid content-start gap-2">
              {dwelling ? (
                <>
                  <p className="m-0" data-testid="my-dwelling">
                    Dwelling no. {dwelling.id}, owned by {names.party(dwelling.owner)}
                    {dwelling.rent_per_cycle != null ? `, ${credits(dwelling.rent_per_cycle)} cr a day` : ", yours"}.
                  </p>
                  <ButtonRow>
                    {myLease ? (
                      <Button variant="danger" onClick={() => terminate.mutate(myLease.id, { onSuccess: ok("Lease ended."), onError: fail })}>End the lease</Button>
                    ) : (
                      <Button variant="danger" onClick={() => moveOut.mutate(dwelling.id, { onSuccess: ok("Moved out."), onError: fail })}>Move out</Button>
                    )}
                  </ButtonRow>
                </>
              ) : (
                <>
                  <p className="text-muted m-0 text-sm">No dwelling: Shelter falls every hour until you rent or buy one.</p>
                  {leases.length === 0 ? (
                    <p className="text-muted m-0 text-sm">Nothing to let on the board.</p>
                  ) : (
                    <ul className="m-0 grid list-none gap-2 p-0" data-testid="leases">
                      {leases.map((o) => {
                        const l = (o.body as Record<string, unknown>).lease as Record<string, unknown>;
                        return (
                          <li key={o.id} className="border-line flex flex-wrap items-center justify-between gap-2 border-b pb-2 last:border-b-0 last:pb-0">
                            <span>
                              <span className="font-bold">{assetText(l.asset, names)}</span>
                              <span className="text-muted block text-sm">
                                {names.party(o.by)} · <span className="tabular-nums">{credits(Number(l.rent_per_cycle))} cr</span> a day ·{" "}
                                {l.term_cycles == null ? "open term" : `${String(l.term_cycles)} days`}
                              </span>
                            </span>
                            {byMe(o) ? (
                              <Button variant="quiet" inline onClick={() => withdraw.mutate(o.id, { onError: fail })}>
                                withdraw
                              </Button>
                            ) : (
                              <Button inline disabled={accept.isPending} onClick={() => accept.mutate(o.id, { onSuccess: ok("Rented. Shelter recovers from the next hour."), onError: fail })}>
                                Rent it
                              </Button>
                            )}
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </>
              )}
            </Tile>
          </div>
        </Card>

        <Two className="items-start">
          <Card
            title="Notice board"
            icon="contract"
            subtitle={
              <>
                Typed ads. {t("job")}s are on the{" "}
                <Link to="/s/$id/orgs" params={{ id: String(id) }}>
                  Organizations
                </Link>{" "}
                screen; dwellings to let are above.
              </>
            }
          >
            {ads.length === 0 ? (
              <p className="text-muted m-0">Nothing posted.</p>
            ) : (
              <ul className="m-0 grid list-none gap-3 p-0" data-testid="ads">
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
                    <li key={o.id}>
                      <Tile className="grid gap-2">
                        <div className="flex flex-wrap items-baseline justify-between gap-x-3">
                          <span className="font-bold">{o.kind.replace("_", " ")}</span>
                          <span className="text-muted text-sm">
                            by {names.party(o.by)}
                            {byMe(o) ? (
                              <Pill className="ml-2">
                                you
                              </Pill>
                            ) : null}
                          </span>
                        </div>
                        <p className="m-0 text-sm">{terms}</p>
                        {byMe(o) ? (
                          <Button variant="quiet" className="justify-self-start" onClick={() => withdraw.mutate(o.id, { onSuccess: ok("Withdrawn."), onError: fail })}>
                            withdraw
                          </Button>
                        ) : action ? (
                          <Button className="justify-self-start" disabled={accept.isPending} onClick={() => accept.mutate(o.id, { onSuccess: ok(`${action === "Buy" ? "Bought" : "Borrowed"}.`), onError: fail })}>
                            {action}
                          </Button>
                        ) : null}
                      </Tile>
                    </li>
                  );
                })}
              </ul>
            )}
          </Card>

          <Card title="Your contracts" icon="contract" aside={active.length > 0 ? <Pill tone="good">{active.length} active</Pill> : null}>
            {mine.length === 0 ? (
              <p className="text-muted m-0">None yet.</p>
            ) : (
              <ul className="m-0 grid list-none gap-3 p-0" data-testid="contracts">
                {mine.map(({ k, line }) => (
                  <li key={k.id}>
                    <Tile testId={`contract-${k.id}`} className="grid gap-1.5">
                      <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
                        <span className="font-bold">
                          {line.kind}
                          {k.role === "manager" ? <span className="text-muted text-sm font-normal"> for your org</span> : null}
                        </span>
                        <Pill tone={line.overdue ? "crit" : (STATUS_TONE[k.status] ?? "neutral")}>{line.overdue ? `${k.status}, overdue` : k.status}</Pill>
                      </div>
                      <p className="m-0 text-sm">{line.text}</p>
                      <p className="text-muted m-0 text-sm">
                        since {whenOfTick(k.created_tick, h.clock.ticks_per_cycle)}
                        {k.term_cycles != null ? ` · term ${k.term_cycles} days` : ""}
                      </p>
                      {line.endable && k.status === "active" ? (
                        <Button variant="danger" inline className="justify-self-start" disabled={terminate.isPending} onClick={() => terminate.mutate(k.id, { onSuccess: ok("Ended; the notice rules apply."), onError: fail })}>
                          End it
                        </Button>
                      ) : null}
                    </Tile>
                  </li>
                ))}
              </ul>
            )}
          </Card>
        </Two>

        <Two className="items-start">
          {c.money && enabled.has("credit") ? (
            <Card title="Offer a loan" icon="coin">
              <form
                className="grid gap-3"
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
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="Principal" unit="cr">
                    <Input aria-label="Principal" type="number" inputMode="decimal" step="0.01" min={0.01} value={credit.principal} onChange={(e) => setCredit({ ...credit, principal: e.target.value })} />
                  </Field>
                  <Field label="Rate a day" unit="%">
                    <Input aria-label="Rate" type="number" inputMode="decimal" step="0.01" min={0} value={credit.rate} onChange={(e) => setCredit({ ...credit, rate: e.target.value })} />
                  </Field>
                  <Field label="Term" unit="days">
                    <Input aria-label="Term" type="number" inputMode="numeric" min={1} value={credit.term} onChange={(e) => setCredit({ ...credit, term: Math.max(1, Number(e.target.value)) })} />
                  </Field>
                  <Field label="To" hint="One citizen, or an open offer.">
                    <Select aria-label="Borrower" value={credit.to} onChange={(e) => setCredit({ ...credit, to: e.target.value })}>
                      <option value="">anyone</option>
                      {others.map((z) => (
                        <option key={z.id} value={z.id}>
                          {z.handle}
                        </option>
                      ))}
                    </Select>
                  </Field>
                  <Field label="Collateral">
                    <Select aria-label="Collateral" value={credit.collateralKind} onChange={(e) => setCredit({ ...credit, collateralKind: e.target.value, collateralId: "" })}>
                      <option value="none">none</option>
                      <option value="dwelling">a dwelling</option>
                      <option value="shares">shares</option>
                    </Select>
                  </Field>
                  {credit.collateralKind === "dwelling" ? (
                    <Field label="Dwelling no.">
                      <Input aria-label="Collateral id" type="number" inputMode="numeric" min={1} value={credit.collateralId} onChange={(e) => setCredit({ ...credit, collateralId: e.target.value })} />
                    </Field>
                  ) : null}
                  {credit.collateralKind === "shares" ? (
                    <>
                      <Field label="Which organization">
                        <Select aria-label="Collateral id" value={credit.collateralId} onChange={(e) => setCredit({ ...credit, collateralId: e.target.value })}>
                          <option value="">choose</option>
                          {(orgs.data ?? []).map((o) => (
                            <option key={o.id} value={o.id}>
                              {o.name}
                            </option>
                          ))}
                        </Select>
                      </Field>
                      <Field label="Shares" unit="shares">
                        <Input aria-label="Collateral shares" type="number" inputMode="numeric" min={1} value={credit.collateralQty} onChange={(e) => setCredit({ ...credit, collateralQty: Number(e.target.value) })} />
                      </Field>
                    </>
                  ) : null}
                </div>
                <Tile testId="schedule" className="text-sm">
                  Repaid as {credit.term} installment(s) of <b className="tabular-nums">{credits(sched.installment)} cr</b>
                  {sched.last !== sched.installment ? ` (the last ${credits(sched.last)} cr)` : ""} at the end of each day, {credits(sched.total)} cr in all. A missed installment seizes the
                  collateral, then flags the borrower.
                </Tile>
                <ButtonRow>
                  <Button type="submit" variant="primary" disabled={offerCredit.isPending} disabledReason={toCents(credit.principal) < 1 ? "set a principal" : undefined}>
                    Offer loan
                  </Button>
                </ButtonRow>
              </form>
            </Card>
          ) : null}

          <Card title="Post to the board" icon="contract" subtitle="A sale, a wanted ad, a dwelling to let, or a transfer.">
            <More summary="Offer goods for sale" testId="sale-more">
              <form
                className="grid gap-3"
                data-testid="sale-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  offerSale.mutate(
                    { asset: { good: [sale.good, sale.qty] }, price: { money: toCents(sale.price) }, ...(sale.to.trim() === "" ? {} : { to: { citizen: Number(sale.to) } }) } as never,
                    { onSuccess: ok("Sale offered; the goods are in escrow."), onError: fail },
                  );
                }}
              >
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="Good">
                    <Select aria-label="Sale good" value={sale.good} onChange={(e) => setSale({ ...sale, good: e.target.value })}>
                      {GOODS.map((g) => (
                        <option key={g} value={g}>
                          {g} ({pantry[g] ?? 0} held)
                        </option>
                      ))}
                    </Select>
                  </Field>
                  <Field label="Quantity" unit={sale.good}>
                    <Input aria-label="Sale quantity" type="number" inputMode="numeric" min={1} value={sale.qty} onChange={(e) => setSale({ ...sale, qty: Number(e.target.value) })} />
                  </Field>
                  <Field label="Price" unit="cr the lot">
                    <Input aria-label="Sale price" type="number" inputMode="decimal" step="0.01" min={0.01} value={sale.price} onChange={(e) => setSale({ ...sale, price: e.target.value })} />
                  </Field>
                  <Field label="To">
                    <Select aria-label="Sale to" value={sale.to} onChange={(e) => setSale({ ...sale, to: e.target.value })}>
                      <option value="">anyone</option>
                      {others.map((z) => (
                        <option key={z.id} value={z.id}>
                          {z.handle}
                        </option>
                      ))}
                    </Select>
                  </Field>
                </div>
                <ButtonRow>
                  <Button type="submit" disabled={offerSale.isPending} disabledReason={toCents(sale.price) < 1 ? "set a price" : undefined}>
                    Offer
                  </Button>
                </ButtonRow>
              </form>
            </More>

            <More summary="Post a wanted ad" testId="wanted-more">
              <form
                className="grid gap-3"
                data-testid="wanted-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  postWanted.mutate({ good: wanted.good, qty: wanted.qty, max_price: toCents(wanted.max) }, { onSuccess: ok("Wanted ad posted."), onError: fail });
                }}
              >
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="Good">
                    <Select aria-label="Wanted good" value={wanted.good} onChange={(e) => setWanted({ ...wanted, good: e.target.value })}>
                      {GOODS.map((g) => (
                        <option key={g} value={g}>
                          {g}
                        </option>
                      ))}
                    </Select>
                  </Field>
                  <Field label="Quantity" unit={wanted.good}>
                    <Input aria-label="Wanted quantity" type="number" inputMode="numeric" min={1} value={wanted.qty} onChange={(e) => setWanted({ ...wanted, qty: Number(e.target.value) })} />
                  </Field>
                  <Field label="At up to" unit="cr each">
                    <Input aria-label="Wanted price" type="number" inputMode="decimal" step="0.01" min={0.01} value={wanted.max} onChange={(e) => setWanted({ ...wanted, max: e.target.value })} />
                  </Field>
                </div>
                <ButtonRow>
                  <Button type="submit" disabled={postWanted.isPending} disabledReason={toCents(wanted.max) < 1 ? "set a price" : undefined}>
                    Post
                  </Button>
                </ButtonRow>
              </form>
            </More>

            {enabled.has("lease") ? (
              <More summary="Let a dwelling you own" testId="lease-more">
                <form
                  className="grid gap-3"
                  data-testid="lease-form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    offerLease.mutate(
                      { asset: { dwelling: Number(lease.dwelling) } as never, rent_per_cycle: toCents(lease.rent), term_cycles: lease.term.trim() === "" ? null : Number(lease.term) },
                      { onSuccess: ok("Dwelling offered to let."), onError: fail },
                    );
                  }}
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    <Field label="Dwelling no.">
                      <Input aria-label="Lease dwelling" type="number" inputMode="numeric" min={1} value={lease.dwelling} onChange={(e) => setLease({ ...lease, dwelling: e.target.value })} />
                    </Field>
                    <Field label="Rent a day" unit="cr">
                      <Input aria-label="Lease rent" type="number" inputMode="decimal" step="0.01" min={0} value={lease.rent} onChange={(e) => setLease({ ...lease, rent: e.target.value })} />
                    </Field>
                    <Field label="Term" unit="days" hint="Empty for an open term.">
                      <Input aria-label="Lease term" type="number" inputMode="numeric" min={1} placeholder="open" value={lease.term} onChange={(e) => setLease({ ...lease, term: e.target.value })} />
                    </Field>
                  </div>
                  <ButtonRow>
                    <Button type="submit" disabled={offerLease.isPending} disabledReason={lease.dwelling.trim() === "" ? "name the dwelling" : undefined}>
                      Offer
                    </Button>
                  </ButtonRow>
                </form>
              </More>
            ) : null}

            <More summary="Send a transfer" testId="transfer-more">
              <form
                className="grid gap-3"
                data-testid="transfer-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  const to = gift.toKind === "citizen" ? { citizen: Number(gift.to) } : { org: Number(gift.to) };
                  const asset = gift.kind === "money" ? { money: toCents(gift.amount) } : { good: [gift.good, gift.qty] };
                  transfer.mutate({ to: to as never, asset: asset as never, memo: gift.memo }, { onSuccess: ok("Sent."), onError: fail });
                }}
              >
                <p className="text-muted m-0 text-sm">A gift, dues, alms, or a side-payment: all transfers, told apart by the memo.</p>
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="To">
                    <Select aria-label="Transfer to kind" value={gift.toKind} onChange={(e) => setGift({ ...gift, toKind: e.target.value, to: "" })}>
                      <option value="citizen">a citizen</option>
                      <option value="org">an organization</option>
                    </Select>
                  </Field>
                  <Field label="Who">
                    <Select aria-label="Transfer to" value={gift.to} onChange={(e) => setGift({ ...gift, to: e.target.value })}>
                      <option value="">choose</option>
                      {(gift.toKind === "citizen" ? others.map((z) => ({ id: z.id, name: z.handle })) : (orgs.data ?? [])).map((z) => (
                        <option key={z.id} value={z.id}>
                          {z.name}
                        </option>
                      ))}
                    </Select>
                  </Field>
                  {c.money ? (
                    <Field label="What">
                      <Select aria-label="Transfer kind" value={gift.kind} onChange={(e) => setGift({ ...gift, kind: e.target.value })}>
                        <option value="money">money</option>
                        <option value="good">goods</option>
                      </Select>
                    </Field>
                  ) : null}
                  {gift.kind === "money" && c.money ? (
                    <Field label="Amount" unit="cr">
                      <Input aria-label="Transfer amount" type="number" inputMode="decimal" step="0.01" min={0.01} value={gift.amount} onChange={(e) => setGift({ ...gift, amount: e.target.value })} />
                    </Field>
                  ) : (
                    <>
                      <Field label="Good">
                        <Select aria-label="Transfer good" value={gift.good} onChange={(e) => setGift({ ...gift, good: e.target.value })}>
                          {GOODS.map((g) => (
                            <option key={g} value={g}>
                              {g}
                            </option>
                          ))}
                        </Select>
                      </Field>
                      <Field label="Quantity" unit={gift.good}>
                        <Input aria-label="Transfer quantity" type="number" inputMode="numeric" min={1} value={gift.qty} onChange={(e) => setGift({ ...gift, qty: Number(e.target.value) })} />
                      </Field>
                    </>
                  )}
                  <Field label="Memo" className="sm:col-span-2 sm:max-w-none">
                    <Input aria-label="Memo" placeholder="memo" value={gift.memo} onChange={(e) => setGift({ ...gift, memo: e.target.value })} maxLength={80} />
                  </Field>
                </div>
                <ButtonRow>
                  <Button type="submit" disabled={transfer.isPending} disabledReason={gift.to.trim() === "" ? "choose who" : undefined}>
                    Send
                  </Button>
                </ButtonRow>
              </form>
            </More>
          </Card>
        </Two>

        <FooterStrip>
          <span>{h.society.population} citizens</span>
          <span>{h.society.active_humans} people</span>
          <span>{h.society.unemployed} without work</span>
          {h.society.price_index != null ? (
            <span>
              {t("society_stat").toLowerCase()} {h.society.price_index.toFixed(2)}
            </span>
          ) : null}
        </FooterStrip>
      </Stack>
    </div>
  );
}
