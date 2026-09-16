// Home, the Situation view (GDD 9.1 step 1, TDD 11): meters, pantry,
// dwelling, balance, labor status, headlines, what the plan did while you
// were away, the next tick and cycle, and one tap to keep the plan.

import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits, type EventRef } from "../api/client";
import { useHome, useLexicon, useSociety } from "../api/hooks";
import { usePayslips, useSetPlan } from "../api/society";
import { Countdown } from "../components/Countdown";
import { DiffSinceLastSeen } from "../components/DiffSinceLastSeen";
import { Ledger } from "../components/Ledger";
import { Meter } from "../components/Meter";
import { Num } from "../components/Num";
import { orgNamer, payslipRows } from "../lib/payslips";
import { Onboarding } from "./Onboarding";

function nextCycleAt(nextTick: string | null | undefined, tickSeconds: number, tick: number, perCycle: number) {
  if (!nextTick || tickSeconds === 0) return null;
  const remaining = perCycle - tick; // ticks after this one until the cycle closes
  return new Date(new Date(nextTick).getTime() + remaining * tickSeconds * 1000).toISOString();
}

export function Home({ id }: { id: number }) {
  const home = useHome(id);
  const society = useSociety(id);
  const slips = usePayslips(id);
  const { t } = useLexicon(id);
  const keep = useSetPlan(id);
  const [kept, setKept] = useState<number | null>(null);
  // A non-citizen is onboarded; the flow stays mounted past the join (which
  // makes Home load) until its last step says it is done.
  const [onboarding, setOnboarding] = useState(false);
  const notCitizen = home.error instanceof ApiError && home.error.status === 403;
  useEffect(() => {
    if (notCitizen) setOnboarding(true);
  }, [notCitizen]);

  if (onboarding) {
    return (
      <Onboarding
        id={id}
        onDone={() => {
          setOnboarding(false);
          void home.refetch();
        }}
      />
    );
  }
  if (home.isPending || notCitizen) return <p className="text-muted">Loading.</p>;
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  const s = society.data;
  const nextCycle = s
    ? nextCycleAt(s.next_tick_at, s.tick_seconds, h.clock.tick, h.clock.ticks_per_cycle)
    : null;
  const rent = h.household.dwelling?.rent_per_cycle;

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">
          {t("home_title")} <span className="text-muted text-base">{h.citizen.handle}</span>
        </h2>
        <div className="flex gap-4">
          <Countdown at={s?.next_tick_at} label="next tick" />
          <Countdown at={nextCycle} label="payday" />
        </div>
      </header>

      {h.citizen.flags && (h.citizen.flags as Record<string, boolean>).in_hardship ? (
        <p className="text-bad text-sm">You are in hardship: Food has been under 20 for a whole cycle. Eat first.</p>
      ) : null}

      <section className="grid gap-8 md:grid-cols-2">
        <div className="flex flex-col gap-2">
          <Meter label="Food" value={h.needs.food} />
          <Meter label="Shelter" value={h.needs.shelter} />
          <Meter label="Comfort" value={h.needs.comfort} />
        </div>
        <dl className="grid grid-cols-2 gap-y-1 text-sm">
          <dt className="text-muted">{t("balance")}</dt>
          <dd className="num">
            <Num value={credits(h.household.balance)} unit="cr" />
          </dd>
          <dt className="text-muted">{t("pantry")}</dt>
          <dd className="num" data-testid="pantry">
            {Object.entries(h.household.pantry)
              .map(([g, n]) => `${n} ${g}`)
              .join(", ") || "empty"}
          </dd>
          <dt className="text-muted">{t("dwelling")}</dt>
          <dd>
            {h.household.dwelling
              ? `#${h.household.dwelling.id}${rent != null ? `, ${credits(rent)} cr a cycle` : ""}`
              : "none (Shelter falls until you rent)"}
          </dd>
          <dt className="text-muted">{t("work_screen")}</dt>
          <dd>
            {h.labor.allocations.length === 0
              ? "no position"
              : h.labor.allocations.map((a) => `${a.org_name}: ${a.hours} h, ${a.effort}`).join("; ")}
            <span className="text-muted"> · budget {h.labor.budget} h · </span>
            <Link to="/s/$id/work" params={{ id: String(id) }} className="text-muted underline">
              adjust
            </Link>
          </dd>
        </dl>
      </section>

      <section className="grid gap-8 md:grid-cols-2">
        <div>
          <h3 className="text-lg">While you were away</h3>
          <p className="text-muted mb-2 text-xs">since tick {h.since_last_seen.since_tick + 1}</p>
          <DiffSinceLastSeen events={h.since_last_seen.events} t={t} me={h.citizen.id} />
        </div>
        <div>
          <h3 className="text-lg">{t("compensation")}</h3>
          <div className="mt-2" data-testid="payslips">
            <Ledger
              rows={
                slips.data
                  ? payslipRows(
                      slips.data.payslips as unknown as EventRef[],
                      orgNamer(h.labor.allocations, h.labor.employment),
                      5,
                    )
                  : []
              }
              empty="No payslip yet. The first comes at the end of the cycle."
            />
          </div>
        </div>
      </section>

      <section className="grid gap-8 md:grid-cols-2">
        <div>
          <h3 className="text-lg">{t("chronicle")}</h3>
          {h.headlines.length === 0 ? (
            <p className="text-muted mt-2 text-sm">Nothing to report yet.</p>
          ) : (
            <ul className="mt-2 flex flex-col gap-1 text-sm">
              {h.headlines.map((hl) => (
                <li key={hl.seq}>
                  <Link to="/s/$id/events/$seq" params={{ id: String(id), seq: String(hl.seq) }} className="underline decoration-dotted">
                    {hl.text}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div>
          <h3 className="text-lg">{t("plan")}</h3>
          <p className="text-muted mt-2 text-sm">
            Keep Food at least {String((h.plan as Record<string, unknown>).keep_food_at_least)}; keep{" "}
            {credits(Number((h.plan as Record<string, unknown>).keep_balance_at_least ?? 0))} cr in hand.
          </p>
          <button
            type="button"
            disabled={keep.isPending}
            className="border-line mt-3 rounded-sm border px-3 py-1 text-sm"
            onClick={() =>
              keep.mutate(h.plan as Record<string, unknown>, { onSuccess: () => setKept(h.clock.cycle) })
            }
          >
            Keep my plan
          </button>
          <Link to="/s/$id/plan" params={{ id: String(id) }} className="text-muted ml-3 text-sm underline">
            Edit plan
          </Link>
          {kept !== null ? <span className="text-muted ml-3 text-sm">kept for cycle {kept}</span> : null}
        </div>
      </section>
      <p className="text-muted text-xs">
        {h.society.population} citizens, {h.society.active_humans} people, {h.society.unemployed} without work
        {h.society.price_index != null ? `, ${t("society_stat").toLowerCase()} ${h.society.price_index.toFixed(2)}` : ""}.
      </p>
    </div>
  );
}
