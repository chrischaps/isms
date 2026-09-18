// The operator's room (S1.13c): every society with its clock, and the hold
// on it. Pause stops the scheduler; step resolves one tick while held;
// resume releases with no catch-up; the tick length can change; an epoch
// can be ended by hand, and an ended society can start its next epoch (the
// roster stays, material state resets). Ending asks twice; it cannot be undone.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useMe } from "../api/hooks";
import { useAdminAct, useAdminSocieties, useAdminTickSeconds } from "../api/admin";
import { Countdown } from "../components/Countdown";

export function Admin() {
  const me = useMe();
  const operator = me.data?.account.operator === true;
  const societies = useAdminSocieties(operator);
  const act = useAdminAct();
  const setTick = useAdminTickSeconds();
  const [tick, setTickInput] = useState<Record<number, string>>({});
  const [arming, setArming] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<Record<number, string>>({});

  if (me.isPending) return <p className="text-muted">Loading.</p>;
  if (!operator) {
    return (
      <p className="text-muted">
        Operators only.{" "}
        <Link to="/" className="underline">
          Societies
        </Link>
      </p>
    );
  }
  const fail = (e: Error) => setError(e.message);
  // Each action answers with the society's new clock; say what it did, since a step changes nothing else on this page.
  const run = (sid: number, what: "pause" | "resume" | "step") => {
    setError(null);
    act.mutate(
      { id: sid, act: what },
      {
        onError: fail,
        onSuccess: (r) => {
          const k = r.summary.clock;
          const left = k.ticks_per_cycle - k.tick;
          const at = `day ${k.cycle}, tick ${k.tick}/${k.ticks_per_cycle}`;
          const payday = left === 0 ? "the next tick closes the day (payday)" : `${left + 1} more ticks close the day (payday)`;
          setNote({
            ...note,
            [sid]:
              what === "step"
                ? `Stepped: one tick resolved; the clock stands at ${at}. ${payday}.`
                : what === "pause"
                  ? `Held at ${at}. Nothing resolves until you step or resume.`
                  : `Resumed at ${at}; the next tick comes one tick length from now.`,
          });
        },
      },
    );
  };

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h1 className="text-2xl">Operator</h1>
        <nav className="flex gap-4 text-sm">
          <Link to="/" className="text-muted underline">
            Societies
          </Link>
          <Link to="/profile" className="text-muted underline">
            Profile
          </Link>
        </nav>
      </header>
      <p className="text-muted text-sm">
        A held clock resolves nothing until released; releasing puts the next tick one tick length from now, with no catch-up. Everything here is logged with your
        account.
      </p>
      {error ?? societies.error ? (
        <p className="text-bad text-sm" role="alert">
          {error ?? String(societies.error)}
        </p>
      ) : null}
      <table className="w-full text-sm" data-testid="admin-societies">
        <thead className="text-muted text-left text-xs uppercase tracking-wide">
          <tr>
            <th className="py-1 font-normal">Society</th>
            <th className="py-1 font-normal">Clock</th>
            <th className="py-1 font-normal">People</th>
            <th className="py-1 font-normal">State</th>
            <th className="py-1 font-normal">Tick length</th>
            <th className="py-1 font-normal" />
          </tr>
        </thead>
        <tbody>
          {(societies.data ?? []).map((v) => {
            const s = v.summary;
            const ended = s.clock.epoch_ended;
            const state = ended ? "epoch ended" : v.paused ? "held" : "running";
            return (
              <tr key={s.id} className="rule align-top" data-testid={`admin-society-${s.id}`}>
                <td className="py-2 pr-2">
                  <Link to="/s/$id" params={{ id: String(s.id) }} className="underline">
                    {s.display}
                  </Link>
                  <span className="text-muted block text-xs">
                    #{s.id} {s.name} · {s.preset}
                  </span>
                </td>
                <td className="num py-2 pr-2" data-testid="admin-clock">
                  epoch {s.clock.epoch} · day {s.clock.cycle} · tick {s.clock.tick}/{s.clock.ticks_per_cycle}
                  <span className="text-muted block text-xs">
                    {!ended && !v.paused ? (
                      <Countdown at={s.next_tick_at} label="next tick" />
                    ) : ended && v.statements_close_at ? (
                      <Countdown at={v.statements_close_at} label="statements close in" />
                    ) : (
                      "no tick due"
                    )}
                  </span>
                  {note[s.id] ? (
                    <span className="text-ink block max-w-xs text-xs" role="status" data-testid="admin-note">
                      {note[s.id]}
                    </span>
                  ) : null}
                </td>
                <td className="num py-2 pr-2">
                  {s.active_humans} of {s.population}
                </td>
                <td className="py-2 pr-2" data-testid="admin-state">
                  <span className={v.paused ? "text-warn" : ended ? "text-muted" : "text-good"}>{state}</span>
                </td>
                <td className="py-2 pr-2">
                  <form
                    className="flex items-center gap-1"
                    onSubmit={(e) => {
                      e.preventDefault();
                      const n = Number(tick[s.id] ?? s.tick_seconds);
                      if (!Number.isFinite(n) || n < 0) return;
                      setError(null);
                      setTick.mutate({ id: s.id, tick_seconds: Math.trunc(n) }, { onError: fail });
                    }}
                  >
                    <input
                      aria-label={`Tick seconds for ${s.display}`}
                      type="number"
                      min={0}
                      className="border-line num w-16 rounded-sm border px-1"
                      value={tick[s.id] ?? String(s.tick_seconds)}
                      onChange={(e) => setTickInput({ ...tick, [s.id]: e.target.value })}
                    />
                    <span className="text-muted text-xs">s</span>
                    <button type="submit" className="border-line rounded-sm border px-2 py-0.5 text-xs">
                      Set
                    </button>
                  </form>
                </td>
                <td className="py-2">
                  <div className="flex flex-wrap gap-2">
                    {ended ? (
                      <button type="button" disabled={act.isPending} className="bg-ink text-paper rounded-sm px-2 py-0.5 text-xs" onClick={() => act.mutate({ id: s.id, act: "new-epoch" }, { onError: fail })}>
                        Start epoch {s.clock.epoch + 1}
                        {v.statements_close_at ? " now" : ""}
                      </button>
                    ) : v.paused ? (
                      <>
                        <button type="button" disabled={act.isPending} className="border-line rounded-sm border px-2 py-0.5 text-xs" onClick={() => run(s.id, "step")}>
                          Step one tick
                        </button>
                        <button type="button" disabled={act.isPending} className="bg-ink text-paper rounded-sm px-2 py-0.5 text-xs" onClick={() => run(s.id, "resume")}>
                          Resume
                        </button>
                      </>
                    ) : (
                      <button type="button" disabled={act.isPending} className="border-line rounded-sm border px-2 py-0.5 text-xs" onClick={() => run(s.id, "pause")}>
                        Pause
                      </button>
                    )}
                    {ended ? null : arming === s.id ? (
                      <>
                        <button type="button" disabled={act.isPending} className="bg-bad text-paper rounded-sm px-2 py-0.5 text-xs" onClick={() => act.mutate({ id: s.id, act: "end-epoch" }, { onError: fail, onSettled: () => setArming(null) })}>
                          Yes, end it
                        </button>
                        <button type="button" className="text-muted text-xs underline" onClick={() => setArming(null)}>
                          keep going
                        </button>
                      </>
                    ) : (
                      <button type="button" className="text-muted text-xs underline" onClick={() => setArming(s.id)}>
                        end epoch
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {societies.data && societies.data.length === 0 ? <p className="text-muted text-sm">No societies loaded.</p> : null}
    </div>
  );
}
