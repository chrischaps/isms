// The Common Store (GDD 6.2, 15; S2.7): the shelves as they stand this hour,
// what each unit does for a meter, what you may draw and what your plan has
// asked for, the rule that decides when the shelf runs short, how the Store
// served everyone yesterday and today, and your own draw record with every
// Explain. No prices anywhere: the absence is the design statement. Mounted
// where `caps.common_store` is on.

import { Link } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useStore, type StockView, type StoreDayView } from "../api/commons";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { Ledger } from "../components/Ledger";
import { drawRows, ruleText } from "../lib/draws";
import { dayOf } from "../lib/when";

const GOOD_ORDER = ["food", "wares", "grain", "ore", "materials", "machines"];

function byOrder(a: { good: string }, b: { good: string }): number {
  const ia = GOOD_ORDER.indexOf(a.good);
  const ib = GOOD_ORDER.indexOf(b.good);
  return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
}

function DayTable({ rows, label, testId }: { rows: StoreDayView[]; label: string; testId: string }) {
  const sorted = rows.slice().sort(byOrder);
  return (
    <div data-testid={testId}>
      <h4 className="text-muted text-xs uppercase tracking-wide">{label}</h4>
      {sorted.length === 0 ? (
        <p className="text-muted mt-1 text-sm">Nobody has asked the Store for anything yet.</p>
      ) : (
        <table className="mt-1 w-full text-sm">
          <thead className="text-muted text-left text-xs uppercase tracking-wide">
            <tr>
              <th className="py-1 font-normal">Good</th>
              <th className="py-1 text-right font-normal">Asked</th>
              <th className="py-1 text-right font-normal">Served</th>
              <th className="py-1 text-right font-normal">Short</th>
              <th className="py-1 text-right font-normal">Hours rationed</th>
              <th className="py-1 text-right font-normal">Shared out</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((r) => (
              <tr key={r.good} className="rule">
                <td className="py-1 pr-2">{r.good}</td>
                <td className="num py-1 text-right">{r.requested}</td>
                <td className="num py-1 text-right">{r.served}</td>
                <td className={`num py-1 text-right ${r.short > 0 ? "text-bad" : "text-muted"}`}>{r.short}</td>
                <td className={`num py-1 text-right ${r.rationed_ticks > 0 ? "text-warn" : "text-muted"}`}>{r.rationed_ticks}</td>
                <td className="num py-1 text-right">
                  {r.shared > 0 ? (
                    <>
                      {r.shared} <span className="text-muted">to {r.shared_with}</span>
                    </>
                  ) : (
                    <span className="text-muted">—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function Shelf({ s, pantry, capacity }: { s: StockView; pantry: Record<string, number>; capacity: Record<string, number> }) {
  const held = pantry[s.good] ?? 0;
  const cap = capacity[s.good];
  const byNeed = s.meter_per_unit != null;
  return (
    <tr className={`rule ${byNeed ? "" : "text-muted"}`} data-testid={`shelf-${s.good}`}>
      <td className="py-2 pr-3 align-top">
        {s.good}
        {byNeed ? <span className="text-muted block text-xs">one unit restores {s.meter_per_unit} points</span> : <span className="text-muted block text-xs">to the workplaces, not to households</span>}
      </td>
      <td className={`num py-2 pr-3 text-right align-top ${s.stock === 0 && byNeed ? "text-bad" : ""}`} data-testid="stock">
        {s.stock}
      </td>
      <td className="num py-2 pr-3 text-right align-top">
        {s.requested > 0 ? (
          <>
            {s.requested} <span className="text-muted text-xs">by {s.requesters}</span>
          </>
        ) : (
          <span className="text-muted">—</span>
        )}
      </td>
      <td className="num py-2 pr-3 text-right align-top" data-testid="entitlement">
        {byNeed ? (
          <>
            {s.my_entitlement}
            {s.my_pending > 0 ? <span className="text-muted text-xs"> · {s.my_pending} asked</span> : null}
          </>
        ) : (
          <span className="text-muted">—</span>
        )}
      </td>
      <td className="num py-2 pr-3 text-right align-top">
        {byNeed ? (
          <>
            {held}
            {cap !== undefined ? <span className="text-muted text-xs"> of {cap}</span> : null}
          </>
        ) : (
          <span className="text-muted">—</span>
        )}
      </td>
      <td className="num py-2 text-right align-top">{byNeed ? s.share_if_shared_now : <span className="text-muted">—</span>}</td>
    </tr>
  );
}

export function Store({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const store = useStore(id, caps.data?.common_store === true);

  if (caps.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error) return <p className="text-bad">Could not load: {String(caps.error)}</p>;
  // No Store, no screen: said before anyone is asked to join.
  if (!caps.data!.common_store) {
    return <p className="text-muted">There is no Common Store in this society.</p>;
  }
  if (home.isPending) return <p className="text-muted">Loading.</p>;
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
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  if (store.isPending) return <p className="text-muted">Loading.</p>;
  if (store.error) return <p className="text-bad">Could not load: {String(store.error)}</p>;
  const v = store.data!;
  const h = home.data!;
  const perDay = h.clock.ticks_per_cycle;
  const pantry = v.my_pantry as Record<string, number>;
  const capacity = (v.pantry_capacity ?? {}) as Record<string, number>;
  const shelves = v.stock.slice().sort(byOrder);
  const food = shelves.find((s) => s.good === "food");
  const wares = shelves.find((s) => s.good === "wares");
  const bare = shelves.filter((s) => s.meter_per_unit != null && s.stock === 0).map((s) => s.good);

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{t("store_screen")}</h2>
        <p className="text-muted text-sm" data-testid="store-rule">
          When a shelf runs short, <span className="text-ink">{ruleText(v.rule)}</span>.
        </p>
      </header>

      {bare.length > 0 ? (
        <p className="text-bad text-sm" data-testid="bare-shelf">
          The shelf is bare of {bare.join(" and ")}. Whatever the workplaces make this hour is served this hour, by the rule.
        </p>
      ) : null}

      <section>
        <h3 className="text-lg">The shelves</h3>
        <table className="mt-2 w-full text-sm" data-testid="shelves">
          <thead className="text-muted text-left text-xs uppercase tracking-wide">
            <tr>
              <th className="py-1 font-normal">Good</th>
              <th className="py-1 text-right font-normal">On the shelf</th>
              <th className="py-1 text-right font-normal">Asked this hour</th>
              <th className="py-1 text-right font-normal">You may draw</th>
              <th className="py-1 text-right font-normal">{t("pantry")}</th>
              <th className="py-1 text-right font-normal">Your share tonight</th>
            </tr>
          </thead>
          <tbody>
            {shelves.map((s) => (
              <Shelf key={s.good} s={s} pantry={pantry} capacity={capacity} />
            ))}
          </tbody>
        </table>
        <p className="text-muted mt-3 max-w-prose text-xs">
          Your {t("plan").toLowerCase()} asks the Store each hour for what would bring your meters to full, no more, and the Store serves it that hour
          while the shelf lasts. At the end of the day whatever is left beyond everyone&apos;s needs is shared out equally among the{" "}
          {v.active_citizens} citizens: the last column is your share if the day ended now.
          {food && food.my_entitlement > 0 && food.my_pending === 0 ? ` You could draw ${food.my_entitlement} Food this hour; your plan will ask for it at the next.` : ""}
          {wares && wares.my_entitlement > 0 && wares.my_pending === 0 ? ` Comfort has room for ${wares.my_entitlement} Wares.` : ""}
        </p>
      </section>

      <section className="grid gap-8 md:grid-cols-2">
        <DayTable rows={v.today} label={`Today, ${dayOf(h.clock.cycle - 1)}`} testId="store-today" />
        <DayTable
          rows={v.yesterday}
          label={v.last_cycle != null ? `Yesterday, ${dayOf(v.last_cycle)}` : "Yesterday"}
          testId="store-yesterday"
        />
      </section>

      <section>
        <h3 className="text-lg">{t("compensation")}</h3>
        <p className="text-muted mt-1 text-xs">Every draw, with the rule that served it. This is the whole of what you receive here: there is no wage.</p>
        <div className="mt-2" data-testid="draw-record">
          <Ledger rows={drawRows(v.my_draws as unknown as Parameters<typeof drawRows>[0], perDay)} empty="No draw yet. Your plan asks for Food when the meter has room for a unit." />
        </div>
      </section>
    </div>
  );
}
