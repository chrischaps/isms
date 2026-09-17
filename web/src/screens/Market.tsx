// Market (GDD 6.1, TDD 11; S1.10): the instrument list with last price and
// the index, one book at a time (ladders, depth, tape, price history), an
// order form with the escrow preview, and your open orders. The pantry and
// Wares rules from the plan are surfaced where they bear on an order. This
// screen mounts only where `order_books` is on.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits } from "../api/client";
import { useHome, useLexicon } from "../api/hooks";
import {
  useBook,
  useBooks,
  useCancelOrder,
  useOrgs,
  usePlaceOrder,
  usePrices,
  type BookSummary,
  type OrgView,
} from "../api/market";
import { OrderBook } from "../components/OrderBook";
import { TimeSeries } from "../components/TimeSeries";
import { useNames } from "../lib/names";
import { deadlineOfTick, whenOf } from "../lib/when";

const GOOD_ORDER = ["food", "wares", "grain", "ore", "materials", "machines"];
type Side = "bid" | "ask";

function goodOf(instrument: string): string | null {
  return instrument.startsWith("share:") ? null : instrument;
}

function orgOf(instrument: string): number | null {
  return instrument.startsWith("share:") ? Number(instrument.slice(6)) : null;
}

function label(instrument: string, orgs: OrgView[] | undefined): string {
  const org = orgOf(instrument);
  if (org === null) return instrument;
  return `shares of ${orgs?.find((o) => o.id === org)?.name ?? `organization no. ${org}`}`;
}

/** Change over the window: first VWAP to last, as a signed percentage. */
function change(points: { instrument: string; tick: number; vwap: number }[], instrument: string): number | null {
  const mine = points.filter((p) => p.instrument === instrument);
  if (mine.length < 2) return null;
  const first = mine[0]!.vwap;
  const last = mine[mine.length - 1]!.vwap;
  return first > 0 ? ((last - first) / first) * 100 : null;
}

export function Market({ id, instrument = "food" }: { id: number; instrument?: string }) {
  const { t } = useLexicon(id);
  const home = useHome(id);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const books = useBooks(id);
  const book = useBook(id, instrument);
  const orgs = useOrgs(id);
  const ticksPerCycle = home.data?.clock.ticks_per_cycle ?? 24;
  const window = ticksPerCycle * 3;
  const prices = usePrices(id, window);
  const place = usePlaceOrder(id);
  const cancel = useCancelOrder(id);
  const [side, setSide] = useState<Side>("bid");
  const [qty, setQty] = useState(1);
  const [limit, setLimit] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [placed, setPlaced] = useState<string | null>(null);

  if (home.isPending || books.isPending) return <p className="text-muted">Loading.</p>;
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
  if (home.error || books.error) return <p className="text-bad">Could not load: {String(home.error ?? books.error)}</p>;
  const h = home.data!;
  const plan = h.plan as Record<string, unknown>;
  const floor = Number(plan.keep_balance_at_least ?? 0);
  const wares = plan.buy_wares_when as { comfort_below: number; balance_above: number; max_price: number | null } | null;
  const pantry = h.household.pantry as Record<string, number>;
  const capacity = (h.household.pantry_capacity ?? {}) as Record<string, number>;

  // The list: goods in a fixed order, then every firm with a share registry
  // (whether or not its book has ever had an order).
  const summaries = new Map<string, BookSummary>(books.data!.books.map((b) => [b.instrument, b]));
  const goods = GOOD_ORDER.filter((g) => summaries.has(g)).concat(
    [...summaries.keys()].filter((k) => !k.startsWith("share:") && !GOOD_ORDER.includes(k)),
  );
  const shares = (orgs.data ?? [])
    .filter((o) => "shares" in (o.ownership as Record<string, unknown>))
    .map((o) => ({ org: o, instrument: `share:${o.id}`, summary: summaries.get(`share:${o.id}`) }));
  const points = prices.data?.points ?? [];

  const b = book.data;
  const last = b?.last_price ?? summaries.get(instrument)?.last_price ?? null;
  const good = goodOf(instrument);
  const limitCents = limit === null ? (last ?? 0) : Math.round(Number(limit) * 100) || 0;
  const held = good ? (pantry[good] ?? 0) : orgOf(instrument) !== null ? (orgs.data?.find((o) => o.id === orgOf(instrument))?.my_shares ?? 0) : 0;
  const openBids = (b?.my_orders ?? []).filter((o) => o.side === "bid").reduce((n, o) => n + o.remaining, 0);
  const room = good && capacity[good] !== undefined ? Math.max(0, capacity[good] - (pantry[good] ?? 0) - openBids) : null;
  const escrow = side === "bid" ? qty * limitCents : null;
  const after = escrow !== null ? h.household.balance - escrow : null;
  const nextCycleEnd = (Math.floor(h.clock.engine_tick / ticksPerCycle) + 2) * ticksPerCycle - 1;

  // The chart: this instrument's VWAP per tick over the window; gaps stay gaps.
  const mine = points.filter((p) => p.instrument === instrument);
  const ticks = mine.map((p) => p.tick);
  const series = [{ label: label(instrument, orgs.data), values: mine.map((p) => p.vwap / 100) }];

  const submit = () => {
    setError(null);
    setPlaced(null);
    place.mutate(
      { instrument, side, qty, limit_price: limitCents },
      {
        onSuccess: (r) => {
          const fills = r.events.filter((e) => e.kind === "Trade");
          const filled = fills.reduce((n, e) => n + Number((e.payload as Record<string, Record<string, unknown>>).Trade?.qty ?? 0), 0);
          setPlaced(
            filled > 0
              ? `Placed; filled ${filled} of ${qty} at once.`
              : `Placed; resting on the book until ${deadlineOfTick(nextCycleEnd, ticksPerCycle)}.`,
          );
          setLimit(null);
        },
        onError: (e) => setError(e.message),
      },
    );
  };

  const row = (name: string, inst: string, s: BookSummary | undefined, extra?: string) => {
    const c = change(points, inst);
    const active = inst === instrument;
    return (
      <tr key={inst} className={`rule ${active ? "text-ink" : "text-muted"}`}>
        <td className="py-1 pr-3">
          <Link to="/s/$id/market/$instrument" params={{ id: String(id), instrument: inst }} className={active ? "" : "underline"}>
            {name}
          </Link>
          {extra ? <span className="text-muted block text-xs">{extra}</span> : null}
        </td>
        <td className="num py-1 pr-3 text-right">{s?.last_price != null ? credits(s.last_price) : "—"}</td>
        <td className={`num py-1 text-right ${c === null ? "text-muted" : c >= 0 ? "text-good" : "text-bad"}`}>
          {c === null ? "—" : `${c >= 0 ? "+" : ""}${c.toFixed(1)}%`}
        </td>
      </tr>
    );
  };

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{t("store")}</h2>
        {books.data!.price_index != null ? (
          <p className="num text-muted text-sm">
            {t("society_stat")} {books.data!.price_index.toFixed(2)} <span className="text-xs">(reference basket, Food = 1)</span>
          </p>
        ) : null}
      </header>

      <div className="grid gap-8 md:grid-cols-[16rem_1fr]">
        <aside>
          <h3 className="text-lg">Instruments</h3>
          <table className="mt-2 w-full text-sm" data-testid="instruments">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">Good</th>
                <th className="py-1 text-right font-normal">Last</th>
                <th className="py-1 text-right font-normal">3 days</th>
              </tr>
            </thead>
            <tbody>{goods.map((g) => row(g, g, summaries.get(g)))}</tbody>
          </table>
          {shares.length > 0 ? (
            <table className="mt-4 w-full text-sm" data-testid="share-instruments">
              <thead className="text-muted text-left text-xs uppercase tracking-wide">
                <tr>
                  <th className="py-1 font-normal">Shares</th>
                  <th className="py-1 text-right font-normal">Last</th>
                  <th className="py-1 text-right font-normal">3 days</th>
                </tr>
              </thead>
              <tbody>
                {shares.map((s) =>
                  row(
                    s.org.name,
                    s.instrument,
                    s.summary,
                    `book ${credits(s.org.book_value)} cr${s.org.my_shares > 0 ? ` · you hold ${s.org.my_shares}` : ""}`,
                  ),
                )}
              </tbody>
            </table>
          ) : null}
        </aside>

        <section className="flex flex-col gap-6">
          <header className="flex flex-wrap items-baseline justify-between gap-3">
            <h3 className="text-lg">{label(instrument, orgs.data)}</h3>
            <p className="num text-muted text-sm">
              {b ? (
                <>
                  spread {b.bids[0] ? credits(b.bids[0].price) : "—"} / {b.asks[0] ? credits(b.asks[0].price) : "—"}
                  {last != null ? ` · last ${credits(last)}` : ""}
                </>
              ) : (
                "Loading."
              )}
            </p>
          </header>

          {b ? (
            <div className="grid gap-6 md:grid-cols-2">
              <div className="grid grid-cols-2 gap-4 text-sm" data-testid="ladders">
                <table>
                  <thead className="text-muted text-left text-xs uppercase tracking-wide">
                    <tr>
                      <th className="py-1 font-normal">Bid qty</th>
                      <th className="py-1 text-right font-normal">Bid</th>
                    </tr>
                  </thead>
                  <tbody>
                    {b.bids.slice(0, 8).map((l) => {
                      const yours = b.my_orders.filter((o) => o.side === "bid" && o.limit_price === l.price).reduce((n, o) => n + o.remaining, 0);
                      return (
                        <tr key={l.price} className="rule">
                          <td className="num py-0.5">
                            {l.qty}
                            {yours > 0 ? <span className="text-muted"> ({yours} yours)</span> : null}
                          </td>
                          <td className="num text-good py-0.5 text-right">{credits(l.price)}</td>
                        </tr>
                      );
                    })}
                    {b.bids.length === 0 ? (
                      <tr>
                        <td colSpan={2} className="text-muted py-0.5">
                          no bids
                        </td>
                      </tr>
                    ) : null}
                  </tbody>
                </table>
                <table>
                  <thead className="text-muted text-left text-xs uppercase tracking-wide">
                    <tr>
                      <th className="py-1 font-normal">Ask</th>
                      <th className="py-1 text-right font-normal">Ask qty</th>
                    </tr>
                  </thead>
                  <tbody>
                    {b.asks.slice(0, 8).map((l) => {
                      const yours = b.my_orders.filter((o) => o.side === "ask" && o.limit_price === l.price).reduce((n, o) => n + o.remaining, 0);
                      return (
                        <tr key={l.price} className="rule">
                          <td className="num text-accent py-0.5">{credits(l.price)}</td>
                          <td className="num py-0.5 text-right">
                            {l.qty}
                            {yours > 0 ? <span className="text-muted"> ({yours} yours)</span> : null}
                          </td>
                        </tr>
                      );
                    })}
                    {b.asks.length === 0 ? (
                      <tr>
                        <td colSpan={2} className="text-muted py-0.5">
                          no asks
                        </td>
                      </tr>
                    ) : null}
                  </tbody>
                </table>
              </div>
              <div className="flex flex-col gap-4">
                <OrderBook bids={b.bids} asks={b.asks} />
                <div>
                  <h4 className="text-muted text-xs uppercase tracking-wide">Tape</h4>
                  <table className="mt-1 w-full text-sm" data-testid="tape">
                    <tbody>
                      {b.tape
                        .slice()
                        .reverse()
                        .slice(0, 10)
                        .map((e) => {
                          const tr = ((e.payload as Record<string, unknown>).Trade ?? {}) as Record<string, unknown>;
                          const buyer = names.party(tr.buyer);
                          const seller = names.party(tr.seller);
                          const yours = buyer === "you" || seller === "you";
                          return (
                            <tr key={e.seq} className={`rule ${yours ? "text-ink" : "text-muted"}`}>
                              <td className="num py-0.5 pr-2 whitespace-nowrap">{whenOf(e, ticksPerCycle)}</td>
                              <td className="num py-0.5 pr-2 text-right">{credits(Number(tr.price ?? 0))}</td>
                              <td className="num py-0.5 pr-2 text-right">{String(tr.qty ?? "")}</td>
                              <td className="py-0.5 text-xs">
                                {buyer} bought from {seller}
                              </td>
                            </tr>
                          );
                        })}
                      {b.tape.length === 0 ? (
                        <tr>
                          <td className="text-muted py-0.5">No trades yet.</td>
                        </tr>
                      ) : null}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          ) : null}

          <div>
            <h4 className="text-muted text-xs uppercase tracking-wide">Last 3 days</h4>
            {ticks.length > 1 ? (
              <TimeSeries ticks={ticks} series={series} height={140} />
            ) : (
              <p className="text-muted mt-1 text-sm">Not enough trades to draw yet.</p>
            )}
          </div>

          <div className="grid gap-6 md:grid-cols-2">
            <form
              className="flex flex-col gap-3 text-sm"
              data-testid="order-form"
              onSubmit={(e) => {
                e.preventDefault();
                submit();
              }}
            >
              <h4 className="text-lg">Place an order</h4>
              <div className="flex gap-4" role="radiogroup" aria-label="Side">
                {(["bid", "ask"] as Side[]).map((s) => (
                  <label key={s} className="flex items-center gap-1">
                    <input type="radio" name="side" value={s} checked={side === s} onChange={() => setSide(s)} />
                    {s === "bid" ? "Bid (buy)" : "Ask (sell)"}
                  </label>
                ))}
              </div>
              <label className="flex items-center gap-2">
                <span className="w-20">Quantity</span>
                <input
                  type="number"
                  inputMode="numeric"
                  min={1}
                  step={1}
                  aria-label="Quantity"
                  className="border-line num w-20 rounded-sm border px-1"
                  value={qty}
                  onChange={(e) => setQty(Math.max(0, Math.trunc(Number(e.target.value) || 0)))}
                />
                <span className="text-muted text-xs">
                  {side === "bid" && room !== null
                    ? `pantry room ${room} of ${capacity[good!]}${openBids > 0 ? ` (${openBids} on open bids)` : ""}`
                    : side === "ask"
                      ? `you hold ${held}`
                      : ""}
                </span>
              </label>
              <label className="flex items-center gap-2">
                <span className="w-20">Limit</span>
                <input
                  type="number"
                  inputMode="decimal"
                  min={0.01}
                  step="0.01"
                  aria-label="Limit price"
                  className="border-line num w-24 rounded-sm border px-1"
                  value={limit ?? (last != null ? (last / 100).toFixed(2) : "")}
                  onChange={(e) => setLimit(e.target.value)}
                />
                <span className="text-muted text-xs">cr{last != null ? ` · last ${credits(last)}` : ""}</span>
              </label>
              <p className="text-muted text-xs">Expires at {deadlineOfTick(nextCycleEnd, ticksPerCycle)} unless filled or cancelled.</p>
              <div className="bg-paper-2 rounded-sm p-2 text-xs" data-testid="escrow-preview">
                {side === "bid" ? (
                  <>
                    Holds <span className="num">{credits(escrow ?? 0)} cr</span> now; balance after{" "}
                    <span className={`num ${after !== null && after < floor ? "text-warn" : ""}`}>{credits(after ?? 0)} cr</span>
                    {floor > 0 ? (after !== null && after < floor ? `, below your ${credits(floor)} floor` : `, above your ${credits(floor)} floor`) : ""}
                    . Fills settle as they match; unfilled escrow returns on cancel or expiry.
                  </>
                ) : (
                  <>
                    Holds <span className="num">{qty}</span> {good ?? "shares"} out of your {good ? t("pantry").toLowerCase() : "holding"} until filled, cancelled or expired.
                  </>
                )}
              </div>
              <div className="flex items-baseline gap-3">
                <button type="submit" disabled={place.isPending || qty < 1 || limitCents < 1} className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50">
                  Place {side}
                </button>
                {placed ? <span className="text-muted">{placed}</span> : null}
                {error ? (
                  <span className="text-bad" role="alert">
                    {error}
                  </span>
                ) : null}
              </div>
              {good === "food" ? (
                <p className="text-muted text-xs">
                  Your {t("plan").toLowerCase()} keeps Food at least {String(plan.keep_food_at_least)} and bids for the shortfall at up to{" "}
                  {plan.max_food_price == null ? "last price x 1.25" : `${credits(Number(plan.max_food_price))} cr`} each hour.
                </p>
              ) : null}
              {good === "wares" ? (
                <p className="text-muted text-xs">
                  {wares
                    ? `Your ${t("plan").toLowerCase()} buys Wares when Comfort is below ${wares.comfort_below} and balance above ${credits(wares.balance_above)} cr${wares.max_price != null ? `, at up to ${credits(wares.max_price)} cr` : ""}.`
                    : `Your ${t("plan").toLowerCase()} has no Wares rule; Comfort holds only while you buy by hand.`}
                </p>
              ) : null}
            </form>

            <div>
              <h4 className="text-lg">Your open orders</h4>
              {b && b.my_orders.length > 0 ? (
                <table className="mt-2 w-full text-sm" data-testid="my-orders">
                  <thead className="text-muted text-left text-xs uppercase tracking-wide">
                    <tr>
                      <th className="py-1 font-normal">Side</th>
                      <th className="py-1 text-right font-normal">Left</th>
                      <th className="py-1 text-right font-normal">Limit</th>
                      <th className="py-1 font-normal">Source</th>
                      <th className="py-1 font-normal" />
                    </tr>
                  </thead>
                  <tbody>
                    {b.my_orders.map((o) => (
                      <tr key={o.id} className="rule">
                        <td className="py-1">{o.side}</td>
                        <td className="num py-1 text-right">
                          {o.remaining} of {o.qty}
                        </td>
                        <td className="num py-1 text-right">{credits(o.limit_price)}</td>
                        <td className="text-muted py-1">{o.source}</td>
                        <td className="py-1 text-right">
                          <button
                            type="button"
                            disabled={cancel.isPending}
                            className="text-muted text-xs underline"
                            onClick={() => cancel.mutate(o.id, { onError: (e) => setError(e.message) })}
                          >
                            cancel
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <p className="text-muted mt-2 text-sm">None on this book. Standing orders from the plan and hand-placed orders share this list.</p>
              )}
              <p className="text-muted mt-3 text-xs" data-testid="pantry-line">
                {t("pantry")}: {Object.entries(pantry).map(([g, n]) => `${n} ${g}`).join(", ") || "empty"}
                {good && capacity[good] !== undefined ? ` · ${good} cap ${capacity[good]}` : ""}
              </p>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
