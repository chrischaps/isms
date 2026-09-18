// Work (GDD 4.3, TDD 11; S1.9): where your hours go. The allocation editor
// covers the positions you hold (at most `max_workplaces`), hours and effort
// with what each effort costs, the payslips with their Explain, and the
// skill panel. The engine validates; its rejection text is shown as is.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { credits, type EventRef } from "../api/client";
import { useHome, useLexicon } from "../api/hooks";
import { usePayslips, useSetLabor, type Allocation, type Effort, type LaborView } from "../api/society";
import { Ledger } from "../components/Ledger";
import { Num } from "../components/Num";
import { useNames } from "../lib/names";
import { payslipRows } from "../lib/payslips";

const EFFORTS: Effort[] = ["low", "normal", "high"];

type Position = {
  contract: number;
  org: number;
  workplace: number;
  pay: string;
  maxHours: number;
  noticeCycles: number;
};

function positions(l: LaborView): Position[] {
  return l.employment.flatMap((k) => {
    const e = k.body.employment as Record<string, unknown> | undefined;
    if (!e) return [];
    const pay = (e.pay ?? {}) as Record<string, number>;
    return [
      {
        contract: k.id,
        org: Number(e.org),
        workplace: Number(e.workplace),
        pay:
          pay.hourly !== undefined
            ? `${credits(pay.hourly)} cr/h`
            : pay.piece_rate !== undefined
              ? `${credits(pay.piece_rate)} cr/unit`
              : Object.keys(pay).join(", "),
        maxHours: Number(e.max_hours),
        noticeCycles: Number(e.notice_cycles ?? 0),
      },
    ];
  });
}

/** What each effort level costs, from the preset by way of the labor view. */
function effortNote(l: LaborView, effort: Effort): string {
  const i = EFFORTS.indexOf(effort);
  const out = l.effort.output_mult[i];
  const decay = l.effort.food_decay_mult[i];
  const base = `output x${out.toFixed(1)}, Food decay x${decay.toFixed(1)}`;
  if (effort !== "high") return base;
  return `${base}; after ${l.effort.high_effort_debt_after_cycles} days in a row, ${l.effort.high_effort_debt_hours} h of fatigue debt each day`;
}

function initialRows(l: LaborView): Allocation[] {
  return positions(l).map((p) => {
    const a = l.allocations.find((x) => x.workplace === p.workplace);
    return { workplace: p.workplace, hours: a?.hours ?? 0, effort: (a?.effort as Effort | undefined) ?? "normal" };
  });
}

export function Work({ id }: { id: number }) {
  const home = useHome(id);
  const slips = usePayslips(id);
  const { t } = useLexicon(id);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const setLabor = useSetLabor(id);
  const [rows, setRows] = useState<Allocation[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  // Until the first edit the editor mirrors the engine's allocations; edits
  // then survive ticks, and a save clears them so the reply shows through.

  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  const l = h.labor;
  const held = positions(l);
  const name = names.org;
  const edit = rows ?? initialRows(l);
  const total = edit.reduce((n, r) => n + r.hours, 0);
  const over = total > l.budget;

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
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{t("work_screen")}</h2>
        <p className="num text-muted text-sm">
          budget {l.budget} h a day
          {l.fatigue_debt > 0 ? ` (${l.fatigue_debt} h of fatigue debt)` : ""}
        </p>
      </header>

      <section>
        <h3 className="text-lg">Your hours</h3>
        {held.length === 0 ? (
          <p className="text-muted mt-2 text-sm">
            You hold no {t("job").toLowerCase()}. The job board is always on the{" "}
            <Link to="/s/$id/orgs" params={{ id: String(id) }} className="underline">
              Organizations
            </Link>{" "}
            screen.
          </p>
        ) : (
          <table className="mt-2 w-full text-sm" data-testid="allocation-editor">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">{t("job")}</th>
                <th className="py-1 font-normal">Pay</th>
                <th className="py-1 font-normal">Hours</th>
                <th className="py-1 font-normal">Effort</th>
              </tr>
            </thead>
            <tbody>
              {held.map((p) => {
                const r = edit.find((x) => x.workplace === p.workplace) ?? {
                  workplace: p.workplace,
                  hours: 0,
                  effort: "normal" as Effort,
                };
                return (
                  <tr key={p.contract} className="rule align-top">
                    <td className="py-2 pr-3">
                      {name(p.org)}
                      <span className="text-muted block text-xs">
                        {names.workplaceTitle(p.workplace)} · up to {p.maxHours} h a day
                      </span>
                    </td>
                    <td className="num py-2 pr-3 whitespace-nowrap">{p.pay}</td>
                    <td className="py-2 pr-3">
                      <input
                        type="number"
                        inputMode="numeric"
                        min={0}
                        max={p.maxHours}
                        step={1}
                        aria-label={`Hours at ${names.workplace(p.workplace)}`}
                        className="border-line num w-16 rounded-sm border px-1"
                        value={r.hours}
                        onChange={(e) => update(p.workplace, { hours: Math.min(p.maxHours, Math.max(0, Math.trunc(Number(e.target.value) || 0))) })}
                      />
                    </td>
                    <td className="py-2">
                      <select
                        aria-label={`Effort at ${names.workplace(p.workplace)}`}
                        className="border-line rounded-sm border px-1"
                        value={r.effort}
                        onChange={(e) => update(p.workplace, { effort: e.target.value as Effort })}
                      >
                        {EFFORTS.map((ef) => (
                          <option key={ef} value={ef} title={effortNote(l, ef)}>
                            {ef}
                          </option>
                        ))}
                      </select>
                      <span className="text-muted block max-w-xs text-xs">{effortNote(l, r.effort)}</span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
        {held.length > 0 ? (
          <div className="mt-3 flex flex-wrap items-baseline gap-3 text-sm">
            <span className={`num ${over ? "text-bad" : ""}`}>
              {total} of {l.budget} h{over ? ": the day has no more hours than that" : ""}
            </span>
            <button
              type="button"
              disabled={setLabor.isPending || over}
              className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50"
              onClick={save}
            >
              Set my hours
            </button>
            {saved ? <span className="text-muted">Set. It counts from the next hour.</span> : null}
            {error ? (
              <span className="text-bad" role="alert">
                {error}
              </span>
            ) : null}
          </div>
        ) : null}
        <p className="text-muted mt-3 text-xs">
          Hours go to at most {l.effort.max_workplaces} workplaces and never beyond the budget, which recovers each day and
          shrinks with fatigue. Output per hour is the base rate times skill, effort, and the workplace&apos;s machines
          {l.output_mult !== 1 ? ` (your multiplier now: x${l.output_mult.toFixed(2)})` : ""}.
        </p>
      </section>

      <section className="grid gap-8 md:grid-cols-2">
        <div>
          <h3 className="text-lg">{t("compensation")}</h3>
          <div className="mt-2" data-testid="payslips">
            <Ledger
              rows={slips.data ? payslipRows(slips.data.payslips as unknown as EventRef[], name) : []}
              empty="No payslip yet. The first comes at the end of the day."
            />
          </div>
        </div>
        <div>
          <h3 className="text-lg">Skill</h3>
          {l.skills.length === 0 ? (
            <p className="text-muted mt-2 text-sm">Nothing yet. Skill grows with hours worked in a job family and fades when unused.</p>
          ) : (
            <table className="mt-2 w-full text-sm" data-testid="skills">
              <thead className="text-muted text-left text-xs uppercase tracking-wide">
                <tr>
                  <th className="py-1 font-normal">Family</th>
                  <th className="py-1 text-right font-normal">Level</th>
                  <th className="py-1 text-right font-normal">Output</th>
                  <th className="py-1 text-right font-normal">Hours</th>
                </tr>
              </thead>
              <tbody>
                {l.skills.map((s) => (
                  <tr key={s.family} className="rule">
                    <td className="py-1 pr-3">{s.family}</td>
                    <td className="num py-1 text-right">{s.level.toFixed(0)}</td>
                    <td className="num py-1 text-right">
                      <Num value={`x${(1 + s.level / 100).toFixed(2)}`} />
                    </td>
                    <td className="num py-1 text-right">{s.hours.toFixed(0)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </section>
    </div>
  );
}
