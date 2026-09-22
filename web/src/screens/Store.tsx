// The Common Store (GDD 6.2, 15; S2.7; docs/style.md §10 by analogy): the
// shelves as they stand this hour, what each unit does for a meter, what you
// may draw and what your plan has asked for, the rule that decides when the
// shelf runs short, how the Store served everyone yesterday and today, and
// your own draw record with every Explain. The Verdict says whether the
// shelves are stocked and what is yours to draw. No prices anywhere: the
// absence is the design statement. Mounted where `caps.common_store` is on.

import { Link } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useStore, type StockView, type StoreDayView } from "../api/commons";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { Card, Stack, Two } from "../components/Card";
import { Figure, Figures } from "../components/Figure";
import { Ledger, TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { Meter } from "../components/Meter";
import { PageHeader } from "../components/PageHeader";
import { Verdict } from "../components/Verdict";
import { drawRows, ruleText, stockLevel } from "../lib/draws";
import { storeVerdict } from "../lib/verdict";
import { dayOf } from "../lib/when";

const GOOD_ORDER = ["food", "wares", "grain", "ore", "materials", "machines"];

function byOrder(a: { good: string }, b: { good: string }): number {
  const ia = GOOD_ORDER.indexOf(a.good);
  const ib = GOOD_ORDER.indexOf(b.good);
  return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
}

/** How the Store served a day: four columns on a phone, the rationing and the share-out folded under the good. */
function DayTable({ rows, testId }: { rows: StoreDayView[]; testId: string }) {
  const sorted = rows.slice().sort(byOrder);
  if (sorted.length === 0) return <p className="text-muted m-0">Nobody has asked the Store for anything yet.</p>;
  return (
    <table className="w-full border-collapse text-[15px]" data-testid={testId}>
      <thead>
        <tr>
          <th className={TH}>Good</th>
          <th className={TH_NUM}>Asked</th>
          <th className={TH_NUM}>Served</th>
          <th className={TH_NUM}>Short</th>
          <th className={`${TH_NUM} hidden md:table-cell`}>Hours rationed</th>
          <th className={`${TH_NUM} hidden md:table-cell`}>Shared out</th>
        </tr>
      </thead>
      <tbody>
        {sorted.map((r) => {
          const shared = r.shared > 0 ? `${r.shared} to ${r.shared_with}` : null;
          return (
            <tr key={r.good} className="hover:bg-surface-2">
              <td className={TD}>
                {r.good}
                <span className="text-muted block text-sm md:hidden">
                  {r.rationed_ticks > 0 ? <span className="text-attn">{r.rationed_ticks} hours rationed</span> : "never rationed"}
                  {shared ? ` · ${shared} shared out` : ""}
                </span>
              </td>
              <td className={TD_NUM}>{r.requested}</td>
              <td className={TD_NUM}>{r.served}</td>
              <td className={`${TD_NUM} ${r.short > 0 ? "text-crit font-bold" : "text-muted"}`}>{r.short}</td>
              <td className={`${TD_NUM} hidden md:table-cell ${r.rationed_ticks > 0 ? "text-attn font-bold" : "text-muted"}`}>{r.rationed_ticks}</td>
              <td className={`${TD_NUM} hidden md:table-cell`}>
                {shared ? (
                  <>
                    {r.shared} <span className="text-muted">to {r.shared_with}</span>
                  </>
                ) : (
                  <span className="text-muted">—</span>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

/** One shelf: the good, what is on it, what you may draw; the rest folded under the good on a phone. */
function Shelf({ s, pantry, capacity, pantryWord }: { s: StockView; pantry: Record<string, number>; capacity: Record<string, number>; pantryWord: string }) {
  const held = pantry[s.good] ?? 0;
  const cap = capacity[s.good];
  const byNeed = s.meter_per_unit != null;
  const asked = s.requested > 0 ? `${s.requested} by ${s.requesters}` : null;
  return (
    <tr className={`hover:bg-surface-2 ${byNeed ? "" : "text-muted"}`} data-testid={`shelf-${s.good}`}>
      <td className={TD}>
        {s.good}
        <span className="text-muted block text-sm">{byNeed ? `one unit restores ${s.meter_per_unit} points` : "to the workplaces, not to households"}</span>
        {byNeed ? (
          <span className="text-muted block text-sm md:hidden">
            {asked ? `asked ${asked} · ` : ""}
            {pantryWord.toLowerCase()} {held}
            {cap !== undefined ? ` of ${cap}` : ""} · share tonight {s.share_if_shared_now}
          </span>
        ) : asked ? (
          <span className="text-muted block text-sm md:hidden">asked {asked}</span>
        ) : null}
      </td>
      <td className={`${TD_NUM} ${s.stock === 0 && byNeed ? "text-crit font-bold" : ""}`} data-testid="stock">
        {s.stock}
      </td>
      <td className={`${TD_NUM} hidden md:table-cell`}>{asked ?? <span className="text-muted">—</span>}</td>
      <td className={TD_NUM} data-testid="entitlement">
        {byNeed ? (
          <>
            {s.my_entitlement}
            {s.my_pending > 0 ? <span className="text-muted block text-sm">{s.my_pending} asked</span> : null}
          </>
        ) : (
          <span className="text-muted">—</span>
        )}
      </td>
      <td className={`${TD_NUM} hidden md:table-cell`}>
        {byNeed ? (
          <>
            {held}
            {cap !== undefined ? <span className="text-muted"> of {cap}</span> : null}
          </>
        ) : (
          <span className="text-muted">—</span>
        )}
      </td>
      <td className={`${TD_NUM} hidden md:table-cell`}>{byNeed ? s.share_if_shared_now : <span className="text-muted">—</span>}</td>
    </tr>
  );
}

export function Store({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const store = useStore(id, caps.data?.common_store === true);

  if (caps.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error) return <p className="text-crit">Could not load: {String(caps.error)}</p>;
  // No Store, no screen: said before anyone is asked to join.
  if (!caps.data!.common_store) {
    return <p className="text-muted">There is no Common Store in this society.</p>;
  }
  if (home.isPending) return <p className="text-muted">Loading.</p>;
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
  if (home.error) return <p className="text-crit">Could not load: {String(home.error)}</p>;
  if (store.isPending) return <p className="text-muted">Loading.</p>;
  if (store.error) return <p className="text-crit">Could not load: {String(store.error)}</p>;
  const v = store.data!;
  const h = home.data!;
  const perDay = h.clock.ticks_per_cycle;
  const pantry = v.my_pantry as Record<string, number>;
  const capacity = (v.pantry_capacity ?? {}) as Record<string, number>;
  const shelves = v.stock.slice().sort(byOrder);
  const food = shelves.find((s) => s.good === "food");
  const wares = shelves.find((s) => s.good === "wares");
  const bare = shelves.filter((s) => s.meter_per_unit != null && s.stock === 0).map((s) => s.good);
  const verdict = storeVerdict({
    bare,
    food: food ? { stock: food.stock, entitlement: food.my_entitlement, pending: food.my_pending } : null,
    rule: v.rule,
  });
  const byNeed = shelves.filter((s) => s.meter_per_unit != null);

  return (
    <div>
      <PageHeader
        title={t("store_screen")}
        meta={
          <span data-testid="store-rule">
            When a shelf runs short, <b>{ruleText(v.rule)}</b>.
          </span>
        }
      />
      <Stack>
        <Card title="The shelves" icon="store">
          <Verdict parts={verdict} />
          {bare.length > 0 ? (
            <p className="text-crit mt-0 mb-3 text-sm" data-testid="bare-shelf">
              The shelf is bare of {bare.join(" and ")}. Whatever the workplaces make this hour is served this hour, by the rule.
            </p>
          ) : null}
          <Figures className="mb-4">
            {byNeed.map((s) => {
              // The shelf as a bar against this hour's requests (S2.11): the same
              // Meter a need has, its fill the engine's stock and its tone the
              // shelf's state, so a Commune's glance reads like Freeport's.
              const level = stockLevel(s.stock, s.requested);
              return (
                <Figure
                  key={s.good}
                  testId={`shelf-figure-${s.good}`}
                  label={`${s.good} on the shelf`}
                  value={String(s.stock)}
                  bar={<Meter label={`${s.good} against what is asked`} value={level.value} threshold={0} tone={level.tone} bare />}
                  tone={level.tone}
                  status={s.stock === 0 ? "Bare this hour." : s.requested > s.stock ? `${s.requested} asked, more than is here.` : s.requested > 0 ? `${s.requested} asked this hour.` : "Nobody has asked this hour."}
                />
              );
            })}
          </Figures>
          <table className="w-full border-collapse text-[15px]" data-testid="shelves">
            <thead>
              <tr>
                <th className={TH}>Good</th>
                <th className={TH_NUM}>On the shelf</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Asked this hour</th>
                <th className={TH_NUM}>You may draw</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>{t("pantry")}</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Your share tonight</th>
              </tr>
            </thead>
            <tbody>
              {shelves.map((s) => (
                <Shelf key={s.good} s={s} pantry={pantry} capacity={capacity} pantryWord={t("pantry")} />
              ))}
            </tbody>
          </table>
          <p className="text-muted mt-3 mb-0 text-sm">
            Your {t("plan").toLowerCase()} asks the Store each hour for what would bring your meters to full, no more, and the Store serves it that hour while the
            shelf lasts. At the end of the day whatever is left beyond everyone's needs is shared out equally among the {v.active_citizens} citizens: "your share
            tonight" is what you would get if the day ended now.
            {food && food.my_entitlement > 0 && food.my_pending === 0 ? ` You could draw ${food.my_entitlement} Food this hour; your plan will ask for it at the next.` : ""}
            {wares && wares.my_entitlement > 0 && wares.my_pending === 0 ? ` Comfort has room for ${wares.my_entitlement} Wares.` : ""}
          </p>
        </Card>

        <Two>
          <Card title="Today" icon="clock" subtitle={`${dayOf(h.clock.cycle - 1)}, so far. What was asked, what was served, and where the rule had to choose.`}>
            <DayTable rows={v.today} testId="store-today" />
          </Card>
          <Card title="Yesterday" icon="archive" subtitle={v.last_cycle != null ? `${dayOf(v.last_cycle)}, the whole day.` : "No day has closed yet."}>
            <DayTable rows={v.yesterday} testId="store-yesterday" />
          </Card>
        </Two>

        <Card title={t("compensation")} icon="ledger" subtitle="Every draw, with the rule that served it. This is the whole of what you receive here: there is no wage.">
          <div data-testid="draw-record">
            <Ledger rows={drawRows(v.my_draws as unknown as Parameters<typeof drawRows>[0], perDay)} amountLabel="Drew" empty="No draw yet. Your plan asks for Food when the meter has room for a unit." />
          </div>
        </Card>
      </Stack>
    </div>
  );
}
