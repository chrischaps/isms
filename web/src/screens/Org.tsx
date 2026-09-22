// One organization (GDD 6.1, 9.2; S1.11; docs/style.md §10). Everyone sees
// the Verdict — what the firm is to you and whether tonight's payroll is
// covered — the treasury, staff and your stake at a glance, the overview
// facts, the ownership, the production per workplace and the open offers.
// A manager gets the workspace: payroll with a top-up, job offers, asks,
// machines, workplaces, the treasury ledger. The controlling owner gets
// dividends, share issues, listing for sale, and the manager appointment.
// An employee sees their own contract with this org.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits, type EventRef, type OfferView } from "../api/client";
import { useBoard, useCapabilities, useHome, useLexicon } from "../api/hooks";
import { useContracts, useOfferLease, useTransfer } from "../api/contracts";
import { useBook, usePlaceOrder } from "../api/market";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { FactList } from "../components/FactList";
import { Field, Input, Select } from "../components/Field";
import { Figure } from "../components/Figure";
import { Ledger, TD, TD_NUM, TH, TH_NUM, type LedgerRow } from "../components/Ledger";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict, type Tone } from "../components/Verdict";
import {
  useAddWorkplace,
  useAppoint,
  useCancelOffer,
  useDividend,
  useIssueShares,
  useMachines,
  useOfferEmployment,
  useOfferSale,
  useOrg,
  useOrgLedger,
  useOrgsView,
  type OrgView,
  type WorkplaceView,
} from "../api/orgs";
import { useNames, workplaceTitles, type Names } from "../lib/names";
import { jobLine } from "../lib/offers";
import { orgVerdict } from "../lib/verdict";
import { whenOf } from "../lib/when";

const WORKPLACE_KINDS = ["farm", "mine", "foundry", "mill", "workshop", "machine_shop", "builder"];
const GOODS = ["grain", "ore", "materials", "food", "wares", "machines"];

type Shares = { issued: number; holdings: Record<string, number> };

function sharesOf(o: OrgView): Shares | null {
  const own = o.ownership as Record<string, unknown>;
  return (own.shares as Shares | undefined) ?? null;
}

function holderName(key: string, names: Names): string {
  if (key === "org_self") return "the firm itself";
  return names.citizen(Number(key.replace("citizen:", "")));
}

export function Org({ id, oid }: { id: number; oid: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const org = useOrg(id, oid);
  const all = useOrgsView(id);
  const board = useBoard(id);
  const offer = useOfferEmployment(id, oid);
  const place = usePlaceOrder(id);
  const machines = useMachines(id, oid);
  const dividend = useDividend(id, oid);
  const appoint = useAppoint(id, oid);
  const issue = useIssueShares(id, oid);
  const addWorkplace = useAddWorkplace(id, oid);
  const sale = useOfferSale(id);
  const lease = useOfferLease(id);
  const cancelOffer = useCancelOffer(id);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const contracts = useContracts(id);
  const transfer = useTransfer(id);
  const [askGood, setAskGood] = useState("ore");
  const askBook = useBook(id, askGood);
  const meId = home.data?.citizen.id ?? -1;
  const ledger = useOrgLedger(
    id,
    oid,
    Boolean(org.data && (org.data.i_manage || org.data.my_shares > 0 || org.data.members.includes(meId))),
  );
  const [topUp, setTopUp] = useState("");
  // Forms.
  const [job, setJob] = useState({ workplace: 0, hourly: "8.00", piece: false, hours: 8, places: 1, term: "", notice: 1 });
  const [ask, setAsk] = useState({ qty: 10, limit: "" });
  const [buy, setBuy] = useState({ qty: 1, limit: "" });
  const [mach, setMach] = useState({ workplace: 0, qty: 1 });
  const [perShare, setPerShare] = useState("0.10");
  const [issueQty, setIssueQty] = useState(100);
  const [listing, setListing] = useState({ qty: 10, price: "" });
  const [manager, setManager] = useState("");
  const [newKind, setNewKind] = useState("mine");
  const [dwellingTerms, setDwellingTerms] = useState({ rent: "8.00", price: "" });

  if (home.isPending || org.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
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
  if (org.error instanceof ApiError && org.error.status === 404) return <p className="text-muted">No such organization.</p>;
  if (home.error || org.error || caps.error) return <p className="text-crit">Could not load: {String(home.error ?? org.error ?? caps.error)}</p>;
  const h = home.data!;
  const o = org.data!;
  const c = caps.data!;
  const me = h.citizen.id;
  const titles = workplaceTitles(o.workplaces);
  const title = (wid: number) => titles.get(wid) ?? names.workplaceTitle(wid);
  const shares = sharesOf(o);
  const myShare = shares && shares.issued > 0 ? o.my_shares / shares.issued : 0;
  const controlling = myShare > 0.5;
  const manage = o.i_manage;
  const slots = (all.data?.slots ?? {}) as Record<string, { total?: number | null; free?: number | null }>;
  const founding = all.data?.founding;
  // A Builder's units are dwellings, listed below, not goods in the inventory (D6).
  const makesDwellings = (kind: string) => all.data?.recipes.find((r) => r.workplace_kind === kind)?.produces_asset ?? false;
  const offers = (board.data?.offers ?? []).filter((x) => (x.by as Record<string, unknown>).org === oid);
  // Payroll due at cycle end (TDD 5.5 8a): every active employment contract of
  // this org, hourly ones at the worker's hours this cycle, piece-rate ones at
  // what has been attributed so far. The treasury covers it or the workers are
  // paid pro rata and their contracts end.
  const payroll = (contracts.data?.contracts ?? [])
    .filter((k) => k.status === "active")
    .flatMap((k) => {
      const e = (k.body as Record<string, unknown>).employment as Record<string, unknown> | undefined;
      if (!e || e.org !== oid) return [];
      const worker = (k.parties as unknown as Record<string, number>[]).find((p) => typeof p.citizen === "number")?.citizen;
      const w = o.workplaces.flatMap((wp) => wp.workers).find((x) => x.citizen === worker);
      const pay = e.pay as Record<string, number>;
      const due =
        pay.hourly !== undefined
          ? (w?.hours ?? 0) * pay.hourly
          : Math.round((w?.attributed_this_cycle ?? 0) * (pay.piece_rate ?? 0));
      return [{ contract: k.id, worker: w?.handle ?? names.citizen(Number(worker)), due }];
    });
  const payrollDue = payroll.reduce((n, p) => n + p.due, 0);
  const shortfall = Math.max(0, payrollDue - o.treasury);
  const myContract = h.labor.employment.find(
    (k) => ((k.body as Record<string, unknown>).employment as Record<string, unknown> | undefined)?.org === oid,
  );
  const employed = myContract !== undefined || o.workplaces.some((w) => w.workers.some((k) => k.citizen === me));
  const sigma = c.monitoring_sigma;
  const fail = (e: Error) => {
    setNote(null);
    setError(e.message);
  };
  const ok = (what: string) => () => {
    setError(null);
    setNote(what);
  };
  const firstWp = o.workplaces[0]?.id ?? 0;
  const jobWp = job.workplace || firstWp;
  const machWp = mach.workplace || firstWp;
  const cents = (s: string) => Math.round(Number(s) * 100) || 0;
  const held = (g: string) => (o.inventory as Record<string, number>)[g] ?? 0;
  const inventory = Object.entries(o.inventory as Record<string, number>)
    .map(([g, n]) => `${n} ${g}`)
    .join(", ");

  const verdict = orgVerdict({
    name: o.name,
    share: myShare,
    controlling,
    manage,
    employed,
    member: o.members.includes(me),
    employees: o.employees,
    money: c.money,
    payroll: manage && c.money ? { due: payrollDue, shortfall, workers: payroll.length } : undefined,
    paymentMissed: o.payment_missed || o.last_payment_missed != null,
    hiring: offers.reduce((n, x) => n + (jobLine(x)?.places ?? 0), 0),
  });
  const treasuryTone: Tone = manage && c.money ? (shortfall > 0 ? "crit" : "good") : "good";
  const treasuryStatus =
    manage && c.money
      ? payroll.length === 0
        ? "Nobody on the payroll."
        : shortfall > 0
          ? `Short by ${credits(shortfall)} cr for tonight's payroll.`
          : `Covers tonight's payroll of ${credits(payrollDue)} cr.`
      : o.escrow > 0
        ? `${credits(o.escrow)} cr more held in open bids.`
        : undefined;

  // The treasury's side of each event, in cents; goods movements as text.
  const ledgerRows: LedgerRow[] = (ledger.data?.entries ?? [])
    .slice()
    .reverse()
    .flatMap((e: EventRef): LedgerRow[] => {
      const p = (e.payload[e.kind] ?? {}) as Record<string, unknown>;
      const isOrg = (party: unknown) => (party as Record<string, unknown> | undefined)?.org === oid;
      const when = whenOf(e, h.clock.ticks_per_cycle);
      const base = { key: String(e.seq), epoch: e.epoch, when, explain: (p.explain as LedgerRow["explain"]) ?? null };
      switch (e.kind) {
        case "Trade": {
          const qty = Number(p.qty ?? 0);
          const price = Number(p.price ?? 0);
          const inst = Object.values((p.instrument as object) ?? {})[0];
          const sold = isOrg(p.seller);
          return [{ ...base, what: `${sold ? "Sold" : "Bought"} ${qty} ${String(inst)} at ${credits(price)}`, cents: sold ? qty * price : -qty * price }];
        }
        case "SaleAccepted": {
          const money = Number((p.price as Record<string, number> | undefined)?.money ?? 0);
          const sold = isOrg(p.seller);
          return [{ ...base, what: `${sold ? "Sold" : "Bought"} by direct sale`, cents: sold ? money : -money }];
        }
        case "Transferred": {
          const asset = (p.asset ?? {}) as Record<string, unknown>;
          const inbound = isOrg(p.to);
          const memo = String(p.memo ?? "").trim();
          const what = `Transfer ${inbound ? "in" : "out"}${memo ? `: ${memo}` : ""}`;
          if (typeof asset.money === "number") return [{ ...base, what, cents: inbound ? asset.money : -asset.money }];
          const g = asset.good as [string, number] | undefined;
          return [{ ...base, what, goods: g ? `${inbound ? "+" : "-"}${g[1]} ${g[0]}` : "" }];
        }
        case "Paid":
          return [{ ...base, what: `Wages, ${names.citizen(Number(p.citizen))}`, cents: -Number(p.amount ?? 0) }];
        case "PaymentMissed":
          return [{ ...base, what: `Payday missed, ${names.citizen(Number(p.citizen))}: owed ${credits(Number(p.owed ?? 0))}, paid`, cents: -Number(p.paid ?? 0) }];
        case "DividendPaid":
          return [{ ...base, what: `Dividend, ${names.citizen(Number(p.citizen))}`, cents: -Number(p.amount ?? 0) }];
        case "DwellingBuilt":
          return [{ ...base, what: `Built dwelling no. ${String(p.dwelling)}`, goods: `-${String(p.materials_consumed ?? 0)} materials` }];
        case "OrderPlaced": {
          const order = (p.order ?? {}) as Record<string, unknown>;
          if (!isOrg(order.owner)) return [];
          const escrow = (p.escrow ?? {}) as Record<string, unknown>;
          const inst = Object.values((order.instrument as object) ?? {})[0];
          if (typeof escrow.money === "number") return [{ ...base, what: `Bid placed for ${String(order.qty)} ${String(inst)} (escrow)`, cents: -escrow.money }];
          const g = escrow.good as [string, number] | undefined;
          return [{ ...base, what: `Ask placed: ${String(order.qty)} ${String(inst)} at ${credits(Number(order.limit_price ?? 0))} (escrow)`, goods: g ? `-${g[1]} ${g[0]}` : "" }];
        }
        default:
          return [];
      }
    });
  const bookHint = askBook.data
    ? `last ${askBook.data.last_price != null ? credits(askBook.data.last_price) : "none"} · best bid ${askBook.data.bids[0] ? credits(askBook.data.bids[0].price) : "none"} · best ask ${askBook.data.asks[0] ? credits(askBook.data.asks[0].price) : "none"}`
    : "";

  const production = (w: WorkplaceView) => (
    <Tile key={w.id} testId={`workplace-${w.id}`} className="grid gap-2">
      <div className="flex flex-wrap items-baseline justify-between gap-x-3">
        <span className="font-bold">{title(w.id)}</span>
        <span className="text-muted text-sm tabular-nums">
          {w.slot != null ? `slot ${w.slot} · ` : ""}
          {w.machines} machines · {w.cycle_output.toFixed(0)} {makesDwellings(w.kind) ? "dwellings" : "units"} today
        </span>
      </div>
      {w.workers.length === 0 ? (
        <p className="text-muted m-0 text-sm">Nobody works here yet.</p>
      ) : (
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr>
              <th className={TH}>Worker</th>
              <th className={TH_NUM}>Hours</th>
              <th className={TH_NUM}>Attributed today</th>
            </tr>
          </thead>
          <tbody>
            {w.workers.map((k) => (
              <tr key={k.citizen} className="hover:bg-surface-2" data-testid={`worker-${k.citizen}`}>
                <td className={TD}>
                  {k.handle}
                  {k.citizen === me ? (
                    <Pill className="ml-2">
                      you
                    </Pill>
                  ) : null}
                </td>
                <td className={TD_NUM}>{k.hours}</td>
                <td className={TD_NUM}>{k.attributed_this_cycle == null ? "—" : k.attributed_this_cycle.toFixed(1)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </Tile>
  );

  const stake = shares && o.my_shares > 0 ? `${(myShare * 100).toFixed(0)}` : null;

  return (
    <div>
      <PageHeader
        title={o.name}
        meta={
          <>
            <span>{o.kind.replace("_", " ")}</span>
            <span>{o.manager != null ? <>managed by <b>{names.citizen(o.manager)}</b></> : "no manager"}</span>
            {stake ? (
              <span>
                you hold <b>{stake}%</b>
                {controlling ? " (controlling)" : ""}
              </span>
            ) : null}
            <Link to="/s/$id/orgs" params={{ id: String(id) }}>
              All organizations
            </Link>
          </>
        }
      />

      {error ? (
        <p className="text-crit mb-4 text-sm" role="alert">
          {error}
        </p>
      ) : null}
      {note ? <p className="text-muted mb-4 text-sm">{note}</p> : null}

      <Stack>
        <Card
          title="The firm"
          icon="org"
          testId="org-glance"
          aside={
            o.last_payment_missed || o.payment_missed ? (
              <Pill tone="crit">missed a payday</Pill>
            ) : null
          }
        >
          <Verdict parts={verdict} />
          {o.last_payment_missed ? (
            <p className="text-crit mb-3 text-sm">
              Missed {names.citizen(o.last_payment_missed.citizen)}&apos;s payday on day {o.last_payment_missed.cycle}: owed {credits(o.last_payment_missed.owed)} cr, paid{" "}
              {credits(o.last_payment_missed.paid)} cr.
            </p>
          ) : null}
          <div className="grid gap-3 @min-[620px]:grid-cols-3">
            {c.money ? <Figure label="Treasury" value={credits(o.treasury)} unit="cr" status={treasuryStatus} tone={treasuryTone} testId="treasury-figure" /> : null}
            <Figure label="Staff" value={String(o.employees)} unit={o.employees === 1 ? "person" : "people"} status={o.workplaces.length === 0 ? "No workplace." : `${o.workplaces.length} workplace${o.workplaces.length === 1 ? "" : "s"}.`} />
            {shares ? (
              <Figure
                label="Your stake"
                value={stake ?? "0"}
                unit="%"
                status={stake ? `${o.my_shares} of ${shares.issued} shares${controlling ? "; controlling" : ""}.` : `${shares.issued} shares issued; none yours.`}
              />
            ) : (
              <Figure label="Members" value={String(o.members.length)} status={o.members.includes(me) ? "You among them." : "Not you."} />
            )}
          </div>
          <div className="mt-4" data-testid="org-overview">
            <FactList
              items={[
                ...(c.money
                  ? [
                      { key: "treasury", label: "Treasury", value: `${credits(o.treasury)} cr`, gloss: o.escrow > 0 ? `${credits(o.escrow)} cr more held in open bids` : undefined },
                      { key: "book", label: "Book value", value: `${credits(o.book_value)} cr` },
                      ...(o.declared_dividend != null
                        ? [{ key: "dividend", label: "Dividend", value: `${credits(o.declared_dividend)} cr a share`, gloss: "declared today, paid at the day's end", testId: "declared-dividend" }]
                        : []),
                    ]
                  : []),
                { key: "inventory", label: "Inventory", value: inventory || "empty" },
                { key: "staff", label: "Staff", value: String(o.employees) },
                ...(shares ? [{ key: "shares", label: "Shares", value: `${shares.issued} issued` }] : []),
              ]}
            />
          </div>
        </Card>

        <Two>
          <Card title="Ownership" icon="coin">
            {shares && Object.keys(shares.holdings).length === 0 ? (
              <p className="text-muted m-0" data-testid="share-registry">
                Nobody holds a share right now: all {shares.issued} sit in escrow, on the notice board or a book.
              </p>
            ) : shares ? (
              <ul className="m-0 grid list-none gap-1.5 p-0" data-testid="share-registry">
                {Object.entries(shares.holdings)
                  .sort((a, b) => b[1] - a[1])
                  .map(([k, n]) => (
                    <li key={k} className="border-line flex justify-between gap-3 border-b pb-1.5 last:border-b-0">
                      <span>
                        {holderName(k, names)}
                        {k === `citizen:${me}` ? (
                          <Pill className="ml-2">
                            you
                          </Pill>
                        ) : null}
                      </span>
                      <span className="tabular-nums">
                        {n} <span className="text-muted">({((100 * n) / shares.issued).toFixed(0)} %)</span>
                      </span>
                    </li>
                  ))}
              </ul>
            ) : (
              <p className="text-muted m-0">
                {o.members.length} members{o.members.includes(me) ? ", you among them" : ""}.
              </p>
            )}
          </Card>

          {myContract ? (
            <Card title={`Your ${t("job").toLowerCase()} here`} icon="contract" testId="my-contract">
              {(() => {
                const e = (myContract.body as Record<string, unknown>).employment as Record<string, unknown>;
                const pay = e.pay as Record<string, number>;
                return (
                  <>
                    <FactList
                      items={[
                        { key: "pay", label: "Pay", value: pay.hourly !== undefined ? `${credits(pay.hourly)} cr an hour` : `${credits(pay.piece_rate ?? 0)} cr a unit` },
                        { key: "hours", label: "Hours", value: `up to ${String(e.max_hours)} h a day` },
                        { key: "notice", label: "Notice", value: `${String(e.notice_cycles)} day(s)` },
                        ...(myContract.term_cycles != null ? [{ key: "term", label: "Term", value: `${myContract.term_cycles} days` }] : []),
                        { key: "status", label: "Status", value: <Pill tone={myContract.status === "active" ? "good" : "neutral"}>{myContract.status}</Pill> },
                      ]}
                    />
                    <p className="text-muted mt-3 mb-0 text-sm">
                      Hours and effort are set on{" "}
                      <Link to="/s/$id/work" params={{ id: String(id) }}>
                        {t("work_screen")}
                      </Link>
                      ; ending it with notice comes with the Contracts screen.
                    </p>
                  </>
                );
              })()}
            </Card>
          ) : (
            <Card title="Open offers" icon="contract">
              <OpenOffers offers={offers} manage={manage} t={t} title={title} onWithdraw={(x) => cancelOffer.mutate(x, { onError: fail })} />
            </Card>
          )}
        </Two>

        <Card title="Production" icon="work" testId="production">
          {o.workplaces.length === 0 ? <p className="text-muted m-0">No workplace.</p> : <div className="grid gap-3">{o.workplaces.map(production)}</div>}
          {manage && c.money ? (
            <Tile testId="payroll" className="mt-3 grid gap-2">
              <p className="m-0">
                Payroll due at the end of the day: <b className="tabular-nums">{credits(payrollDue)} cr</b>
                {payroll.length > 0 ? ` for ${payroll.length} worker${payroll.length === 1 ? "" : "s"}` : ""}. Treasury: <b className="tabular-nums">{credits(o.treasury)} cr</b>.{" "}
                {payroll.length === 0 ? (
                  <span className="text-muted">Nobody on the payroll.</span>
                ) : shortfall > 0 ? (
                  <span className="text-crit">Short by {credits(shortfall)} cr: workers would be paid pro rata, their contracts would end, and the firm would be flagged.</span>
                ) : (
                  <span className="text-good">Covered.</span>
                )}
              </p>
              {payroll.length > 0 ? <p className="text-muted m-0 text-sm">{payroll.map((p) => `${p.worker} ${credits(p.due)} cr`).join(" · ")}</p> : null}
              <form
                className="grid gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  transfer.mutate(
                    { to: { org: oid } as never, asset: { money: cents(topUp) } as never, memo: "treasury" },
                    { onSuccess: ok(`Moved ${credits(cents(topUp))} cr into the treasury.`), onError: fail },
                  );
                  setTopUp("");
                }}
              >
                <Field label={`Top up from your balance (${credits(h.household.balance)} cr)`} unit="cr">
                  <Input
                    aria-label="Top up amount"
                    type="number"
                    inputMode="decimal"
                    step="0.01"
                    min={0.01}
                    placeholder={shortfall > 0 ? (shortfall / 100).toFixed(2) : ""}
                    value={topUp}
                    onChange={(e) => setTopUp(e.target.value)}
                  />
                </Field>
                <ButtonRow>
                  <Button type="submit" disabled={transfer.isPending || cents(topUp) < 1}>
                    Transfer
                  </Button>
                  {shortfall > 0 ? (
                    <Button variant="quiet" onClick={() => setTopUp((shortfall / 100).toFixed(2))}>
                      cover the shortfall
                    </Button>
                  ) : null}
                </ButtonRow>
              </form>
            </Tile>
          ) : null}
          {manage ? (
            <p className="text-muted mt-3 mb-0 text-sm">
              {sigma === 0
                ? "Monitoring here is exact: attributed output is true output."
                : `Monitoring here is noisy (sigma ${sigma}): attributed output is a manager's estimate; each worker sees their own true figure.`}
            </p>
          ) : null}
        </Card>

        {o.dwellings.length > 0 || o.workplaces.some((w) => makesDwellings(w.kind)) ? (
          <Card title="Dwellings" icon="home" testId="org-dwellings">
            {o.dwellings.length === 0 ? (
              <p className="text-muted m-0">None yet. A Builder turns 10 Materials into one dwelling, on the hour its output reaches a whole unit; it appears here, owned by the firm.</p>
            ) : (
              <ul className="m-0 grid list-none gap-3 p-0">
                {o.dwellings.map((d) => {
                  const free = d.lease == null && d.offer == null;
                  return (
                    <li key={d.id}>
                      <Tile testId={`dwelling-${d.id}`} className="flex flex-wrap items-center justify-between gap-3">
                        <span>
                          <span className="font-bold">no. {d.id}</span>
                          <span className="text-muted"> · {d.occupant != null ? names.citizen(d.occupant) : "empty"}</span>
                          <span className="text-muted"> · {d.lease != null ? `let at ${credits(d.rent_per_cycle ?? 0)} cr a day` : d.offer != null ? "on the notice board" : "free"}</span>
                        </span>
                        {manage && free ? (
                          <span className="flex flex-wrap gap-2">
                            <Button
                              inline
                              onClick={() =>
                                lease.mutate(
                                  { asset: { dwelling: d.id } as never, rent_per_cycle: cents(dwellingTerms.rent), term_cycles: null, on_behalf_of: oid },
                                  { onSuccess: ok(`Dwelling no. ${d.id} is on the notice board to let.`), onError: fail },
                                )
                              }
                            >
                              Let it
                            </Button>
                            <Button
                              inline
                              disabled={cents(dwellingTerms.price) < 1}
                              onClick={() =>
                                sale.mutate(
                                  { asset: { dwelling: d.id }, price: { money: cents(dwellingTerms.price) }, on_behalf_of: oid },
                                  { onSuccess: ok(`Dwelling no. ${d.id} is on the notice board for sale.`), onError: fail },
                                )
                              }
                            >
                              Sell it
                            </Button>
                          </span>
                        ) : null}
                      </Tile>
                    </li>
                  );
                })}
              </ul>
            )}
            {manage && o.dwellings.some((d) => d.lease == null && d.offer == null) ? (
              <div className="mt-3 grid gap-3 sm:grid-cols-2">
                <Field label="Rent a day" unit="cr">
                  <Input aria-label="Dwelling rent" type="number" inputMode="decimal" step="0.01" min={0} value={dwellingTerms.rent} onChange={(e) => setDwellingTerms({ ...dwellingTerms, rent: e.target.value })} />
                </Field>
                <Field label="Sale price" unit="cr" hint="Rent settles to the treasury each day; a sale is for the price in one payment.">
                  <Input aria-label="Dwelling price" type="number" inputMode="decimal" step="0.01" min={0.01} value={dwellingTerms.price} onChange={(e) => setDwellingTerms({ ...dwellingTerms, price: e.target.value })} />
                </Field>
              </div>
            ) : null}
          </Card>
        ) : null}

        {myContract ? (
          <Card title="Open offers" icon="contract">
            <OpenOffers offers={offers} manage={manage} t={t} title={title} onWithdraw={(x) => cancelOffer.mutate(x, { onError: fail })} />
          </Card>
        ) : null}

        {manage ? (
          <Two className="items-start" testId="manager-workspace">
            <Card title="Post a job offer" icon="work">
              <form
                className="grid gap-3"
                data-testid="job-offer-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  offer.mutate(
                    {
                      workplace: jobWp,
                      pay: job.piece ? { piece_rate: cents(job.hourly) } : { hourly: cents(job.hourly) },
                      max_hours: job.hours,
                      places: job.places,
                      term_cycles: job.term.trim() === "" ? null : Number(job.term),
                      notice_cycles: job.notice,
                    } as never,
                    { onSuccess: ok("Job offer posted to the notice board."), onError: fail },
                  );
                }}
              >
                <Field label="Workplace">
                  <Select aria-label="Job workplace" value={jobWp} onChange={(e) => setJob({ ...job, workplace: Number(e.target.value) })}>
                    {o.workplaces.map((w) => (
                      <option key={w.id} value={w.id}>
                        {title(w.id)}
                      </option>
                    ))}
                  </Select>
                </Field>
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="Pay" unit="cr">
                    <Input aria-label="Pay" type="number" inputMode="decimal" step="0.01" min={0.01} value={job.hourly} onChange={(e) => setJob({ ...job, hourly: e.target.value })} />
                  </Field>
                  <Field label="Pay basis">
                    <Select aria-label="Pay basis" value={job.piece ? "piece" : "hourly"} onChange={(e) => setJob({ ...job, piece: e.target.value === "piece" })}>
                      <option value="hourly">an hour</option>
                      <option value="piece">a unit</option>
                    </Select>
                  </Field>
                  <Field label="Max hours" unit="h">
                    <Input aria-label="Max hours" type="number" inputMode="numeric" min={1} max={8} value={job.hours} onChange={(e) => setJob({ ...job, hours: Number(e.target.value) })} />
                  </Field>
                  <Field label="Places">
                    <Input aria-label="Places" type="number" inputMode="numeric" min={1} value={job.places} onChange={(e) => setJob({ ...job, places: Number(e.target.value) })} />
                  </Field>
                  <Field label="Term" unit="days" hint="Empty for an open term.">
                    <Input aria-label="Term" type="number" inputMode="numeric" min={1} placeholder="open" value={job.term} onChange={(e) => setJob({ ...job, term: e.target.value })} />
                  </Field>
                  <Field label="Notice" unit="days">
                    <Input aria-label="Notice" type="number" inputMode="numeric" min={0} value={job.notice} onChange={(e) => setJob({ ...job, notice: Number(e.target.value) })} />
                  </Field>
                </div>
                <ButtonRow>
                  <Button type="submit" variant="primary" disabled={offer.isPending} disabledReason={o.workplaces.length === 0 ? "add a workplace first" : undefined}>
                    Post job offer
                  </Button>
                </ButtonRow>
                <p className="text-muted m-0 text-sm">A new firm has an empty treasury: the founding fee is burned, not deposited. Check the payroll line above before hiring.</p>
              </form>
            </Card>

            <Stack>
              {c.order_books ? (
                <Card title="Sell from inventory" icon="market">
                  <form
                    className="grid gap-3"
                    data-testid="ask-form"
                    onSubmit={(e) => {
                      e.preventDefault();
                      place.mutate(
                        { instrument: askGood, side: "ask", qty: ask.qty, limit_price: cents(ask.limit), on_behalf_of: oid },
                        { onSuccess: ok(`Ask placed on the ${askGood} book for the firm.`), onError: fail },
                      );
                    }}
                  >
                    <Field label="Good">
                      <Select aria-label="Ask good" value={askGood} onChange={(e) => setAskGood(e.target.value)}>
                        {GOODS.map((g) => (
                          <option key={g} value={g}>
                            {g} ({held(g)} held)
                          </option>
                        ))}
                      </Select>
                    </Field>
                    <Field label="Quantity" unit={askGood}>
                      <Input aria-label="Ask quantity" type="number" inputMode="numeric" min={1} value={ask.qty} onChange={(e) => setAsk({ ...ask, qty: Number(e.target.value) })} />
                    </Field>
                    <p className="text-muted -mt-1 mb-0 text-sm">
                      <span data-testid="ask-held">{held(askGood)} held</span> ·{" "}
                      <button type="button" className="text-accent underline" onClick={() => setAsk({ ...ask, qty: held(askGood) })}>
                        Max
                      </button>
                    </p>
                    <Field
                      label="Limit"
                      unit="cr a unit"
                      hint={<span data-testid="ask-book-hint">{bookHint ? `${bookHint}; ` : ""}the goods sit in escrow until filled</span>}
                    >
                      <Input aria-label="Ask limit" type="number" inputMode="decimal" step="0.01" min={0.01} value={ask.limit} onChange={(e) => setAsk({ ...ask, limit: e.target.value })} />
                    </Field>
                    <ButtonRow>
                      <Button type="submit" variant="primary" disabled={place.isPending} disabledReason={cents(ask.limit) < 1 ? "set a limit" : undefined}>
                        Post ask
                      </Button>
                    </ButtonRow>
                  </form>
                </Card>
              ) : null}

              <Card title="Machines" icon="gear" testId="machines" subtitle={`Installed machines raise the workplace's capital multiplier; they wear a little each day. The firm holds ${held("machines")} uninstalled.`}>
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="Workplace">
                    <Select aria-label="Machines workplace" value={machWp} onChange={(e) => setMach({ ...mach, workplace: Number(e.target.value) })}>
                      {o.workplaces.map((w) => (
                        <option key={w.id} value={w.id}>
                          {title(w.id)} ({w.machines} installed)
                        </option>
                      ))}
                    </Select>
                  </Field>
                  <Field label="Machines">
                    <Input aria-label="Machines quantity" type="number" inputMode="numeric" min={1} value={mach.qty} onChange={(e) => setMach({ ...mach, qty: Number(e.target.value) })} />
                  </Field>
                </div>
                <ButtonRow>
                  <Button onClick={() => machines.mutate({ workplace: machWp, action: "install", qty: mach.qty }, { onSuccess: ok("Installed."), onError: fail })}>Install</Button>
                  <Button onClick={() => machines.mutate({ workplace: machWp, action: "uninstall", qty: mach.qty }, { onSuccess: ok("Uninstalled."), onError: fail })}>Uninstall</Button>
                </ButtonRow>
                {c.order_books ? (
                  <div className="border-line mt-4 border-t border-dashed pt-3">
                    <p className="mb-2 text-sm font-bold">Bid for machines from the treasury</p>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <Field label="Quantity" unit="machines">
                        <Input aria-label="Machines bid quantity" type="number" inputMode="numeric" min={1} value={buy.qty} onChange={(e) => setBuy({ ...buy, qty: Number(e.target.value) })} />
                      </Field>
                      <Field label="Limit" unit="cr">
                        <Input aria-label="Machines bid limit" type="number" inputMode="decimal" step="0.01" min={0.01} value={buy.limit} onChange={(e) => setBuy({ ...buy, limit: e.target.value })} />
                      </Field>
                    </div>
                    <ButtonRow>
                      <Button
                        disabledReason={cents(buy.limit) < 1 ? "set a limit" : undefined}
                        onClick={() =>
                          place.mutate(
                            { instrument: "machines", side: "bid", qty: buy.qty, limit_price: cents(buy.limit), on_behalf_of: oid },
                            { onSuccess: ok("Bid placed on the machines book for the firm."), onError: fail },
                          )
                        }
                      >
                        Place bid
                      </Button>
                    </ButtonRow>
                  </div>
                ) : null}
              </Card>

              <Card title="Add a workplace" icon="org" testId="add-workplace">
                <Field label="Kind" hint={founding ? `${founding.materials} Materials from the firm's inventory.` : undefined}>
                  <Select aria-label="New workplace kind" value={newKind} onChange={(e) => setNewKind(e.target.value)}>
                    {WORKPLACE_KINDS.map((k) => {
                      const s = slots[k];
                      return (
                        <option key={k} value={k} disabled={s?.total != null && (s.free ?? 0) === 0}>
                          {k.replace("_", " ")} {s?.total == null ? "" : `(${s.free ?? 0} of ${s.total} slots free)`}
                        </option>
                      );
                    })}
                  </Select>
                </Field>
                <ButtonRow>
                  <Button onClick={() => addWorkplace.mutate({ kind: newKind }, { onSuccess: ok("Workplace added."), onError: fail })}>Add</Button>
                </ButtonRow>
              </Card>
            </Stack>
          </Two>
        ) : null}

        {shares && controlling ? (
          <Card title="Owner's tools" icon="coin" testId="owner-tools">
            <div className="grid gap-3 md:grid-cols-2">
              {c.money ? (
                <Tile className="grid content-start gap-3">
                  <span className="font-bold">Dividend</span>
                  {o.declared_dividend != null ? (
                    <p className="text-muted m-0 text-sm">Declared for today: {credits(o.declared_dividend)} cr a share, paid at the day&apos;s end. One a day.</p>
                  ) : (
                    <>
                      <Field
                        label="Per share"
                        unit="cr"
                        hint={`${credits(cents(perShare) * (shares.issued - (shares.holdings.org_self ?? 0)))} cr from the treasury, split by share count; the firm's own shares earn nothing.`}
                      >
                        <Input aria-label="Dividend per share" type="number" inputMode="decimal" step="0.01" min={0.01} value={perShare} onChange={(e) => setPerShare(e.target.value)} />
                      </Field>
                      <ButtonRow>
                        <Button onClick={() => dividend.mutate(cents(perShare), { onSuccess: ok("Dividend declared."), onError: fail })}>Declare</Button>
                      </ButtonRow>
                    </>
                  )}
                </Tile>
              ) : null}
              <Tile className="grid content-start gap-3">
                <span className="font-bold">Issue shares</span>
                <Field label="Quantity" unit="shares" hint="New shares land in the firm's own holding, to be sold on its behalf.">
                  <Input aria-label="Issue quantity" type="number" inputMode="numeric" min={1} value={issueQty} onChange={(e) => setIssueQty(Number(e.target.value))} />
                </Field>
                <ButtonRow>
                  <Button onClick={() => issue.mutate(issueQty, { onSuccess: ok("Shares issued."), onError: fail })}>Issue</Button>
                </ButtonRow>
              </Tile>
              <Tile className="grid content-start gap-3">
                <span className="font-bold">List for sale</span>
                <Field label="Shares" unit="shares">
                  <Input aria-label="Listing quantity" type="number" inputMode="numeric" min={1} max={o.my_shares} value={listing.qty} onChange={(e) => setListing({ ...listing, qty: Number(e.target.value) })} />
                </Field>
                <Field label="Price" unit="cr for the lot" hint="A sale offer on the notice board for your own shares; the order book takes them too.">
                  <Input aria-label="Listing price" type="number" inputMode="decimal" step="0.01" min={0.01} value={listing.price} onChange={(e) => setListing({ ...listing, price: e.target.value })} />
                </Field>
                <ButtonRow>
                  <Button
                    disabledReason={cents(listing.price) < 1 ? "set a price" : undefined}
                    onClick={() =>
                      sale.mutate(
                        { asset: { shares: [oid, listing.qty] }, price: { money: cents(listing.price) } },
                        { onSuccess: ok("Listed on the notice board."), onError: fail },
                      )
                    }
                  >
                    List
                  </Button>
                </ButtonRow>
              </Tile>
              <Tile className="grid content-start gap-3">
                <span className="font-bold">Manager</span>
                <Field label="Citizen id">
                  <Input aria-label="Manager citizen" type="number" inputMode="numeric" min={1} placeholder={String(o.manager ?? "")} value={manager} onChange={(e) => setManager(e.target.value)} />
                </Field>
                <ButtonRow>
                  <Button disabled={manager.trim() === ""} onClick={() => appoint.mutate(Number(manager), { onSuccess: ok("Manager appointed."), onError: fail })}>
                    Appoint
                  </Button>
                  <Button variant="quiet" onClick={() => appoint.mutate(null, { onSuccess: ok("Manager vacated."), onError: fail })}>
                    vacate
                  </Button>
                </ButtonRow>
              </Tile>
            </div>
          </Card>
        ) : null}

        {ledger.data ? (
          <Card title="Treasury ledger" icon="ledger" subtitle="Every movement of the treasury and its escrow, newest first: trades and sales, transfers, wages, dividends, orders placed, dwellings built.">
            <div data-testid="treasury-ledger">
              <Ledger rows={ledgerRows} empty="Nothing has moved the treasury yet." />
            </div>
          </Card>
        ) : null}

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

/** A non-job offer of the firm's in a sentence: what is offered and for what. */
function offerLine(x: OfferView): string {
  const b = x.body as Record<string, Record<string, unknown>>;
  const asset = (a: unknown): string => {
    const o = (a ?? {}) as Record<string, unknown>;
    if (Array.isArray(o.shares)) return `${String(o.shares[1])} shares of the firm`;
    if (Array.isArray(o.good)) return `${String(o.good[1])} ${String(o.good[0])}`;
    if (o.dwelling !== undefined) return `dwelling no. ${String(o.dwelling)}`;
    if (typeof o.money === "number") return `${credits(o.money)} cr`;
    return JSON.stringify(a);
  };
  if (b.sale) return `For sale: ${asset(b.sale.asset)} for ${asset(b.sale.price)}`;
  if (b.lease) return `To let: ${asset(b.lease.asset)} at ${credits(Number(b.lease.rent_per_cycle))} cr a day`;
  return `${x.kind}: ${JSON.stringify(x.body)}`;
}

function OpenOffers({
  offers,
  manage,
  t,
  title,
  onWithdraw,
}: {
  offers: OfferView[];
  manage: boolean;
  t: (k: string) => string;
  title: (wid: number) => string;
  onWithdraw: (id: number) => void;
}) {
  if (offers.length === 0) return <p className="text-muted m-0">None on the notice board.</p>;
  return (
    <ul className="m-0 grid list-none gap-1.5 p-0" data-testid="org-offers">
      {offers.map((x) => {
        const j = jobLine(x);
        return (
          <li key={x.id} className="border-line flex flex-wrap items-center justify-between gap-2 border-b pb-1.5 last:border-b-0">
            <span>{j ? `${t("job")}: ${j.pay}, up to ${j.hours} h, ${j.term}, notice ${j.notice} day(s), ${j.places} open (${title(j.workplace)})` : offerLine(x)}</span>
            {manage ? (
              <Button variant="quiet" inline onClick={() => onWithdraw(x.id)}>
                withdraw
              </Button>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}
