// Work (GDD 4.3, TDD 11; S1.9; docs/style.md §10 Work): where your hours go.
// The Verdict, the hours used against the budget as a figure with a bar, one
// tile per position you hold (at most `max_workplaces`) with its hours and
// effort and what that effort costs, the payslips with their Explain, the
// skill table, and the output rule as the detail. The engine validates; its
// rejection text is shown as is. Under a work norm (S2.7) positions come
// without a contract: the picker lists every workplace with room, and the
// Ledger of Contribution, not a payslip, is the record.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { credits, type EventRef } from "../api/client";
import { useLedger, useLeavePosition, useTakePosition } from "../api/commons";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { useOrgsView } from "../api/orgs";
import { usePayslips, useSetLabor, type Allocation, type Effort, type LaborView } from "../api/society";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { FactList } from "../components/FactList";
import { Field, Input } from "../components/Field";
import { Ledger, TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { Meter } from "../components/Meter";
import { More } from "../components/More";
import { Num } from "../components/Num";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Segmented } from "../components/Segmented";
import { Verdict, type Tone } from "../components/Verdict";
import { useNames } from "../lib/names";
import { payslipRows } from "../lib/payslips";
import { workVerdict } from "../lib/verdict";

const EFFORTS: Effort[] = ["low", "normal", "high"];

type Position = {
  contract: number | null;
  org: number;
  workplace: number;
  pay: string;
  maxHours: number;
};

/** Every position held: by contract (its pay and cap) or, under a norm, none (the budget is the cap). */
function positions(l: LaborView, byNeed: string): Position[] {
  return (l.positions ?? []).map((p) => {
    const k = p.contract != null ? l.employment.find((x) => x.id === p.contract) : undefined;
    const e = k?.body.employment as Record<string, unknown> | undefined;
    const pay = (e?.pay ?? {}) as Record<string, number>;
    return {
      contract: p.contract ?? null,
      org: p.org,
      workplace: p.workplace,
      pay: !e
        ? byNeed
        : pay.hourly !== undefined
          ? `${credits(pay.hourly)} cr/h`
          : pay.piece_rate !== undefined
            ? `${credits(pay.piece_rate)} cr/unit`
            : Object.keys(pay).join(", "),
      maxHours: e ? Number(e.max_hours) : l.budget,
    };
  });
}

/** What each effort level costs, from the preset by way of the labor view (§8.5: multipliers as ×). */
function effortNote(l: LaborView, effort: Effort): string {
  const i = EFFORTS.indexOf(effort);
  const out = l.effort.output_mult[i];
  const decay = l.effort.food_decay_mult[i];
  const base = `output ×${out.toFixed(1)} · Food decay ×${decay.toFixed(1)}`;
  if (effort !== "high") return base;
  return `${base} · after ${l.effort.high_effort_debt_after_cycles} days in a row, ${l.effort.high_effort_debt_hours} h of fatigue debt a day`;
}

function initialRows(l: LaborView): Allocation[] {
  return positions(l, "").map((p) => {
    const a = l.allocations.find((x) => x.workplace === p.workplace);
    return { workplace: p.workplace, hours: a?.hours ?? 0, effort: (a?.effort as Effort | undefined) ?? "normal" };
  });
}

const STATUS: Record<Tone, string> = { good: "text-muted", attn: "text-attn", crit: "text-crit" };

export function Work({ id }: { id: number }) {
  const home = useHome(id);
  const slips = usePayslips(id);
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const byNorm = caps.data?.labor === "norm";
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const setLabor = useSetLabor(id);
  // The picker's data, only under a norm: every workplace with its head count and the cap.
  const orgs = useOrgsView(id);
  const ledger = useLedger(id, byNorm);
  const take = useTakePosition(id);
  const leave = useLeavePosition(id);
  const [rows, setRows] = useState<Allocation[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [taken, setTaken] = useState<string | null>(null);
  const [pickError, setPickError] = useState<string | null>(null);
  // Until the first edit the editor mirrors the engine's allocations; edits
  // then survive ticks, and a save clears them so the reply shows through.

  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error) return <p className="text-crit">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  const l = h.labor;
  const held = positions(l, "by need, from the Store");
  const name = names.org;
  // The Plan's target per workplace (S2.8), advisory under a direct assembly: read beside your hours.
  const targets = new Map((orgs.data?.orgs ?? []).flatMap((o) => o.workplaces.map((w) => [w.id, { target: w.target ?? null, yesterday: w.last_cycle_output ?? 0 }] as const)));
  const anyTarget = [...targets.values()].some((x) => x.target != null);
  const edit = rows ?? initialRows(l);
  const total = edit.reduce((n, r) => n + r.hours, 0);
  const over = total > l.budget;

  // The glance reads the engine's allocations, not the draft: what is true this hour.
  const engineHours = l.allocations.reduce((n, a) => n + a.hours, 0);
  const verdict = workVerdict({
    hours: engineHours,
    budget: l.budget,
    fatigue: l.fatigue_debt,
    positions: held.map((p) => {
      const a = l.allocations.find((x) => x.workplace === p.workplace);
      return { name: name(p.org), hours: a?.hours ?? 0, effort: a?.effort ?? "normal" };
    }),
    byNorm,
  });
  const hoursTone: Tone = held.length === 0 || engineHours === 0 ? "attn" : "good";
  const hoursStatus =
    held.length === 0
      ? "No position, so no hours."
      : engineHours === 0
        ? "Nothing set yet. Hours count from the next hour after you set them."
        : engineHours < l.budget
          ? `${l.budget - engineHours} h of today's budget unused${l.fatigue_debt > 0 ? `; fatigue owes ${l.fatigue_debt} h` : ""}.`
          : `The whole budget${l.fatigue_debt > 0 ? `, ${l.fatigue_debt} h short for fatigue` : ""}.`;
  const efforts = new Set(l.allocations.filter((a) => a.hours > 0).map((a) => a.effort as Effort));

  const update = (workplace: number, patch: Partial<Allocation>) =>
    setRows(edit.map((r) => (r.workplace === workplace ? { ...r, ...patch } : r)));

  const save = () => {
    setError(null);
    setSaved(false);
    setLabor.mutate(
      edit.filter((r) => r.hours > 0),
      {
        onSuccess: () => {
          setSaved(true);
          setRows(null);
        },
        onError: (e) => setError(e.message),
      },
    );
  };

  return (
    <div className="mx-auto max-w-page">
      <PageHeader
        title={t("work_screen")}
        meta={
          <>
            <span>
              budget <b>{l.budget} h</b> a day
            </span>
            {l.fatigue_debt > 0 ? (
              <span>
                fatigue debt <b>{l.fatigue_debt} h</b>
              </span>
            ) : null}
          </>
        }
      />

      <Stack>
        <Card title="Your hours" icon="work" testId="your-hours">
          <Verdict parts={verdict} />

          <Tile testId="hours-glance" className="grid gap-1.5">
            <div className="flex items-baseline justify-between gap-3">
              <span className="font-display text-2xl leading-none font-bold">
                {engineHours}
                <small className="text-muted ml-1 font-body text-xs font-normal">/ {l.budget} h today</small>
              </span>
              {efforts.size > 0 ? (
                <span className="text-muted text-sm">
                  {[...efforts].join(" and ")} effort
                </span>
              ) : null}
            </div>
            <Meter label="Hours used" value={l.budget > 0 ? (engineHours / l.budget) * 100 : 0} threshold={0} tone={hoursTone} bare />
            <p className={`m-0 text-[13.5px] ${STATUS[hoursTone]}`}>{hoursStatus}</p>
            {[...efforts].map((e) => (
              <p key={e} className="text-muted m-0 text-[13.5px]">
                {e} effort: {effortNote(l, e)}
              </p>
            ))}
          </Tile>

          {held.length === 0 ? (
            byNorm ? (
              <p className="text-muted mt-4">You hold no position. Take one below; any workplace with room is yours to join.</p>
            ) : (
              <p className="text-muted mt-4">
                You hold no {t("job").toLowerCase()}. The job board is always on the{" "}
                <Link to="/s/$id/orgs" params={{ id: String(id) }}>
                  Organizations
                </Link>{" "}
                screen.
              </p>
            )
          ) : (
            <div className="mt-4 grid gap-3" data-testid="allocation-editor">
              {held.map((p) => {
                const r = edit.find((x) => x.workplace === p.workplace) ?? {
                  workplace: p.workplace,
                  hours: 0,
                  effort: "normal" as Effort,
                };
                const target = targets.get(p.workplace);
                return (
                  <Tile key={p.workplace} className="grid gap-3">
                    <div>
                      <div className="flex flex-wrap items-center gap-2 font-bold">
                        <span>{name(p.org)}</span>
                        <span className="text-muted text-sm font-normal tabular-nums">{p.pay}</span>
                      </div>
                      <p className="text-muted m-0 text-sm">
                        {names.workplaceTitle(p.workplace)} · up to {p.maxHours} h a day
                        {target?.target != null ? (
                          <span data-testid={`target-${p.workplace}`}>
                            {" · "}the Plan asks {target.target.toFixed(0)} a day; yesterday it made {target.yesterday.toFixed(0)}
                          </span>
                        ) : null}
                        {p.contract === null ? (
                          <>
                            {" · "}
                            <button
                              type="button"
                              className="text-accent underline"
                              disabled={leave.isPending}
                              onClick={() => {
                                setPickError(null);
                                leave.mutate(p.workplace, { onSuccess: () => setRows(null), onError: (e) => setPickError(e.message) });
                              }}
                            >
                              leave
                            </button>
                          </>
                        ) : null}
                      </p>
                    </div>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <Field label="Hours" unit="h">
                        <Input
                          type="number"
                          inputMode="numeric"
                          min={0}
                          max={p.maxHours}
                          step={1}
                          aria-label={`Hours at ${names.workplace(p.workplace)}`}
                          value={r.hours}
                          onChange={(e) => update(p.workplace, { hours: Math.min(p.maxHours, Math.max(0, Math.trunc(Number(e.target.value) || 0))) })}
                        />
                      </Field>
                      <div className="grid content-start gap-1.5">
                        <span className="text-sm font-bold">Effort</span>
                        <Segmented
                          label={`Effort at ${names.workplace(p.workplace)}`}
                          value={r.effort}
                          options={EFFORTS.map((ef) => ({ value: ef, label: ef }))}
                          onChange={(effort) => update(p.workplace, { effort })}
                        />
                        <span className="text-muted text-sm">{effortNote(l, r.effort)}</span>
                      </div>
                    </div>
                  </Tile>
                );
              })}
            </div>
          )}
          {held.length > 0 ? (
            <ButtonRow>
              <Button variant="primary" disabled={setLabor.isPending} disabledReason={over ? "the day has no more hours than that" : undefined} onClick={save}>
                Set my hours
              </Button>
              <span className={`text-sm tabular-nums ${over ? "text-crit" : "text-muted"}`}>
                {total} of {l.budget} h
              </span>
              {saved ? <span className="text-muted text-sm">Set. It counts from the next hour.</span> : null}
              {error ? (
                <span className="text-crit text-sm" role="alert">
                  {error}
                </span>
              ) : null}
            </ButtonRow>
          ) : null}

          <More summary="How output is worked out" testId="output-detail">
            <FactList
              items={[
                { key: "budget", label: "Hour budget", value: `${l.budget} h a day`, gloss: l.fatigue_debt > 0 ? `${l.fatigue_debt} h owed to fatigue` : "recovers daily" },
                { key: "output", label: "Output today", value: `×${l.output_mult.toFixed(2)}`, gloss: l.output_mult < 1 ? "hunger or no dwelling" : undefined },
                ...EFFORTS.map((e, i) => ({
                  key: `effort-${e}`,
                  label: `Effort ${e}`,
                  value: `×${(l.effort.output_mult[i] ?? 1).toFixed(2)} output`,
                  gloss: `×${(l.effort.food_decay_mult[i] ?? 1).toFixed(2)} food decay`,
                })),
                { key: "debt", label: "High effort", value: `${l.effort.high_effort_debt_hours} h of debt a day`, gloss: `after ${l.effort.high_effort_debt_after_cycles} days in a row` },
                { key: "workplaces", label: "Workplaces", value: `up to ${l.effort.max_workplaces} at once` },
              ]}
            />
            <p className="text-muted mt-3 max-w-prose text-sm">
              Hours go to at most {l.effort.max_workplaces} workplaces and never beyond the budget, which recovers each day and shrinks with fatigue. Output per
              hour is the base rate times skill, effort, and the workplace&apos;s machines.
            </p>
          </More>
        </Card>

        {byNorm ? (
          <Card title="Take a position" icon="org" testId="positions">
            <p className="text-muted mb-3 max-w-prose text-sm">
              Under the norm there is no contract and no wage: any workplace with room is yours to join and yours to leave, at most{" "}
              {ledger.data?.max_workplaces ?? l.effort.max_workplaces} at once, and what you give goes on the{" "}
              <Link to="/s/$id/ledger" params={{ id: String(id) }}>
                {t("ledger_screen")}
              </Link>
              .{ledger.data?.least_staffed != null ? ` Labor is scarcest at the ${names.workplace(ledger.data.least_staffed)}.` : ""}
            </p>
            {orgs.data ? (
              <table className="w-full border-collapse text-[15px]" data-testid="workplace-picker">
                <thead>
                  <tr>
                    <th className={TH}>Workplace</th>
                    <th className={TH_NUM}>Working there</th>
                    {anyTarget ? <th className={TH_NUM}>The Plan asks</th> : null}
                    <th className={TH} />
                  </tr>
                </thead>
                <tbody>
                  {orgs.data.orgs.flatMap((o) =>
                    o.workplaces.map((w) => {
                      const mine = held.some((p) => p.workplace === w.id);
                      const cap = ledger.data?.max_workers_per_workplace;
                      const full = cap !== undefined && w.workers.length >= cap;
                      const scarce = ledger.data?.least_staffed === w.id;
                      return (
                        <tr key={w.id} className="hover:bg-surface-2" data-testid={`pick-${w.id}`}>
                          <td className={TD}>
                            {names.workplace(w.id)}
                            {scarce ? <span className="text-accent block text-xs">labor is scarcest here</span> : null}
                          </td>
                          <td className={TD_NUM}>
                            {w.workers.length}
                            {cap !== undefined ? <span className="text-muted text-xs"> of {cap}</span> : null}
                          </td>
                          {anyTarget ? <td className={TD_NUM}>{targets.get(w.id)?.target != null ? `${targets.get(w.id)!.target!.toFixed(0)} a day` : <span className="text-muted">—</span>}</td> : null}
                          <td className={`${TD} text-right`}>
                            {mine ? (
                              <Pill>yours</Pill>
                            ) : (
                              <Button
                                inline
                                disabled={take.isPending || full}
                                onClick={() => {
                                  setPickError(null);
                                  setTaken(null);
                                  take.mutate(w.id, {
                                    onSuccess: () => {
                                      setRows(null);
                                      setTaken(`Taken: the ${names.workplace(w.id)}. Set your hours above.`);
                                    },
                                    onError: (e) => setPickError(e.message),
                                  });
                                }}
                              >
                                {full ? "Full" : "Take a position"}
                              </Button>
                            )}
                          </td>
                        </tr>
                      );
                    }),
                  )}
                </tbody>
              </table>
            ) : (
              <p className="text-muted">Loading.</p>
            )}
            {taken ? <p className="text-muted mt-3 text-sm">{taken}</p> : null}
            {pickError ? (
              <p className="text-crit mt-3 text-sm" role="alert">
                {pickError}
              </p>
            ) : null}
          </Card>
        ) : null}

        <Two>
          <Card title={t("compensation")} icon="coin">
            {byNorm ? (
              <p className="text-muted m-0">
                No payslip here: you draw from the Store by need, and your hours are on the{" "}
                <Link to="/s/$id/ledger" params={{ id: String(id) }}>
                  {t("ledger_screen")}
                </Link>
                .
              </p>
            ) : (
              <div data-testid="payslips">
                <Ledger rows={slips.data ? payslipRows(slips.data.payslips as unknown as EventRef[], name) : []} empty="No payslip yet. The first comes at the end of the day." />
              </div>
            )}
          </Card>
          <Card title="Skill" icon="gear">
            {l.skills.length === 0 ? (
              <p className="text-muted m-0">Nothing yet. Skill grows with hours worked in a job family and fades when unused.</p>
            ) : (
              <table className="w-full border-collapse text-[15px]" data-testid="skills">
                <thead>
                  <tr>
                    <th className={TH}>Family</th>
                    <th className={TH_NUM}>Level</th>
                    <th className={TH_NUM}>Output</th>
                    <th className={TH_NUM}>Hours</th>
                  </tr>
                </thead>
                <tbody>
                  {l.skills.map((s) => (
                    <tr key={s.family} className="hover:bg-surface-2">
                      <td className={TD}>{s.family}</td>
                      <td className={TD_NUM}>{s.level.toFixed(0)}</td>
                      <td className={TD_NUM}>
                        <Num value={`×${(1 + s.level / 100).toFixed(2)}`} />
                      </td>
                      <td className={TD_NUM}>{s.hours.toFixed(0)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </Card>
        </Two>

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
