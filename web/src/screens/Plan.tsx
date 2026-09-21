// The Standing Plan (GDD 9.3, TDD 6; S1.9) on one screen: how labor is
// chosen, the consumption rules, the saving rule, standing orders, and the
// vote default. Each rule mounts on a capability: no money, no money rules;
// no order books, no standing orders; no governance, no vote default. The
// plan is sent whole (`SetStandingPlan` is a full replace) and the engine
// validates it against the constitution.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useCitizens } from "../api/civic";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { usePlan, useSetPlan } from "../api/society";

type Money = number; // cents on the wire
type LaborPlan = "explicit" | "accept_assignment" | "follow_norm";
type Refresh = "each_tick" | "each_cycle";
type Side = "bid" | "ask";
type Instrument = { good: string } | { share: number };
type StandingOrder = { instrument: Instrument; side: Side; qty: number; limit_price: Money; refresh: Refresh };
type VoteDefault = "abstain" | "none" | { follow: number };
type BuyRule = { comfort_below: number; balance_above: Money; max_price: Money | null };
type Plan = {
  labor: LaborPlan;
  keep_food_at_least: number;
  max_food_price: Money | null;
  buy_wares_when: BuyRule | null;
  keep_balance_at_least: Money;
  standing_orders: StandingOrder[];
  vote_default: VoteDefault;
};

const GOODS = ["grain", "ore", "materials", "food", "wares", "machines"];

/** Credits typed by a person to cents on the wire, and back. */
function toCents(s: string): Money {
  const n = Number(s);
  return Number.isFinite(n) ? Math.round(n * 100) : 0;
}
function fromCents(c: Money): string {
  return (c / 100).toFixed(2);
}

function laborChoices(mode: string): { value: LaborPlan; label: string; hint: string }[] {
  switch (mode) {
    case "assigned":
      return [{ value: "accept_assignment", label: "Accept my assignment", hint: "the planner places you; hours follow the assignment" }];
    case "norm":
      return [
        { value: "explicit", label: "My own hours", hint: "the workplaces and hours set on the Work screen" },
        { value: "follow_norm", label: "Follow the work norm", hint: "work the norm's hours where labor is scarcest" },
      ];
    default:
      return [{ value: "explicit", label: "My own hours", hint: "the workplaces and hours set on the Work screen" }];
  }
}

function MoneyField({
  label,
  value,
  onChange,
  hint,
  optional,
  none = "no limit",
}: {
  label: string;
  value: Money | null;
  onChange: (v: Money | null) => void;
  hint?: string;
  optional?: boolean;
  none?: string;
}) {
  return (
    <label className="flex flex-col gap-1 text-sm">
      <span>{label}</span>
      <span className="flex items-center gap-2">
        {optional ? (
          <input
            type="checkbox"
            aria-label={`${label}: set a limit`}
            checked={value !== null}
            onChange={(e) => onChange(e.target.checked ? 0 : null)}
          />
        ) : null}
        {value === null ? (
          <span className="text-muted">{none}</span>
        ) : (
          <input
            type="number"
            inputMode="decimal"
            min={0}
            step="0.01"
            aria-label={label}
            className="border-line num w-28 rounded-sm border px-1"
            value={fromCents(value)}
            onChange={(e) => onChange(toCents(e.target.value))}
          />
        )}
        <span className="text-muted text-xs">cr</span>
      </span>
      {hint ? <span className="text-muted text-xs">{hint}</span> : null}
    </label>
  );
}

export function PlanScreen({ id }: { id: number }) {
  const plan = usePlan(id);
  const caps = useCapabilities(id);
  const { t } = useLexicon(id);
  const setPlan = useSetPlan(id);
  // The citizens, so a followed ballot is picked by name (S2.6); only where there is an assembly.
  const citizens = useCitizens(id);
  const home = useHome(id);
  const [draft, setDraft] = useState<Plan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const loaded = plan.data?.plan as unknown as Plan | undefined;
  // Until the first edit the form mirrors the engine's plan; edits then
  // survive ticks, and a save clears them so what the engine holds shows.

  if (plan.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
  if (plan.error || caps.error) return <p className="text-bad">Could not load: {String(plan.error ?? caps.error)}</p>;
  const c = caps.data!;
  const d = draft ?? structuredClone(loaded!);
  const set = (patch: Partial<Plan>) => setDraft({ ...d, ...patch });
  const choices = laborChoices(c.labor);
  const follow = typeof d.vote_default === "object" ? d.vote_default.follow : null;

  const save = () => {
    setError(null);
    setPlan.mutate(d as unknown as Record<string, unknown>, {
      onSuccess: () => {
        setSaved(true);
        setDraft(null);
      },
      onError: (e) => setError(e.message),
    });
  };

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{t("plan")}</h2>
        <p className="text-muted text-sm">Runs every hour, here or not. Sent whole when you save.</p>
      </header>

      <section className="flex flex-col gap-3" data-testid="plan-labor">
        <h3 className="text-lg">Labor</h3>
        {choices.map((ch) => (
          <label key={ch.value} className="flex items-baseline gap-2 text-sm">
            <input
              type="radio"
              name="labor"
              value={ch.value}
              checked={d.labor === ch.value}
              onChange={() => set({ labor: ch.value })}
            />
            <span>
              {ch.label} <span className="text-muted text-xs">{ch.hint}</span>
            </span>
          </label>
        ))}
        <p className="text-muted text-xs">
          Workplaces, hours and effort live on the{" "}
          <Link to="/s/$id/work" params={{ id: String(id) }} className="underline">
            {t("work_screen")}
          </Link>{" "}
          screen.
        </p>
      </section>

      <section className="grid gap-6 md:grid-cols-2" data-testid="plan-consumption">
        <div className="flex flex-col gap-4">
          <h3 className="text-lg">Eating</h3>
          <label className="flex flex-col gap-1 text-sm">
            <span>Keep {t("pantry").toLowerCase()} Food at least</span>
            <span className="flex items-center gap-2">
              <input
                type="number"
                inputMode="numeric"
                min={0}
                step={1}
                aria-label="Keep Food at least"
                className="border-line num w-20 rounded-sm border px-1"
                value={d.keep_food_at_least}
                onChange={(e) => set({ keep_food_at_least: Math.max(0, Math.trunc(Number(e.target.value) || 0)) })}
              />
              <span className="text-muted text-xs">units; the plan {c.money ? "bids for" : "draws"} the shortfall each hour</span>
            </span>
          </label>
          {c.money ? (
            <MoneyField
              label="Food price ceiling"
              optional
              none="last price x 1.25"
              value={d.max_food_price}
              onChange={(v) => set({ max_food_price: v })}
              hint="the most a Food bid will pay per unit"
            />
          ) : null}
        </div>
        <div className="flex flex-col gap-4">
          <h3 className="text-lg">Wares</h3>
          <label className="flex items-baseline gap-2 text-sm">
            <input
              type="checkbox"
              aria-label="Buy Wares by rule"
              checked={d.buy_wares_when !== null}
              onChange={(e) =>
                set({ buy_wares_when: e.target.checked ? { comfort_below: 50, balance_above: 0, max_price: null } : null })
              }
            />
            <span>{c.money ? "Buy" : "Draw"} Wares when Comfort is low</span>
          </label>
          {d.buy_wares_when ? (
            <div className="ml-6 flex flex-col gap-3">
              <label className="flex flex-col gap-1 text-sm">
                <span>Comfort below</span>
                <input
                  type="number"
                  inputMode="numeric"
                  min={0}
                  max={100}
                  step={1}
                  aria-label="Comfort below"
                  className="border-line num w-20 rounded-sm border px-1"
                  value={d.buy_wares_when.comfort_below}
                  onChange={(e) =>
                    set({
                      buy_wares_when: {
                        ...d.buy_wares_when!,
                        comfort_below: Math.min(100, Math.max(0, Math.trunc(Number(e.target.value) || 0))),
                      },
                    })
                  }
                />
              </label>
              {c.money ? (
                <>
                  <MoneyField
                    label="and balance above"
                    value={d.buy_wares_when.balance_above}
                    onChange={(v) => set({ buy_wares_when: { ...d.buy_wares_when!, balance_above: v ?? 0 } })}
                  />
                  <MoneyField
                    label="Wares price ceiling"
                    optional
                    none="last price x 1.25"
                    value={d.buy_wares_when.max_price}
                    onChange={(v) => set({ buy_wares_when: { ...d.buy_wares_when!, max_price: v } })}
                  />
                </>
              ) : null}
            </div>
          ) : null}
        </div>
      </section>

      {c.money ? (
        <section className="flex flex-col gap-3" data-testid="plan-saving">
          <h3 className="text-lg">Saving</h3>
          <MoneyField
            label="Keep balance at least"
            value={d.keep_balance_at_least}
            onChange={(v) => set({ keep_balance_at_least: v ?? 0 })}
            hint="a hard floor: the plan's bids never commit money below it"
          />
        </section>
      ) : null}

      {c.order_books ? (
        <section className="flex flex-col gap-3" data-testid="plan-orders">
          <h3 className="text-lg">Standing orders</h3>
          {d.standing_orders.length === 0 ? (
            <p className="text-muted text-sm">None. A standing order is placed again every hour or every day at your limit.</p>
          ) : (
            <table className="w-full text-sm">
              <thead className="text-muted text-left text-xs uppercase tracking-wide">
                <tr>
                  <th className="py-1 font-normal">Instrument</th>
                  <th className="py-1 font-normal">Side</th>
                  <th className="py-1 font-normal">Qty</th>
                  <th className="py-1 font-normal">Limit (cr)</th>
                  <th className="py-1 font-normal">Refresh</th>
                  <th className="py-1 font-normal" />
                </tr>
              </thead>
              <tbody>
                {d.standing_orders.map((o, i) => {
                  const patch = (p: Partial<StandingOrder>) =>
                    set({ standing_orders: d.standing_orders.map((x, j) => (j === i ? { ...x, ...p } : x)) });
                  const isShare = "share" in o.instrument;
                  return (
                    <tr key={i} className="rule align-top">
                      <td className="py-2 pr-2">
                        <select
                          aria-label={`Order ${i + 1} instrument kind`}
                          className="border-line rounded-sm border px-1"
                          value={isShare ? "share" : "good"}
                          onChange={(e) =>
                            patch({ instrument: e.target.value === "share" ? { share: 1 } : { good: "food" } })
                          }
                        >
                          <option value="good">good</option>
                          <option value="share">share</option>
                        </select>{" "}
                        {isShare ? (
                          <input
                            type="number"
                            min={1}
                            aria-label={`Order ${i + 1} org`}
                            className="border-line num w-16 rounded-sm border px-1"
                            value={(o.instrument as { share: number }).share}
                            onChange={(e) => patch({ instrument: { share: Math.max(1, Math.trunc(Number(e.target.value) || 1)) } })}
                          />
                        ) : (
                          <select
                            aria-label={`Order ${i + 1} good`}
                            className="border-line rounded-sm border px-1"
                            value={(o.instrument as { good: string }).good}
                            onChange={(e) => patch({ instrument: { good: e.target.value } })}
                          >
                            {GOODS.map((g) => (
                              <option key={g} value={g}>
                                {g}
                              </option>
                            ))}
                          </select>
                        )}
                      </td>
                      <td className="py-2 pr-2">
                        <select
                          aria-label={`Order ${i + 1} side`}
                          className="border-line rounded-sm border px-1"
                          value={o.side}
                          onChange={(e) => patch({ side: e.target.value as Side })}
                        >
                          <option value="bid">bid</option>
                          <option value="ask">ask</option>
                        </select>
                      </td>
                      <td className="py-2 pr-2">
                        <input
                          type="number"
                          min={1}
                          step={1}
                          aria-label={`Order ${i + 1} quantity`}
                          className="border-line num w-16 rounded-sm border px-1"
                          value={o.qty}
                          onChange={(e) => patch({ qty: Math.max(0, Math.trunc(Number(e.target.value) || 0)) })}
                        />
                      </td>
                      <td className="py-2 pr-2">
                        <input
                          type="number"
                          min={0}
                          step="0.01"
                          aria-label={`Order ${i + 1} limit`}
                          className="border-line num w-24 rounded-sm border px-1"
                          value={fromCents(o.limit_price)}
                          onChange={(e) => patch({ limit_price: toCents(e.target.value) })}
                        />
                      </td>
                      <td className="py-2 pr-2">
                        <select
                          aria-label={`Order ${i + 1} refresh`}
                          className="border-line rounded-sm border px-1"
                          value={o.refresh}
                          onChange={(e) => patch({ refresh: e.target.value as Refresh })}
                        >
                          <option value="each_tick">each hour</option>
                          <option value="each_cycle">each day</option>
                        </select>
                      </td>
                      <td className="py-2">
                        <button
                          type="button"
                          className="text-muted text-xs underline"
                          onClick={() => set({ standing_orders: d.standing_orders.filter((_, j) => j !== i) })}
                        >
                          remove
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
          <button
            type="button"
            className="border-line self-start rounded-sm border px-2 py-0.5 text-sm"
            onClick={() =>
              set({
                standing_orders: [
                  ...d.standing_orders,
                  { instrument: { good: "food" }, side: "bid", qty: 1, limit_price: 100, refresh: "each_cycle" },
                ],
              })
            }
          >
            Add a standing order
          </button>
        </section>
      ) : null}

      {c.governance !== "none" ? (
        <section className="flex flex-col gap-3" data-testid="plan-vote">
          <h3 className="text-lg">Votes</h3>
          <p className="text-muted text-xs">What your ballot does when you are not here to cast it.</p>
          {(
            [
              ["none", "No default", "an uncast ballot stays uncast"],
              ["abstain", "Abstain", "counts toward quorum, neither for nor against"],
              ["follow", "Follow a citizen", "votes as they vote"],
            ] as const
          ).map(([v, label, hint]) => (
            <label key={v} className="flex items-baseline gap-2 text-sm">
              <input
                type="radio"
                name="vote"
                value={v}
                checked={v === "follow" ? follow !== null : d.vote_default === v}
                onChange={() =>
                set({
                  vote_default:
                    v === "follow"
                      ? { follow: follow ?? (citizens.data?.citizens ?? []).find((z) => z.id !== home.data?.citizen.id)?.id ?? 1 }
                      : v,
                })
              }
              />
              <span>
                {label} <span className="text-muted text-xs">{hint}</span>
              </span>
            </label>
          ))}
          {follow !== null ? (
            <label className="ml-6 flex items-center gap-2 text-sm">
              <span>Whose ballot</span>
              <select
                aria-label="Citizen to follow"
                className="border-line w-48 rounded-sm border px-1"
                value={follow}
                onChange={(e) => set({ vote_default: { follow: Number(e.target.value) } })}
              >
                {(citizens.data?.citizens ?? [])
                  .filter((z) => z.id !== home.data?.citizen.id)
                  .map((z) => (
                    <option key={z.id} value={z.id}>
                      {z.handle}
                    </option>
                  ))}
                {(citizens.data?.citizens ?? []).some((z) => z.id === follow) ? null : (
                  <option value={follow}>citizen no. {follow}</option>
                )}
              </select>
              <span className="text-muted text-xs">one hop: if they cast no ballot of their own, you abstain</span>
            </label>
          ) : null}
        </section>
      ) : null}

      <div className="flex flex-wrap items-baseline gap-3 text-sm">
        <button
          type="button"
          disabled={setPlan.isPending}
          className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50"
          onClick={save}
        >
          Save my plan
        </button>
        <button
          type="button"
          disabled={draft === null}
          className="text-muted underline disabled:opacity-50"
          onClick={() => setDraft(null)}
        >
          Discard changes
        </button>
        {saved && draft === null ? <span className="text-muted">Saved. It acts from the next hour.</span> : null}
        {error ? (
          <span className="text-bad" role="alert">
            {error}
          </span>
        ) : null}
      </div>
    </div>
  );
}
