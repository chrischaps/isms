// Market (GDD 6.1, TDD 11; S1.10; docs/style.md §10 Market): the instrument
// list with last price and the index, one book at a time (the Verdict, the
// last price and the 3-day change at a glance, then ladders, depth, tape and
// price history as tabs on a phone and side by side from md), an order form
// with the escrow preview, your open orders, and where the good comes from.
// The pantry and Wares rules from the plan are surfaced where they bear on an
// order. This screen mounts only where `order_books` is on.

import { Fragment, useState, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits } from "../api/client";
import { useHome, useLexicon } from "../api/hooks";
import { useBook, useBooks, useCancelOrder, useOrgs, usePlaceOrder, usePrices, type BookSummary, type OrgView } from "../api/market";
import { useOrgsView } from "../api/orgs";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { FactList } from "../components/FactList";
import { Field, Input } from "../components/Field";
import { EpochDivider, TH, TH_NUM } from "../components/Ledger";
import { More } from "../components/More";
import { OrderBook } from "../components/OrderBook";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Provenance } from "../components/Provenance";
import { Tabs } from "../components/Tabs";
import { TimeSeries } from "../components/TimeSeries";
import { Verdict } from "../components/Verdict";
import { useNames } from "../lib/names";
import { marketVerdict } from "../lib/verdict";
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

/** The 3-day change as a pill: a figure, not a state, so never good or crit. */
function ChangePill({ c, days }: { c: number | null; days: number }) {
  if (c === null) return <Pill>no {days}-day change yet</Pill>;
  if (Math.abs(c) < 0.5) return <Pill>steady {days} days</Pill>;
  return (
    <Pill tone="info">
      {c > 0 ? "+" : "−"}
      {Math.abs(c).toFixed(1)} % in {days} days
    </Pill>
  );
}

/** A ladder cell: tighter than a ledger row, since eight levels sit in a card. */
const LADDER = "border-line border-b px-2 py-1 tabular-nums";

export function Market({ id, instrument = "food" }: { id: number; instrument?: string }) {
  const { t } = useLexicon(id);
  const home = useHome(id);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const books = useBooks(id);
  const book = useBook(id, instrument);
  const orgs = useOrgs(id);
  // The same cache entry, whole: the recipes and every org's workplaces and stock.
  const directory = useOrgsView(id);
  const ticksPerCycle = home.data?.clock.ticks_per_cycle ?? 24;
  const days = 3;
  const window = ticksPerCycle * days;
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
        <Link to="/s/$id" params={{ id: String(id) }}>
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error || books.error) return <p className="text-crit">Could not load: {String(home.error ?? books.error)}</p>;
  const h = home.data!;
  const plan = h.plan as Record<string, unknown>;
  const floor = Number(plan.keep_balance_at_least ?? 0);
  const keepFood = Number(plan.keep_food_at_least ?? 0);
  const wares = plan.buy_wares_when as { comfort_below: number; balance_above: number; max_price: number | null } | null;
  const pantry = h.household.pantry as Record<string, number>;
  const capacity = (h.household.pantry_capacity ?? {}) as Record<string, number>;

  // The list: goods in a fixed order, then every firm with a share registry
  // (whether or not its book has ever had an order).
  const summaries = new Map<string, BookSummary>(books.data!.books.map((b) => [b.instrument, b]));
  const goods = GOOD_ORDER.filter((g) => summaries.has(g)).concat([...summaries.keys()].filter((k) => !k.startsWith("share:") && !GOOD_ORDER.includes(k)));
  const shares = (orgs.data ?? [])
    .filter((o) => "shares" in (o.ownership as Record<string, unknown>))
    .map((o) => ({ org: o, instrument: `share:${o.id}`, summary: summaries.get(`share:${o.id}`) }));
  const points = prices.data?.points ?? [];

  const b = book.data;
  const last = b?.last_price ?? summaries.get(instrument)?.last_price ?? null;
  const good = goodOf(instrument);
  const name = label(instrument, orgs.data);
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
  // An empty ask side with a busy tape: the good is selling out as it is posted, not absent.
  const tapeTrades = (b?.tape ?? []).map((e) => ({ epoch: e.epoch, tick: e.tick, ...(((e.payload as Record<string, unknown>).Trade ?? {}) as { qty?: number; price?: number }) }));
  const newest = tapeTrades[tapeTrades.length - 1];
  const lastHour = newest ? tapeTrades.filter((x) => x.epoch === newest.epoch && x.tick === newest.tick) : [];
  const soldOut =
    b && b.asks.length === 0 && newest && newest.epoch + 1 === h.clock.epoch && lastHour.length > 0
      ? { qty: lastHour.reduce((n, x) => n + Number(x.qty ?? 0), 0), price: Number(newest.price ?? 0) }
      : null;
  const series = [{ label: name, values: mine.map((p) => p.vwap / 100) }];
  const c = change(points, instrument);
  const verdict = marketVerdict({
    name,
    isShare: good === null,
    last,
    change: c,
    days,
    soldOut: soldOut !== null,
    noAsks: b !== undefined && b.asks.length === 0,
    food: good === "food" ? { keepFood, pantryFood: pantry.food ?? 0 } : undefined,
    wares: good === "wares" ? (wares ? { comfortBelow: wares.comfort_below } : null) : undefined,
    openOrders: b?.my_orders.length ?? 0,
  });

  const submit = () => {
    setError(null);
    setPlaced(null);
    place.mutate(
      { instrument, side, qty, limit_price: limitCents },
      {
        onSuccess: (r) => {
          const fills = r.events.filter((e) => e.kind === "Trade");
          const filled = fills.reduce((n, e) => n + Number((e.payload as Record<string, Record<string, unknown>>).Trade?.qty ?? 0), 0);
          setPlaced(filled > 0 ? `Placed; filled ${filled} of ${qty} at once.` : `Placed; resting on the book until ${deadlineOfTick(nextCycleEnd, ticksPerCycle)}.`);
          setLimit(null);
        },
        onError: (e) => setError(e.message),
      },
    );
  };

  // One item of the instrument list: a chip on a phone, a row with its figures from md.
  const item = (text: string, inst: string, s: BookSummary | undefined, sub?: ReactNode) => {
    const active = inst === instrument;
    const ch = change(points, inst);
    return (
      <li key={inst} className="shrink-0">
        <Link
          to="/s/$id/market/$instrument"
          params={{ id: String(id), instrument: inst }}
          aria-current={active ? "page" : undefined}
          className={[
            "flex min-h-touch items-center gap-2 rounded-pill border px-3.5 py-1.5 hover:no-underline md:justify-between md:rounded-md md:px-2.5",
            active ? "bg-accent-soft border-accent-soft text-accent font-bold" : "border-line text-ink md:border-transparent",
          ].join(" ")}
        >
          <span className="min-w-0">
            {text}
            {sub ? <span className="text-muted hidden text-xs font-normal md:block">{sub}</span> : null}
          </span>
          <span className="text-muted text-sm tabular-nums">
            {s?.last_price != null ? credits(s.last_price) : "—"}
            <span className="hidden md:inline">{ch === null || Math.abs(ch) < 0.05 ? "" : ` · ${ch > 0 ? "+" : "−"}${Math.abs(ch).toFixed(1)} %`}</span>
          </span>
        </Link>
      </li>
    );
  };
  const listClass = "m-0 flex list-none gap-2 overflow-x-auto p-0 md:grid md:gap-0.5 md:overflow-visible";

  const ladder = (levels: { price: number; qty: number }[], s: Side) => (
    <table className="w-full border-collapse text-sm">
      <thead>
        <tr>
          {s === "bid" ? (
            <>
              <th className={TH}>Bid qty</th>
              <th className={TH_NUM}>Bid</th>
            </>
          ) : (
            <>
              <th className={TH}>Ask</th>
              <th className={TH_NUM}>Ask qty</th>
            </>
          )}
        </tr>
      </thead>
      <tbody>
        {levels.slice(0, 8).map((l) => {
          const yours = b!.my_orders.filter((o) => o.side === s && o.limit_price === l.price).reduce((n, o) => n + o.remaining, 0);
          const q = (
            <>
              {l.qty}
              {yours > 0 ? (
                <>
                  {" "}
                  <Pill>you {yours}</Pill>
                </>
              ) : null}
            </>
          );
          const p = <span className={s === "bid" ? "text-good" : "text-crit"}>{credits(l.price)}</span>;
          return (
            <tr key={l.price}>
              <td className={LADDER}>{s === "bid" ? q : p}</td>
              <td className={`${LADDER} text-right`}>{s === "bid" ? p : q}</td>
            </tr>
          );
        })}
        {levels.length === 0 ? (
          <tr>
            <td colSpan={2} className="text-muted px-2 py-1">
              no {s}s
            </td>
          </tr>
        ) : null}
      </tbody>
    </table>
  );

  const tapeRows = b ? b.tape.slice().reverse() : [];
  const tape = (rows: typeof tapeRows, testId?: string) => (
    <table className="w-full border-collapse text-sm" data-testid={testId}>
      <tbody>
        {rows.map((e, i, shown) => {
          const tr = ((e.payload as Record<string, unknown>).Trade ?? {}) as Record<string, unknown>;
          const buyer = names.party(tr.buyer);
          const seller = names.party(tr.seller);
          const yours = buyer === "you" || seller === "you";
          // Last epoch's trades stay on the tape until this one's push them off; mark where they start.
          const spans = shown.some((x) => x.epoch !== e.epoch);
          const first = e.epoch !== shown[i - 1]?.epoch;
          return (
            <Fragment key={e.seq}>
              {spans && first ? <EpochDivider epoch={e.epoch} current={e.epoch + 1 === h.clock.epoch} span={4} /> : null}
              <tr className={yours ? "text-ink" : "text-muted"}>
                <td className={`${LADDER} whitespace-nowrap`}>{whenOf(e, ticksPerCycle)}</td>
                <td className={`${LADDER} text-right`}>{credits(Number(tr.price ?? 0))}</td>
                <td className={`${LADDER} text-right`}>{String(tr.qty ?? "")}</td>
                <td className={`${LADDER} text-xs`}>
                  {buyer} bought from {seller}
                </td>
              </tr>
            </Fragment>
          );
        })}
        {rows.length === 0 ? (
          <tr>
            <td className="text-muted px-2 py-1">No trades yet.</td>
          </tr>
        ) : null}
      </tbody>
    </table>
  );

  return (
    <div>
      <PageHeader
        title={t("store_screen")}
        meta={
          books.data!.price_index != null ? (
            <span>
              {t("society_stat").toLowerCase()} <b>{books.data!.price_index.toFixed(2)}</b> <span className="text-xs">(reference basket, Food = 1)</span>
            </span>
          ) : undefined
        }
      />

      <div className="grid grid-cols-[minmax(0,1fr)] gap-4 md:grid-cols-[16rem_minmax(0,1fr)] md:items-start">
        <Card title="Instruments" icon="market" className="min-w-0">
          <ul className={listClass} data-testid="instruments" aria-label="Goods">
            {goods.map((g) => item(g, g, summaries.get(g)))}
          </ul>
          {shares.length > 0 ? (
            <>
              <h3 className="text-muted mt-4 mb-1 text-xs font-bold tracking-caps uppercase">Shares</h3>
              <ul className={listClass} data-testid="share-instruments" aria-label="Shares">
                {shares.map((s) => item(s.org.name, s.instrument, s.summary, `book ${credits(s.org.book_value)} cr${s.org.my_shares > 0 ? ` · you hold ${s.org.my_shares}` : ""}`))}
              </ul>
              <More summary="All shares" testId="all-shares">
                <FactList
                  items={shares.map((s) => ({
                    key: s.instrument,
                    label: s.org.name,
                    value: `book ${credits(s.org.book_value)} cr`,
                    gloss: s.org.my_shares > 0 ? `you hold ${s.org.my_shares}` : undefined,
                  }))}
                />
              </More>
            </>
          ) : null}
        </Card>

        <Stack className="min-w-0">
          <Card title={name} icon="market" aside={<ChangePill c={c} days={days} />} testId="instrument">
            <Verdict parts={verdict} />
            <div className="grid gap-3 sm:grid-cols-3" data-testid="glance">
              <Tile>
                <div className="text-muted text-xs font-bold tracking-caps uppercase">Last price</div>
                <div className="font-display text-2xl leading-none font-bold">
                  {last != null ? credits(last) : "—"}
                  <small className="text-muted ml-1 font-body text-xs font-normal">cr</small>
                </div>
              </Tile>
              <Tile>
                <div className="text-muted text-xs font-bold tracking-caps uppercase">Spread</div>
                <div className="font-display text-2xl leading-none font-bold tabular-nums">
                  {b ? (
                    <>
                      {b.bids[0] ? credits(b.bids[0].price) : "—"}
                      <small className="text-muted mx-1 font-body text-xs font-normal">bid / ask</small>
                      {b.asks[0] ? credits(b.asks[0].price) : "—"}
                    </>
                  ) : (
                    "…"
                  )}
                </div>
              </Tile>
              <Tile>
                <div className="text-muted text-xs font-bold tracking-caps uppercase">{good ? `Your ${t("pantry").toLowerCase()}` : "You hold"}</div>
                <div className="font-display text-2xl leading-none font-bold">
                  {held}
                  <small className="text-muted ml-1 font-body text-xs font-normal">{good ?? "shares"}{good && capacity[good] !== undefined ? ` of ${capacity[good]}` : ""}</small>
                </div>
              </Tile>
            </div>

            {b ? (
              <div className="mt-4">
                <Tabs
                  label={`${name} book`}
                  layout="md:grid-cols-2"
                  tabs={[
                    {
                      key: "book",
                      label: "Book",
                      node: (
                        <div className="grid gap-4">
                          <div className="grid grid-cols-2 gap-3" data-testid="ladders">
                            {ladder(b.bids, "bid")}
                            {ladder(b.asks, "ask")}
                          </div>
                          <OrderBook bids={b.bids} asks={b.asks} />
                        </div>
                      ),
                    },
                    {
                      key: "tape",
                      label: "Tape",
                      node: (
                        <div className="overflow-x-auto">
                          {tape(tapeRows.slice(0, 10), "tape")}
                          {tapeRows.length > 10 ? (
                            <More summary={`Earlier trades (${tapeRows.length - 10} more)`} testId="tape-more">
                              <div className="overflow-x-auto">{tape(tapeRows.slice(10))}</div>
                            </More>
                          ) : null}
                        </div>
                      ),
                    },
                    {
                      key: "chart",
                      label: `Last ${days} days`,
                      className: "md:col-span-2",
                      node: ticks.length > 1 ? <TimeSeries ticks={ticks} series={series} height={140} /> : <p className="text-muted m-0 text-sm">Not enough trades to draw yet.</p>,
                    },
                  ]}
                />
              </div>
            ) : (
              <p className="text-muted mt-4">Loading.</p>
            )}

            {soldOut ? (
              <p className="mt-4 text-sm" data-testid="sold-out-note">
                Nothing is on offer, yet {soldOut.qty} changed hands in the last hour traded, at {credits(soldOut.price)} cr: what sellers post is met at once by bids
                already waiting. A bid of yours at or above that price waits in the same line, best price first, then oldest.
              </p>
            ) : null}
          </Card>

          <Two>
            <Card title="Place an order" icon="coin">
              <form
                className="grid gap-3"
                data-testid="order-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  submit();
                }}
              >
                <div className="flex gap-4" role="radiogroup" aria-label="Side">
                  {(["bid", "ask"] as Side[]).map((s) => (
                    <label key={s} className="flex min-h-touch items-center gap-2">
                      <input type="radio" name="side" value={s} checked={side === s} onChange={() => setSide(s)} />
                      {s === "bid" ? "Bid (buy)" : "Ask (sell)"}
                    </label>
                  ))}
                </div>
                <Field
                  label="Quantity"
                  unit={good ?? "shares"}
                  hint={
                    side === "bid" && room !== null
                      ? `${t("pantry").toLowerCase()} room ${room} of ${capacity[good!]}${openBids > 0 ? ` (${openBids} on open bids)` : ""}`
                      : side === "ask"
                        ? `you hold ${held}`
                        : undefined
                  }
                >
                  <Input type="number" inputMode="numeric" min={1} step={1} aria-label="Quantity" value={qty} onChange={(e) => setQty(Math.max(0, Math.trunc(Number(e.target.value) || 0)))} />
                </Field>
                <Field label="Limit" unit="cr" hint={last != null ? `last ${credits(last)} cr` : undefined}>
                  <Input
                    type="number"
                    inputMode="decimal"
                    min={0.01}
                    step="0.01"
                    aria-label="Limit price"
                    value={limit ?? (last != null ? (last / 100).toFixed(2) : "")}
                    onChange={(e) => setLimit(e.target.value)}
                  />
                </Field>
                <p className="text-muted m-0 text-sm">Expires at {deadlineOfTick(nextCycleEnd, ticksPerCycle)} unless filled or cancelled.</p>
                <div className="bg-surface-2 rounded-sm px-3 py-2.5 text-sm" data-testid="escrow-preview">
                  {side === "bid" ? (
                    <>
                      Holds <span className="tabular-nums">{credits(escrow ?? 0)} cr</span> now; balance after{" "}
                      <span className={`tabular-nums ${after !== null && after < floor ? "text-attn font-bold" : ""}`}>{credits(after ?? 0)} cr</span>
                      {floor > 0 ? (after !== null && after < floor ? `, below your ${credits(floor)} floor` : `, above your ${credits(floor)} floor`) : ""}. Fills settle as
                      they match; unfilled escrow returns on cancel or expiry.
                    </>
                  ) : (
                    <>
                      Holds <span className="tabular-nums">{qty}</span> {good ?? "shares"} out of your {good ? t("pantry").toLowerCase() : "holding"} until filled, cancelled or
                      expired.
                    </>
                  )}
                </div>
                <ButtonRow>
                  <Button type="submit" variant="primary" disabled={place.isPending || qty < 1 || limitCents < 1}>
                    Place {side}
                  </Button>
                  {placed ? <span className="text-muted text-sm">{placed}</span> : null}
                  {error ? (
                    <span className="text-crit text-sm" role="alert">
                      {error}
                    </span>
                  ) : null}
                </ButtonRow>
                {good === "food" ? (
                  <p className="text-muted m-0 text-sm">
                    Your {t("plan").toLowerCase()} keeps Food at least {keepFood} and bids for the shortfall at up to{" "}
                    {plan.max_food_price == null ? "last price × 1.25" : `${credits(Number(plan.max_food_price))} cr`} each hour.
                  </p>
                ) : null}
                {good === "wares" ? (
                  <p className="text-muted m-0 text-sm">
                    {wares
                      ? `Your ${t("plan").toLowerCase()} buys Wares when Comfort is below ${wares.comfort_below} and balance above ${credits(wares.balance_above)} cr${wares.max_price != null ? `, at up to ${credits(wares.max_price)} cr` : ""}.`
                      : `Your ${t("plan").toLowerCase()} has no Wares rule; Comfort holds only while you buy by hand.`}
                  </p>
                ) : null}
              </form>
            </Card>

            <Card title="Your open orders" icon="page" subtitle="Standing orders from the plan and hand-placed orders share this list.">
              {b && b.my_orders.length > 0 ? (
                // Five columns is card-per-row on a phone (§3); the list is short, so at every width.
                <ul className="m-0 grid list-none gap-2 p-0" data-testid="my-orders">
                  {b.my_orders.map((o) => (
                    <li key={o.id}>
                      <Tile className="flex flex-wrap items-center gap-x-4 gap-y-1">
                        <span className="font-bold">
                          {o.side} <Pill>{o.source}</Pill>
                        </span>
                        <span className="tabular-nums">
                          {o.remaining} of {o.qty} left
                        </span>
                        <span className="tabular-nums">at {credits(o.limit_price)} cr</span>
                        <span className="ml-auto">
                          <Button variant="quiet" inline disabled={cancel.isPending} onClick={() => cancel.mutate(o.id, { onError: (e) => setError(e.message) })}>
                            cancel
                          </Button>
                        </span>
                      </Tile>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted m-0">None on this book.</p>
              )}
              <p className="text-muted mt-3 mb-0 text-sm" data-testid="pantry-line">
                {t("pantry")}: {Object.entries(pantry).map(([g, n]) => `${n} ${g}`).join(", ") || "empty"}
                {good && capacity[good] !== undefined ? ` · ${good} cap ${capacity[good]}` : ""}
              </p>
            </Card>
          </Two>

          {good && (directory.data?.recipes ?? []).length > 0 ? (
            <Card title="Where it comes from" icon="org">
              <Provenance
                id={id}
                good={good}
                recipes={directory.data!.recipes}
                orgs={directory.data!.orgs as unknown as Parameters<typeof Provenance>[0]["orgs"]}
                foundingMaterials={good === "materials" ? directory.data!.founding.materials : undefined}
                bare
              />
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
    </div>
  );
}
