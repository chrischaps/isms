// The Standing Plan (GDD 9.3, TDD 6; S1.9; docs/style.md §10 Standing plan)
// on one screen: the Verdict and the plan as a checklist first, then one card
// per rule with its editable fields — how labor is chosen, the consumption
// rules, the saving rule, standing orders, and the vote default. Each rule
// mounts on a capability: no money, no money rules; no order books, no
// standing orders; no governance, no vote default. The plan is sent whole
// (`SetStandingPlan` is a full replace) and the engine validates it against
// the constitution. The wire carries no log of what the plan did (Q152); the
// detail shows the digest's trades and draws since you last looked.

import { useState, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { useCitizens } from "../api/civic";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { usePlan, useSetPlan } from "../api/society";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile } from "../components/Card";
import { DiffSinceLastSeen } from "../components/DiffSinceLastSeen";
import { Field, Input, Select } from "../components/Field";
import { RuleList } from "../components/Feed";
import { More } from "../components/More";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { planVerdict } from "../lib/verdict";
import { whenOfTick } from "../lib/when";

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

/** A money field: a limit in credits, or, when optional, a checkbox that turns the limit on. */
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
  hint?: ReactNode;
  optional?: boolean;
  none?: string;
}) {
  if (optional && value === null) {
    return (
      <label className="flex items-center gap-2.5 text-sm">
        <input type="checkbox" aria-label={`${label}: set a limit`} checked={false} onChange={() => onChange(0)} />
        <span>
          <b>{label}</b> <span className="text-muted">— {none}</span>
        </span>
      </label>
    );
  }
  return (
    <div className="grid gap-1.5">
      {optional ? (
        <label className="flex items-center gap-2.5 text-sm">
          <input type="checkbox" aria-label={`${label}: set a limit`} checked onChange={() => onChange(null)} />
          <b>{label}</b>
        </label>
      ) : null}
      <Field label={optional ? <span className="sr-only">{label}</span> : label} unit="cr" hint={hint}>
        <Input type="number" inputMode="decimal" min={0} step="0.01" aria-label={label} value={fromCents(value ?? 0)} onChange={(e) => onChange(toCents(e.target.value))} />
      </Field>
    </div>
  );
}

/** A radio list with a bold label and a muted hint per choice. */
function Radios<V extends string>({ name, value, choices, onChange }: { name: string; value: V; choices: { value: V; label: string; hint: string }[]; onChange: (v: V) => void }) {
  return (
    <div className="grid gap-2">
      {choices.map((ch) => (
        <label key={ch.value} className="flex min-h-touch items-center gap-2.5">
          <input type="radio" name={name} value={ch.value} checked={value === ch.value} onChange={() => onChange(ch.value)} />
          <span>
            <b>{ch.label}</b> <span className="text-muted text-sm">{ch.hint}</span>
          </span>
        </label>
      ))}
    </div>
  );
}

const orderText = (o: StandingOrder) =>
  `${o.side === "bid" ? "Bid for" : "Offer"} ${o.qty} ${"good" in o.instrument ? o.instrument.good : `shares of org no. ${o.instrument.share}`} at ${fromCents(o.limit_price)} cr, ${o.refresh === "each_tick" ? "every hour" : "every day"}`;

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
  if (plan.error || caps.error) return <p className="text-crit">Could not load: {String(plan.error ?? caps.error)}</p>;
  const c = caps.data!;
  const h = home.data;
  const d = draft ?? structuredClone(loaded!);
  const set = (patch: Partial<Plan>) => setDraft({ ...d, ...patch });
  const choices = laborChoices(c.labor);
  const follow = typeof d.vote_default === "object" ? d.vote_default.follow : null;
  const handleOf = (cid: number) => (citizens.data?.citizens ?? []).find((z) => z.id === cid)?.handle ?? `citizen no. ${cid}`;
  const pantry = t("pantry").toLowerCase();
  const verb = c.common_store ? "draw" : "buy";

  // The Verdict and the checklist read the draft, so an edit reads back before it is saved.
  const verdict = planVerdict({
    keepFood: d.keep_food_at_least,
    keepBalance: d.keep_balance_at_least,
    wares: d.buy_wares_when !== null,
    orders: d.standing_orders.length,
    money: c.money,
    store: c.common_store,
    labor: d.labor,
    vote: c.governance === "none" ? null : follow !== null ? "follow" : d.vote_default === "abstain" ? "abstain" : "none",
    followHandle: follow !== null ? handleOf(follow) : undefined,
  });
  const rules: { key: string; node: ReactNode; blocked?: ReactNode }[] = [
    {
      key: "labor",
      node: choices.find((ch) => ch.value === d.labor) ? (
        <>
          Labor: <b>{choices.find((ch) => ch.value === d.labor)!.label.toLowerCase()}</b>.
        </>
      ) : (
        <>
          Labor: <b>{d.labor}</b>.
        </>
      ),
    },
    {
      key: "food",
      node: (
        <>
          Keep <b>Food</b> at least <b>{d.keep_food_at_least}</b> in the {pantry}
          {c.money ? (
            <>
              , bidding up to <b>{d.max_food_price === null ? "last price × 1.25" : `${fromCents(d.max_food_price)} cr`}</b> each.
            </>
          ) : (
            "."
          )}
        </>
      ),
      blocked: d.keep_food_at_least === 0 ? `the plan never ${verb}s Food` : undefined,
    },
    {
      key: "wares",
      node: d.buy_wares_when ? (
        <>
          {c.money ? "Buy" : "Draw"} <b>Wares</b> when Comfort is below <b>{d.buy_wares_when.comfort_below}</b>
          {c.money ? (
            <>
              {" "}
              and balance above <b>{fromCents(d.buy_wares_when.balance_above)} cr</b>
              {d.buy_wares_when.max_price !== null ? (
                <>
                  , at up to <b>{fromCents(d.buy_wares_when.max_price)} cr</b>
                </>
              ) : null}
            </>
          ) : null}
          .
        </>
      ) : (
        <>
          No <b>Wares</b> rule: Comfort holds only while you {verb} by hand.
        </>
      ),
    },
    ...(c.money
      ? [
          {
            key: "balance",
            node: (
              <>
                Keep <b>{fromCents(d.keep_balance_at_least)} cr</b> in hand{d.keep_balance_at_least === 0 ? " (spend freely)." : "."}
              </>
            ),
          },
        ]
      : []),
    ...(c.order_books
      ? d.standing_orders.map((o, i) => ({
          key: `order-${i}`,
          node: <>{orderText(o)}.</>,
          blocked: o.qty === 0 ? "a quantity of 0 places nothing" : o.limit_price === 0 ? "a limit of 0 cr places nothing" : undefined,
        }))
      : []),
    ...(c.governance !== "none"
      ? [
          {
            key: "vote",
            node:
              follow !== null ? (
                <>
                  Vote as <b>{handleOf(follow)}</b> votes when you are away.
                </>
              ) : d.vote_default === "abstain" ? (
                <>
                  <b>Abstain</b> when you are away.
                </>
              ) : (
                <>
                  Leave an uncast ballot <b>uncast</b>.
                </>
              ),
          },
        ]
      : []),
  ];

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
    <div className="mx-auto max-w-page">
      <PageHeader title={t("plan")} meta={<span>Runs every hour, here or not. Sent whole when you save.</span>} />

      <Stack>
        <Card title="Your plan" icon="plan" aside={draft !== null ? <Pill tone="attn">unsaved changes</Pill> : null} testId="plan-summary">
          <Verdict parts={verdict} />
          <RuleList rules={rules} />
          <p className="text-muted mt-3 mb-0 text-sm">Each one is edited in its own card below; save at the bottom.</p>
        </Card>

        <Card title="Labor" icon="work" testId="plan-labor">
          <Radios name="labor" value={d.labor} choices={choices} onChange={(labor) => set({ labor })} />
          <p className="text-muted mt-3 text-sm">
            Workplaces, hours and effort live on the{" "}
            <Link to="/s/$id/work" params={{ id: String(id) }}>
              {t("work_screen")}
            </Link>{" "}
            screen.
          </p>
        </Card>

        <Card title="Eating and Wares" icon="food" testId="plan-consumption">
          <div className="grid gap-4 md:grid-cols-2">
            <Tile className="grid content-start gap-3">
              <h3 className="m-0 text-[15px] font-bold">Eating</h3>
              <Field label={`Keep ${pantry} Food at least`} unit="units" hint={`the plan ${c.money ? "bids for" : "draws"} the shortfall each hour`}>
                <Input
                  type="number"
                  inputMode="numeric"
                  min={0}
                  step={1}
                  aria-label="Keep Food at least"
                  value={d.keep_food_at_least}
                  onChange={(e) => set({ keep_food_at_least: Math.max(0, Math.trunc(Number(e.target.value) || 0)) })}
                />
              </Field>
              {c.money ? (
                <MoneyField label="Food price ceiling" optional none="last price × 1.25" value={d.max_food_price} onChange={(v) => set({ max_food_price: v })} hint="the most a Food bid will pay per unit" />
              ) : null}
            </Tile>
            <Tile className="grid content-start gap-3">
              <h3 className="m-0 text-[15px] font-bold">Wares</h3>
              <label className="flex items-center gap-2.5 text-sm">
                <input
                  type="checkbox"
                  aria-label="Buy Wares by rule"
                  checked={d.buy_wares_when !== null}
                  onChange={(e) => set({ buy_wares_when: e.target.checked ? { comfort_below: 50, balance_above: 0, max_price: null } : null })}
                />
                <b>
                  {c.money ? "Buy" : "Draw"} Wares when Comfort is low
                </b>
              </label>
              {d.buy_wares_when ? (
                <>
                  <Field label="Comfort below">
                    <Input
                      type="number"
                      inputMode="numeric"
                      min={0}
                      max={100}
                      step={1}
                      aria-label="Comfort below"
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
                  </Field>
                  {c.money ? (
                    <>
                      <MoneyField label="and balance above" value={d.buy_wares_when.balance_above} onChange={(v) => set({ buy_wares_when: { ...d.buy_wares_when!, balance_above: v ?? 0 } })} />
                      <MoneyField
                        label="Wares price ceiling"
                        optional
                        none="last price × 1.25"
                        value={d.buy_wares_when.max_price}
                        onChange={(v) => set({ buy_wares_when: { ...d.buy_wares_when!, max_price: v } })}
                      />
                    </>
                  ) : null}
                </>
              ) : null}
            </Tile>
          </div>
        </Card>

        {c.money ? (
          <Card title="Saving" icon="coin" testId="plan-saving">
            <MoneyField label="Keep balance at least" value={d.keep_balance_at_least} onChange={(v) => set({ keep_balance_at_least: v ?? 0 })} hint="a hard floor: the plan's bids never commit money below it" />
          </Card>
        ) : null}

        {c.order_books ? (
          <Card title="Standing orders" icon="market" subtitle="Placed again every hour or every day at your limit." testId="plan-orders">
            {d.standing_orders.length === 0 ? (
              <p className="text-muted m-0">None.</p>
            ) : (
              <div className="grid gap-3">
                {d.standing_orders.map((o, i) => {
                  const patch = (p: Partial<StandingOrder>) => set({ standing_orders: d.standing_orders.map((x, j) => (j === i ? { ...x, ...p } : x)) });
                  const isShare = "share" in o.instrument;
                  return (
                    <Tile key={i} className="grid gap-3">
                      <div className="flex items-center justify-between gap-3">
                        <b>Order {i + 1}</b>
                        <Button variant="quiet" inline onClick={() => set({ standing_orders: d.standing_orders.filter((_, j) => j !== i) })}>
                          remove
                        </Button>
                      </div>
                      <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-3">
                        <Field label="Instrument">
                          <Select
                            aria-label={`Order ${i + 1} instrument kind`}
                            value={isShare ? "share" : "good"}
                            onChange={(e) => patch({ instrument: e.target.value === "share" ? { share: 1 } : { good: "food" } })}
                          >
                            <option value="good">good</option>
                            <option value="share">share</option>
                          </Select>
                        </Field>
                        {isShare ? (
                          <Field label="Organization no.">
                            <Input
                              type="number"
                              min={1}
                              aria-label={`Order ${i + 1} org`}
                              value={(o.instrument as { share: number }).share}
                              onChange={(e) => patch({ instrument: { share: Math.max(1, Math.trunc(Number(e.target.value) || 1)) } })}
                            />
                          </Field>
                        ) : (
                          <Field label="Good">
                            <Select aria-label={`Order ${i + 1} good`} value={(o.instrument as { good: string }).good} onChange={(e) => patch({ instrument: { good: e.target.value } })}>
                              {GOODS.map((g) => (
                                <option key={g} value={g}>
                                  {g}
                                </option>
                              ))}
                            </Select>
                          </Field>
                        )}
                        <Field label="Side">
                          <Select aria-label={`Order ${i + 1} side`} value={o.side} onChange={(e) => patch({ side: e.target.value as Side })}>
                            <option value="bid">bid</option>
                            <option value="ask">ask</option>
                          </Select>
                        </Field>
                        <Field label="Quantity">
                          <Input type="number" min={1} step={1} aria-label={`Order ${i + 1} quantity`} value={o.qty} onChange={(e) => patch({ qty: Math.max(0, Math.trunc(Number(e.target.value) || 0)) })} />
                        </Field>
                        <Field label="Limit" unit="cr">
                          <Input type="number" min={0} step="0.01" aria-label={`Order ${i + 1} limit`} value={fromCents(o.limit_price)} onChange={(e) => patch({ limit_price: toCents(e.target.value) })} />
                        </Field>
                        <Field label="Refresh">
                          <Select aria-label={`Order ${i + 1} refresh`} value={o.refresh} onChange={(e) => patch({ refresh: e.target.value as Refresh })}>
                            <option value="each_tick">each hour</option>
                            <option value="each_cycle">each day</option>
                          </Select>
                        </Field>
                      </div>
                    </Tile>
                  );
                })}
              </div>
            )}
            <ButtonRow>
              <Button
                onClick={() =>
                  set({
                    standing_orders: [...d.standing_orders, { instrument: { good: "food" }, side: "bid", qty: 1, limit_price: 100, refresh: "each_cycle" }],
                  })
                }
              >
                Add a standing order
              </Button>
            </ButtonRow>
          </Card>
        ) : null}

        {c.governance !== "none" ? (
          <Card title="Votes" icon="assembly" subtitle="What your ballot does when you are not here to cast it." testId="plan-vote">
            <Radios
              name="vote"
              value={follow !== null ? "follow" : (d.vote_default as "none" | "abstain")}
              choices={[
                { value: "none", label: "No default", hint: "an uncast ballot stays uncast" },
                { value: "abstain", label: "Abstain", hint: "counts toward quorum, neither for nor against" },
                { value: "follow", label: "Follow a citizen", hint: "votes as they vote" },
              ]}
              onChange={(v) =>
                set({
                  vote_default: v === "follow" ? { follow: follow ?? (citizens.data?.citizens ?? []).find((z) => z.id !== h?.citizen.id)?.id ?? 1 } : v,
                })
              }
            />
            {follow !== null ? (
              <Field label="Whose ballot" hint="one hop: if they cast no ballot of their own, you abstain" className="mt-3">
                <Select aria-label="Citizen to follow" value={follow} onChange={(e) => set({ vote_default: { follow: Number(e.target.value) } })}>
                  {(citizens.data?.citizens ?? [])
                    .filter((z) => z.id !== h?.citizen.id)
                    .map((z) => (
                      <option key={z.id} value={z.id}>
                        {z.handle}
                      </option>
                    ))}
                  {(citizens.data?.citizens ?? []).some((z) => z.id === follow) ? null : <option value={follow}>citizen no. {follow}</option>}
                </Select>
              </Field>
            ) : null}
          </Card>
        ) : null}

        <Card title="Save" testId="plan-save">
          <p className="text-muted m-0 text-sm">The plan is sent whole; the engine checks it against the constitution and says what it refuses.</p>
          <ButtonRow>
            <Button variant="primary" disabled={setPlan.isPending} onClick={save}>
              Save my plan
            </Button>
            <Button variant="quiet" disabled={draft === null} onClick={() => setDraft(null)}>
              Discard changes
            </Button>
            {saved && draft === null ? <span className="text-muted text-sm">Saved. It acts from the next hour.</span> : null}
            {error ? (
              <span className="text-crit text-sm" role="alert">
                {error}
              </span>
            ) : null}
          </ButtonRow>
          {h ? (
            <More summary="What the plan did since you last looked" testId="plan-activity">
              <p className="text-muted mb-2 text-sm">
                Trades and draws since {whenOfTick(h.since_last_seen.since_tick + 1, h.clock.ticks_per_cycle)}, the plan&apos;s and your own alike; the wire keeps no separate log of the plan&apos;s runs.
              </p>
              <DiffSinceLastSeen events={h.since_last_seen.events.filter((e) => e.kind === "Trade" || e.kind === "Drew")} t={t} me={h.citizen.id} />
            </More>
          ) : null}
        </Card>

        {h ? (
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
        ) : null}
      </Stack>
    </div>
  );
}
