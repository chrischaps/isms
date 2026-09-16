// One organization (GDD 6.1, 9.2; S1.11). Everyone sees the overview and
// the open offers. A manager gets the workspace: production per workplace
// with per-worker attribution and the monitoring note, job offers, asks,
// machines, the treasury, and workplaces. The controlling owner gets
// dividends, share issues, listing for sale, and the manager appointment.
// An employee sees their own contract with this org.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits, type OfferView } from "../api/client";
import { useBoard, useCapabilities, useHome, useLexicon } from "../api/hooks";
import { usePlaceOrder } from "../api/market";
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
  useOrgsView,
  type OrgView,
  type WorkplaceView,
} from "../api/orgs";
import { jobLine } from "../lib/offers";

const WORKPLACE_KINDS = ["farm", "mine", "foundry", "mill", "workshop", "machine_shop", "builder"];
const GOODS = ["grain", "ore", "materials", "food", "wares", "machines"];

type Shares = { issued: number; holdings: Record<string, number> };

function sharesOf(o: OrgView): Shares | null {
  const own = o.ownership as Record<string, unknown>;
  return (own.shares as Shares | undefined) ?? null;
}

function holderName(key: string, me: number, handles: Map<number, string>): string {
  if (key === "org_self") return "the firm itself";
  const n = Number(key.replace("citizen:", ""));
  if (n === me) return "you";
  return handles.get(n) ?? `citizen ${n}`;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex items-center gap-2">
      <span className="w-28">{label}</span>
      {children}
    </label>
  );
}

export function Org({ id, oid }: { id: number; oid: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
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
  const cancelOffer = useCancelOffer(id);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  // Forms.
  const [job, setJob] = useState({ workplace: 0, hourly: "8.00", piece: false, hours: 8, places: 1, term: "", notice: 1 });
  const [ask, setAsk] = useState({ good: "ore", qty: 10, limit: "" });
  const [buy, setBuy] = useState({ qty: 1, limit: "" });
  const [mach, setMach] = useState({ workplace: 0, qty: 1 });
  const [perShare, setPerShare] = useState("0.10");
  const [issueQty, setIssueQty] = useState(100);
  const [listing, setListing] = useState({ qty: 10, price: "" });
  const [manager, setManager] = useState("");
  const [newKind, setNewKind] = useState("mine");

  if (home.isPending || org.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
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
  if (org.error instanceof ApiError && org.error.status === 404) return <p className="text-muted">No such organization.</p>;
  if (home.error || org.error || caps.error) return <p className="text-bad">Could not load: {String(home.error ?? org.error ?? caps.error)}</p>;
  const h = home.data!;
  const o = org.data!;
  const c = caps.data!;
  const me = h.citizen.id;
  const handles = new Map<number, string>();
  for (const w of o.workplaces) for (const k of w.workers) handles.set(k.citizen, k.handle);
  const shares = sharesOf(o);
  const myShare = shares && shares.issued > 0 ? o.my_shares / shares.issued : 0;
  const controlling = myShare > 0.5;
  const manage = o.i_manage;
  const slots = (all.data?.slots ?? {}) as Record<string, { total?: number | null; free?: number | null }>;
  const founding = all.data?.founding;
  const offers = (board.data?.offers ?? []).filter((x) => (x.by as Record<string, unknown>).org === oid);
  const myContract = h.labor.employment.find(
    (k) => ((k.body as Record<string, unknown>).employment as Record<string, unknown> | undefined)?.org === oid,
  );
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

  const production = (w: WorkplaceView) => (
    <div key={w.id} className="mt-3" data-testid={`workplace-${w.id}`}>
      <h4 className="text-sm">
        {w.kind.replace("_", " ")} <span className="text-muted">workplace {w.id}{w.slot != null ? `, slot ${w.slot}` : ""}</span>
        <span className="num text-muted"> · {w.machines} machines · {w.cycle_output.toFixed(0)} units this cycle</span>
      </h4>
      {w.workers.length === 0 ? (
        <p className="text-muted mt-1 text-xs">Nobody works here yet.</p>
      ) : (
        <table className="mt-1 w-full text-sm">
          <thead className="text-muted text-left text-xs uppercase tracking-wide">
            <tr>
              <th className="py-1 font-normal">Worker</th>
              <th className="py-1 text-right font-normal">Hours</th>
              <th className="py-1 text-right font-normal">Attributed this cycle</th>
            </tr>
          </thead>
          <tbody>
            {w.workers.map((k) => (
              <tr key={k.citizen} className="rule" data-testid={`worker-${k.citizen}`}>
                <td className="py-1 pr-2">
                  {k.handle}
                  {k.citizen === me ? <span className="text-muted"> (you)</span> : null}
                </td>
                <td className="num py-1 text-right">{k.hours}</td>
                <td className="num py-1 text-right">{k.attributed_this_cycle == null ? "—" : k.attributed_this_cycle.toFixed(1)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <div>
          <h2 className="text-2xl">{o.name}</h2>
          <p className="text-muted text-sm">
            {o.kind.replace("_", " ")}
            {o.manager != null ? ` · managed by ${o.manager === me ? "you" : (handles.get(o.manager) ?? `citizen ${o.manager}`)}` : " · no manager"}
            {shares && o.my_shares > 0 ? ` · you hold ${(myShare * 100).toFixed(0)}%${controlling ? " (controlling)" : ""}` : ""}
            {o.payment_missed ? " · missed a payday" : ""}
          </p>
        </div>
        <Link to="/s/$id/orgs" params={{ id: String(id) }} className="text-muted text-sm underline">
          All organizations
        </Link>
      </header>

      {error ? (
        <p className="text-bad text-sm" role="alert">
          {error}
        </p>
      ) : null}
      {note ? <p className="text-muted text-sm">{note}</p> : null}

      <section className="grid gap-8 md:grid-cols-2">
        <dl className="grid grid-cols-2 gap-y-1 text-sm" data-testid="org-overview">
          {c.money ? (
            <>
              <dt className="text-muted">Treasury</dt>
              <dd className="num">{credits(o.treasury)} cr</dd>
              <dt className="text-muted">Book value</dt>
              <dd className="num">{credits(o.book_value)} cr</dd>
            </>
          ) : null}
          <dt className="text-muted">Inventory</dt>
          <dd className="num">
            {Object.entries(o.inventory as Record<string, number>)
              .map(([g, n]) => `${n} ${g}`)
              .join(", ") || "empty"}
          </dd>
          <dt className="text-muted">Staff</dt>
          <dd className="num">{o.employees}</dd>
          {shares ? (
            <>
              <dt className="text-muted">Shares</dt>
              <dd className="num">{shares.issued} issued</dd>
            </>
          ) : null}
        </dl>
        <div>
          <h3 className="text-lg">Ownership</h3>
          {shares ? (
            <ul className="mt-2 text-sm" data-testid="share-registry">
              {Object.entries(shares.holdings)
                .sort((a, b) => b[1] - a[1])
                .map(([k, n]) => (
                  <li key={k} className="rule flex justify-between pt-1">
                    <span>{holderName(k, me, handles)}</span>
                    <span className="num">
                      {n} ({((100 * n) / shares.issued).toFixed(0)}%)
                    </span>
                  </li>
                ))}
            </ul>
          ) : (
            <p className="text-muted mt-2 text-sm">{o.members.length} members{o.members.includes(me) ? ", you among them" : ""}.</p>
          )}
        </div>
      </section>

      {myContract ? (
        <section className="bg-paper-2 rounded-sm p-3 text-sm" data-testid="my-contract">
          <h3 className="text-lg">Your {t("job").toLowerCase()} here</h3>
          {(() => {
            const e = (myContract.body as Record<string, unknown>).employment as Record<string, unknown>;
            const pay = e.pay as Record<string, number>;
            return (
              <p className="mt-1">
                Contract #{myContract.id}: {pay.hourly !== undefined ? `${credits(pay.hourly)} cr an hour` : `${credits(pay.piece_rate ?? 0)} cr a unit`}, up to{" "}
                {String(e.max_hours)} h a cycle, notice {String(e.notice_cycles)} cycle(s)
                {myContract.term_cycles != null ? `, term ${myContract.term_cycles} cycles` : ""}. Status: {myContract.status}. Hours and effort are set on{" "}
                <Link to="/s/$id/work" params={{ id: String(id) }} className="underline">
                  {t("work_screen")}
                </Link>
                ; ending it with notice comes with the Contracts screen.
              </p>
            );
          })()}
        </section>
      ) : null}

      <section>
        <h3 className="text-lg">Production</h3>
        {o.workplaces.length === 0 ? <p className="text-muted mt-2 text-sm">No workplace.</p> : o.workplaces.map(production)}
        {manage ? (
          <p className="text-muted mt-2 text-xs">
            {sigma === 0
              ? "Monitoring here is exact: attributed output is true output."
              : `Monitoring here is noisy (sigma ${sigma}): attributed output is a manager's estimate; each worker sees their own true figure.`}
          </p>
        ) : null}
      </section>

      <section>
        <h3 className="text-lg">Open offers</h3>
        {offers.length === 0 ? (
          <p className="text-muted mt-2 text-sm">None on the notice board.</p>
        ) : (
          <ul className="mt-2 flex flex-col gap-1 text-sm" data-testid="org-offers">
            {offers.map((x: OfferView) => {
              const j = jobLine(x);
              return (
                <li key={x.id} className="rule flex flex-wrap items-baseline justify-between gap-2 pt-1">
                  <span>
                    {j
                      ? `${t("job")}: ${j.pay}, up to ${j.hours} h, ${j.term}, notice ${j.notice}, ${j.places} open (workplace ${j.workplace})`
                      : `${x.kind}: ${JSON.stringify(x.body)}`}
                  </span>
                  {manage && x.kind !== "employment" ? (
                    <button type="button" className="text-muted text-xs underline" onClick={() => cancelOffer.mutate(x.id, { onError: fail })}>
                      withdraw
                    </button>
                  ) : null}
                </li>
              );
            })}
          </ul>
        )}
      </section>

      {manage ? (
        <section className="grid gap-8 md:grid-cols-2" data-testid="manager-workspace">
          <form
            className="flex flex-col gap-2 text-sm"
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
            <h3 className="text-lg">Post a job offer</h3>
            <Field label="Workplace">
              <select aria-label="Job workplace" className="border-line rounded-sm border px-1" value={jobWp} onChange={(e) => setJob({ ...job, workplace: Number(e.target.value) })}>
                {o.workplaces.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.kind.replace("_", " ")} #{w.id}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Pay">
              <input aria-label="Pay" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={job.hourly} onChange={(e) => setJob({ ...job, hourly: e.target.value })} />
              <select aria-label="Pay basis" className="border-line rounded-sm border px-1" value={job.piece ? "piece" : "hourly"} onChange={(e) => setJob({ ...job, piece: e.target.value === "piece" })}>
                <option value="hourly">cr an hour</option>
                <option value="piece">cr a unit</option>
              </select>
            </Field>
            <Field label="Max hours">
              <input aria-label="Max hours" type="number" min={1} max={8} className="border-line num w-16 rounded-sm border px-1" value={job.hours} onChange={(e) => setJob({ ...job, hours: Number(e.target.value) })} />
            </Field>
            <Field label="Places">
              <input aria-label="Places" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={job.places} onChange={(e) => setJob({ ...job, places: Number(e.target.value) })} />
            </Field>
            <Field label="Term (cycles)">
              <input aria-label="Term" type="number" min={1} placeholder="open" className="border-line num w-16 rounded-sm border px-1" value={job.term} onChange={(e) => setJob({ ...job, term: e.target.value })} />
            </Field>
            <Field label="Notice (cycles)">
              <input aria-label="Notice" type="number" min={0} className="border-line num w-16 rounded-sm border px-1" value={job.notice} onChange={(e) => setJob({ ...job, notice: Number(e.target.value) })} />
            </Field>
            <button type="submit" disabled={offer.isPending || o.workplaces.length === 0} className="bg-ink text-paper self-start rounded-sm px-3 py-1 disabled:opacity-50">
              Post job offer
            </button>
            <p className="text-muted text-xs">Payroll is due at cycle end from the treasury; short, and workers are paid pro rata and the firm is flagged.</p>
          </form>

          <div className="flex flex-col gap-6">
            {c.order_books ? (
              <form
                className="flex flex-col gap-2 text-sm"
                data-testid="ask-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  place.mutate(
                    { instrument: ask.good, side: "ask", qty: ask.qty, limit_price: cents(ask.limit), on_behalf_of: oid },
                    { onSuccess: ok(`Ask placed on the ${ask.good} book for the firm.`), onError: fail },
                  );
                }}
              >
                <h3 className="text-lg">Sell from inventory</h3>
                <Field label="Good">
                  <select aria-label="Ask good" className="border-line rounded-sm border px-1" value={ask.good} onChange={(e) => setAsk({ ...ask, good: e.target.value })}>
                    {GOODS.map((g) => (
                      <option key={g} value={g}>
                        {g} ({(o.inventory as Record<string, number>)[g] ?? 0} held)
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Quantity">
                  <input aria-label="Ask quantity" type="number" min={1} className="border-line num w-20 rounded-sm border px-1" value={ask.qty} onChange={(e) => setAsk({ ...ask, qty: Number(e.target.value) })} />
                </Field>
                <Field label="Limit">
                  <input aria-label="Ask limit" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={ask.limit} onChange={(e) => setAsk({ ...ask, limit: e.target.value })} />
                  <span className="text-muted text-xs">cr a unit; the goods sit in escrow until filled</span>
                </Field>
                <button type="submit" disabled={place.isPending || cents(ask.limit) < 1} className="border-line self-start rounded-sm border px-3 py-1 text-sm">
                  Post ask
                </button>
              </form>
            ) : null}

            <div className="flex flex-col gap-2 text-sm" data-testid="machines">
              <h3 className="text-lg">Machines</h3>
              <p className="text-muted text-xs">
                Installed machines raise the workplace's capital multiplier; they wear a little each cycle. The firm holds {(o.inventory as Record<string, number>).machines ?? 0} uninstalled.
              </p>
              <div className="flex flex-wrap items-center gap-2">
                <select aria-label="Machines workplace" className="border-line rounded-sm border px-1" value={machWp} onChange={(e) => setMach({ ...mach, workplace: Number(e.target.value) })}>
                  {o.workplaces.map((w) => (
                    <option key={w.id} value={w.id}>
                      {w.kind.replace("_", " ")} #{w.id} ({w.machines} installed)
                    </option>
                  ))}
                </select>
                <input aria-label="Machines quantity" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={mach.qty} onChange={(e) => setMach({ ...mach, qty: Number(e.target.value) })} />
                <button type="button" className="border-line rounded-sm border px-2 py-0.5" onClick={() => machines.mutate({ workplace: machWp, action: "install", qty: mach.qty }, { onSuccess: ok("Installed."), onError: fail })}>
                  Install
                </button>
                <button type="button" className="border-line rounded-sm border px-2 py-0.5" onClick={() => machines.mutate({ workplace: machWp, action: "uninstall", qty: mach.qty }, { onSuccess: ok("Uninstalled."), onError: fail })}>
                  Uninstall
                </button>
              </div>
              {c.order_books ? (
                <div className="flex flex-wrap items-center gap-2">
                  <span>Bid for machines:</span>
                  <input aria-label="Machines bid quantity" type="number" min={1} className="border-line num w-16 rounded-sm border px-1" value={buy.qty} onChange={(e) => setBuy({ ...buy, qty: Number(e.target.value) })} />
                  <span>at</span>
                  <input aria-label="Machines bid limit" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={buy.limit} onChange={(e) => setBuy({ ...buy, limit: e.target.value })} />
                  <span className="text-muted text-xs">cr, from the treasury</span>
                  <button
                    type="button"
                    disabled={cents(buy.limit) < 1}
                    className="border-line rounded-sm border px-2 py-0.5 disabled:opacity-50"
                    onClick={() =>
                      place.mutate(
                        { instrument: "machines", side: "bid", qty: buy.qty, limit_price: cents(buy.limit), on_behalf_of: oid },
                        { onSuccess: ok("Bid placed on the machines book for the firm."), onError: fail },
                      )
                    }
                  >
                    Place bid
                  </button>
                </div>
              ) : null}
            </div>

            <div className="flex flex-col gap-2 text-sm" data-testid="add-workplace">
              <h3 className="text-lg">Add a workplace</h3>
              <div className="flex flex-wrap items-center gap-2">
                <select aria-label="New workplace kind" className="border-line rounded-sm border px-1" value={newKind} onChange={(e) => setNewKind(e.target.value)}>
                  {WORKPLACE_KINDS.map((k) => {
                    const s = slots[k];
                    return (
                      <option key={k} value={k} disabled={s?.total != null && (s.free ?? 0) === 0}>
                        {k.replace("_", " ")} {s?.total == null ? "" : `(${s.free ?? 0} of ${s.total} slots free)`}
                      </option>
                    );
                  })}
                </select>
                <button type="button" className="border-line rounded-sm border px-2 py-0.5" onClick={() => addWorkplace.mutate({ kind: newKind }, { onSuccess: ok("Workplace added."), onError: fail })}>
                  Add
                </button>
                <span className="text-muted text-xs">{founding ? `${founding.materials} Materials from the firm's inventory` : ""}</span>
              </div>
            </div>
          </div>
        </section>
      ) : null}

      {shares && controlling ? (
        <section className="grid gap-8 md:grid-cols-3 text-sm" data-testid="owner-tools">
          {c.money ? (
            <div className="flex flex-col gap-2">
              <h3 className="text-lg">Dividend</h3>
              <Field label="Per share">
                <input aria-label="Dividend per share" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={perShare} onChange={(e) => setPerShare(e.target.value)} />
              </Field>
              <p className="text-muted text-xs">
                {credits(cents(perShare) * shares.issued)} cr from the treasury, split by share count.
              </p>
              <button type="button" className="border-line self-start rounded-sm border px-2 py-0.5" onClick={() => dividend.mutate(cents(perShare), { onSuccess: ok("Dividend declared."), onError: fail })}>
                Declare
              </button>
            </div>
          ) : null}
          <div className="flex flex-col gap-2">
            <h3 className="text-lg">Issue shares</h3>
            <Field label="Quantity">
              <input aria-label="Issue quantity" type="number" min={1} className="border-line num w-20 rounded-sm border px-1" value={issueQty} onChange={(e) => setIssueQty(Number(e.target.value))} />
            </Field>
            <p className="text-muted text-xs">New shares land in the firm's own holding, to be sold on its behalf.</p>
            <button type="button" className="border-line self-start rounded-sm border px-2 py-0.5" onClick={() => issue.mutate(issueQty, { onSuccess: ok("Shares issued."), onError: fail })}>
              Issue
            </button>
          </div>
          <div className="flex flex-col gap-2">
            <h3 className="text-lg">List for sale</h3>
            <Field label="Shares">
              <input aria-label="Listing quantity" type="number" min={1} max={o.my_shares} className="border-line num w-20 rounded-sm border px-1" value={listing.qty} onChange={(e) => setListing({ ...listing, qty: Number(e.target.value) })} />
            </Field>
            <Field label="Price">
              <input aria-label="Listing price" type="number" step="0.01" min={0.01} className="border-line num w-24 rounded-sm border px-1" value={listing.price} onChange={(e) => setListing({ ...listing, price: e.target.value })} />
              <span className="text-muted text-xs">cr for the lot</span>
            </Field>
            <p className="text-muted text-xs">A sale offer on the notice board for your own shares; the order book takes them too.</p>
            <button
              type="button"
              disabled={cents(listing.price) < 1}
              className="border-line self-start rounded-sm border px-2 py-0.5 disabled:opacity-50"
              onClick={() =>
                sale.mutate(
                  { asset: { shares: [oid, listing.qty] }, price: { money: cents(listing.price) } },
                  { onSuccess: ok("Listed on the notice board."), onError: fail },
                )
              }
            >
              List
            </button>
          </div>
          <div className="flex flex-col gap-2">
            <h3 className="text-lg">Manager</h3>
            <Field label="Citizen id">
              <input aria-label="Manager citizen" type="number" min={1} placeholder={String(o.manager ?? "")} className="border-line num w-20 rounded-sm border px-1" value={manager} onChange={(e) => setManager(e.target.value)} />
            </Field>
            <div className="flex gap-2">
              <button type="button" disabled={manager.trim() === ""} className="border-line rounded-sm border px-2 py-0.5 disabled:opacity-50" onClick={() => appoint.mutate(Number(manager), { onSuccess: ok("Manager appointed."), onError: fail })}>
                Appoint
              </button>
              <button type="button" className="text-muted text-xs underline" onClick={() => appoint.mutate(null, { onSuccess: ok("Manager vacated."), onError: fail })}>
                vacate
              </button>
            </div>
          </div>
        </section>
      ) : null}
    </div>
  );
}
