// Home, the Situation view (GDD 9.1 step 1, TDD 11; docs/style.md §10 Home):
// the Verdict, three NeedCards, the household facts, what the plan did while
// you were away, the compensation record, the plan as a checklist, the
// Chronicle, and the footer strip. Every datum the old Home showed is here,
// each at its layer (§2); the hour budget and the multipliers are the detail.

import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ApiError, credits, type EventRef } from "../api/client";
import { useCapabilities, useHome, useLexicon, useSociety } from "../api/hooks";
import { useStore } from "../api/commons";
import { usePayslips, useSetPlan } from "../api/society";
import { Button, ButtonLink, ButtonRow } from "../components/Button";
import { Card, Stack, Two } from "../components/Card";
import { Countdown } from "../components/Countdown";
import { DiffSinceLastSeen } from "../components/DiffSinceLastSeen";
import { FactList, type Fact } from "../components/FactList";
import { Feed, RuleList } from "../components/Feed";
import { Ledger } from "../components/Ledger";
import { More } from "../components/More";
import { NeedCard, Needs } from "../components/NeedCard";
import { Num } from "../components/Num";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { useNames } from "../lib/names";
import { drawRows } from "../lib/draws";
import { needHints, needStatus } from "../lib/needs";
import { payslipRows } from "../lib/payslips";
import { greeting, homeVerdict } from "../lib/verdict";
import { epochEndingText, whenOfTick } from "../lib/when";
import { Onboarding } from "./Onboarding";

function nextCycleAt(nextTick: string | null | undefined, tickSeconds: number, tick: number, perCycle: number) {
  if (!nextTick || tickSeconds === 0) return null;
  const remaining = perCycle - tick; // ticks after this one until the cycle closes
  return new Date(new Date(nextTick).getTime() + remaining * tickSeconds * 1000).toISOString();
}

/** A headline that names trouble is hot (§7.13); housekeeping stays muted. */
const HOT = /hardship|hungry|default|reject|destitut|evict|unpaid|short/i;

const EFFORTS = ["low", "normal", "high"] as const;

export function Home({ id }: { id: number }) {
  const home = useHome(id);
  const society = useSociety(id);
  const slips = usePayslips(id);
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const hasStore = caps.data?.common_store === true;
  const store = useStore(id, hasStore && home.data !== undefined);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
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
  if (home.error) return <p className="text-crit">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  const s = society.data;
  const nextCycle = s ? nextCycleAt(s.next_tick_at, s.tick_seconds, h.clock.tick, h.clock.ticks_per_cycle) : null;
  const rent = h.household.dwelling?.rent_per_cycle;
  const hints = needHints(t, caps.data);
  const ending = epochEndingText(h.clock);
  const money = caps.data?.money !== false;
  const shelf = (good: string) => store.data?.stock.find((s) => s.good === good);
  const food = shelf("food");
  const wares = shelf("wares");
  const pantry = h.household.pantry as Record<string, number>;
  const hardship = Boolean((h.citizen.flags as Record<string, boolean>).in_hardship);
  const housed = h.household.dwelling != null;
  const hours = h.labor.allocations.reduce((n, a) => n + a.hours, 0);
  const hired = h.labor.employment.length > 0 || (h.labor.positions?.length ?? 0) > 0;
  const plan = h.plan as Record<string, unknown>;
  const keepFood = Number(plan.keep_food_at_least ?? 0);
  const keepBalance = Number(plan.keep_balance_at_least ?? 0);
  const verdict = homeVerdict({ food: h.needs.food, shelter: h.needs.shelter, comfort: h.needs.comfort, hardship, housed, hours, hired });
  const status = needStatus(t, {
    food: h.needs.food,
    shelter: h.needs.shelter,
    comfort: h.needs.comfort,
    hardship,
    housed,
    pantryFood: pantry.food ?? 0,
    pantryWares: pantry.wares ?? 0,
  });
  const pantryText =
    Object.entries(pantry)
      .map(([g, n]) => `${n} ${g}`)
      .join(", ") || "empty";
  const effort = h.labor.effort;

  const facts: Fact[] = [];
  if (money) facts.push({ key: "balance", label: t("balance"), value: <Num value={credits(h.household.balance)} unit="cr" /> });
  if (hasStore) {
    facts.push({
      key: "store",
      label: t("store"),
      testId: "store-tile",
      value: store.data ? (
        <>
          {food?.stock ?? 0} food, {wares?.stock ?? 0} wares on the shelf
          <span className="text-muted block text-xs">
            {food && food.my_pending > 0
              ? `${food.my_pending} Food asked this hour`
              : food && food.my_entitlement > 0
                ? `you may draw ${food.my_entitlement} Food`
                : "your Food meter is full"}
            {" · "}
            <Link to="/s/$id/store" params={{ id: String(id) }}>
              the shelves
            </Link>
          </span>
        </>
      ) : (
        <span className="text-muted">Loading.</span>
      ),
    });
  }
  facts.push({
    key: "pantry",
    label: t("pantry"),
    testId: "pantry",
    value: pantryText,
    // One gloss per screen (§8.5): a unit of Food is about an hour of eating.
    gloss: (pantry.food ?? 0) > 0 ? `about ${pantry.food} hours` : undefined,
  });
  facts.push({
    key: "dwelling",
    label: t("dwelling"),
    value: h.household.dwelling ? `No. ${h.household.dwelling.id}` : <Pill tone="attn">none</Pill>,
    gloss: h.household.dwelling ? (rent != null ? `${credits(rent)} cr a day` : undefined) : "Shelter falls until you rent",
  });
  facts.push({
    key: "work",
    label: t("work_screen"),
    value:
      h.labor.allocations.length === 0
        ? h.labor.employment.length > 0
          ? "hired, but no hours set"
          : "no position"
        : h.labor.allocations.map((a) => `${a.org_name}: ${a.hours} h, ${a.effort}`).join("; "),
    action: (
      <Link to="/s/$id/work" params={{ id: String(id) }}>
        adjust
      </Link>
    ),
  });

  return (
    <div className="mx-auto max-w-page">
      <PageHeader
        title={greeting(h.citizen.handle, h.clock)}
        meta={
          <>
            <Countdown at={s?.next_tick_at} label="next hour" />
            <Countdown at={nextCycle} label="payday" />
          </>
        }
      />

      {h.clock.epoch_ended ? (
        <p className="mb-4 flex flex-wrap items-center gap-2 text-sm" data-testid="epoch-ended">
          <Pill tone="info">epoch over</Pill>
          <span>
            The epoch has ended and the ledgers are closed.{" "}
            <Link to="/s/$id/archives" params={{ id: String(id) }}>
              Read the archive and leave your closing statement
            </Link>{" "}
            before the next epoch opens.
          </span>
        </p>
      ) : ending ? (
        <p className="text-attn mb-4 flex flex-wrap items-center gap-2 text-sm" data-testid="epoch-ending">
          <Pill tone="attn">epoch ending</Pill>
          <span>{ending} Settle what you can.</span>
        </p>
      ) : null}

      <Stack>
        <Card title={t("home_title")} aside={hardship ? <Pill tone="crit">in hardship</Pill> : null} testId="right-now">
          <Verdict parts={verdict} />
          <Needs>
            <NeedCard label="Food" icon="food" value={h.needs.food} status={status.food} note={hints.food} testId="need-food" />
            <NeedCard label="Shelter" icon="home" value={h.needs.shelter} status={status.shelter} note={hints.shelter} tone={status.shelterTone} testId="need-shelter" />
            <NeedCard label="Comfort" icon="spark" value={h.needs.comfort} status={status.comfort} note={hints.comfort} testId="need-comfort" />
          </Needs>
          <FactList items={facts} className="mt-4" />
          <More summary="More about today's work" testId="work-detail">
            <FactList
              items={[
                { key: "budget", label: "Hour budget", value: `${h.labor.budget} h a day`, gloss: h.labor.fatigue_debt > 0 ? `${h.labor.fatigue_debt} h owed to fatigue` : "recovers daily" },
                { key: "output", label: "Output today", value: `×${h.labor.output_mult.toFixed(2)}`, gloss: h.labor.output_mult < 1 ? "hunger or no dwelling" : undefined },
                ...EFFORTS.map((e, i) => ({
                  key: `effort-${e}`,
                  label: `Effort ${e}`,
                  value: `×${(effort.output_mult[i] ?? 1).toFixed(2)} output`,
                  gloss: `×${(effort.food_decay_mult[i] ?? 1).toFixed(2)} food decay`,
                })),
                { key: "workplaces", label: "Workplaces", value: `up to ${effort.max_workplaces} at once` },
              ]}
            />
          </More>
        </Card>

        <Two>
          <Card title="While you were away" icon="clock" subtitle={`since ${whenOfTick(h.since_last_seen.since_tick + 1, h.clock.ticks_per_cycle)}`}>
            <DiffSinceLastSeen events={h.since_last_seen.events} t={t} me={h.citizen.id} />
          </Card>
          <Card title={t("compensation")} icon="coin">
            {hasStore ? (
              <div data-testid="draws">
                <Ledger
                  rows={store.data ? drawRows(store.data.my_draws as unknown as EventRef[], h.clock.ticks_per_cycle, 5) : []}
                  empty="No draw yet. Your plan asks the Store for Food when the meter has room for a unit."
                />
              </div>
            ) : (
              <div data-testid="payslips">
                <Ledger
                  rows={slips.data ? payslipRows(slips.data.payslips as unknown as EventRef[], names.org, 5) : []}
                  empty="No payslip yet. The first comes at the end of the day."
                />
              </div>
            )}
          </Card>
        </Two>

        <Card title={t("plan")} icon="plan" subtitle="This runs every hour, even while you're gone.">
          <RuleList
            rules={[
              {
                key: "food",
                node: (
                  <>
                    Keep <b>Food</b> at least <b>{keepFood}</b> in the {t("pantry").toLowerCase()}
                    {hasStore ? " (draw from the Store when it dips)." : caps.data?.order_books ? " (buy more when it dips)." : "."}
                  </>
                ),
              },
              ...(money
                ? [
                    {
                      key: "balance",
                      node: (
                        <>
                          Keep <b>{credits(keepBalance)} cr</b> in hand{keepBalance === 0 ? " (spend freely)." : "."}
                        </>
                      ),
                    },
                  ]
                : []),
            ]}
          />
          <ButtonRow>
            <Button
              variant="primary"
              disabled={keep.isPending}
              onClick={() => keep.mutate(h.plan as Record<string, unknown>, { onSuccess: () => setKept(h.clock.cycle) })}
            >
              Keep my plan
            </Button>
            <ButtonLink to="/s/$id/plan" params={{ id: String(id) }}>
              Edit plan
            </ButtonLink>
            {kept !== null ? <span className="text-muted text-sm">kept for Day {kept}</span> : null}
          </ButtonRow>
        </Card>

        <Card title={t("chronicle")} icon="page" subtitle={s ? `What's happening in ${s.display}` : undefined}>
          <Feed
            testId="home-chronicle"
            empty="Nothing to report yet."
            items={h.headlines.map((hl) => ({
              key: hl.seq,
              hot: HOT.test(hl.text),
              node: (
                <Link to="/s/$id/events/$seq" params={{ id: String(id), seq: String(hl.seq) }} className="font-normal text-current">
                  {hl.text}
                </Link>
              ),
            }))}
          />
        </Card>

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
