// The operator's room (S1.13c; docs/style.md §10 Operator): every society as
// a card with its clock, its people and the hold on it. Pause stops the
// scheduler; step resolves one tick while held; resume releases with no
// catch-up; the tick length can change; an epoch can be ended by hand, and
// an ended society can start its next epoch (the roster stays, material
// state resets). Ending asks twice; it cannot be undone.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useMe } from "../api/hooks";
import { useAdminAct, useAdminSocieties, useAdminTickSeconds } from "../api/admin";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack } from "../components/Card";
import { Countdown } from "../components/Countdown";
import { FactList } from "../components/FactList";
import { Field, Input } from "../components/Field";
import { PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";

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
        Operators only. <Link to="/">Societies</Link>
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
    <div>
      <PageHeader
        title="Operator"
        meta={
          <>
            <Link to="/">Societies</Link>
            <Link to="/profile">Profile</Link>
          </>
        }
      />
      <p className="text-muted mt-0 mb-4 text-sm">
        A held clock resolves nothing until released; releasing puts the next tick one tick length from now, with no catch-up. Everything here is logged with your
        account.
      </p>
      {error ?? societies.error ? (
        <p className="text-crit mt-0 mb-4 text-sm" role="alert">
          {error ?? String(societies.error)}
        </p>
      ) : null}
      <Stack testId="admin-societies">
        {(societies.data ?? []).map((v) => {
          const s = v.summary;
          const ended = s.clock.epoch_ended;
          const state = ended ? "epoch ended" : v.paused ? "held" : "running";
          const tone = ended ? "neutral" : v.paused ? "attn" : "good";
          return (
            <Card
              key={s.id}
              title={
                <Link to="/s/$id" params={{ id: String(s.id) }} className="text-ink hover:text-accent">
                  {s.display}
                </Link>
              }
              icon="globe"
              testId={`admin-society-${s.id}`}
              aside={
                <Pill tone={tone} testId="admin-state">
                  {state}
                </Pill>
              }
              subtitle={
                <>
                  #{s.id} {s.name} · {s.preset}
                </>
              }
            >
              <div className="grid gap-4 md:grid-cols-[1fr_auto]">
                <FactList
                  items={[
                    {
                      key: "clock",
                      label: "Clock",
                      value: (
                        <span data-testid="admin-clock">
                          epoch {s.clock.epoch} · day {s.clock.cycle} · tick {s.clock.tick}/{s.clock.ticks_per_cycle}
                        </span>
                      ),
                      gloss:
                        !ended && !v.paused ? (
                          <Countdown at={s.next_tick_at} label="next tick" />
                        ) : ended && v.statements_close_at ? (
                          <Countdown at={v.statements_close_at} label="statements close in" />
                        ) : (
                          "no tick due"
                        ),
                    },
                    { key: "people", label: "People", value: `${s.active_humans} of ${s.population}`, gloss: "citizens" },
                  ]}
                />
                <form
                  className="flex items-end gap-2"
                  onSubmit={(e) => {
                    e.preventDefault();
                    const n = Number(tick[s.id] ?? s.tick_seconds);
                    if (!Number.isFinite(n) || n < 0) return;
                    setError(null);
                    setTick.mutate({ id: s.id, tick_seconds: Math.trunc(n) }, { onError: fail });
                  }}
                >
                  <Field label="Tick length" unit="s" className="w-[9rem]">
                    <Input
                      aria-label={`Tick seconds for ${s.display}`}
                      type="number"
                      min={0}
                      value={tick[s.id] ?? String(s.tick_seconds)}
                      onChange={(e) => setTickInput({ ...tick, [s.id]: e.target.value })}
                    />
                  </Field>
                  <Button type="submit" inline>
                    Set
                  </Button>
                </form>
              </div>
              {note[s.id] ? (
                <p className="text-good mt-3 mb-0 text-sm" role="status" data-testid="admin-note">
                  {note[s.id]}
                </p>
              ) : null}
              <ButtonRow>
                {ended ? (
                  <Button variant="primary" disabled={act.isPending} onClick={() => act.mutate({ id: s.id, act: "new-epoch" }, { onError: fail })}>
                    Start epoch {s.clock.epoch + 1}
                    {v.statements_close_at ? " now" : ""}
                  </Button>
                ) : v.paused ? (
                  <>
                    <Button disabled={act.isPending} onClick={() => run(s.id, "step")}>
                      Step one tick
                    </Button>
                    <Button variant="primary" disabled={act.isPending} onClick={() => run(s.id, "resume")}>
                      Resume
                    </Button>
                  </>
                ) : (
                  <Button disabled={act.isPending} onClick={() => run(s.id, "pause")}>
                    Pause
                  </Button>
                )}
                {ended ? null : arming === s.id ? (
                  <>
                    <Button variant="danger" disabled={act.isPending} onClick={() => act.mutate({ id: s.id, act: "end-epoch" }, { onError: fail, onSettled: () => setArming(null) })}>
                      Yes, end it
                    </Button>
                    <Button variant="quiet" onClick={() => setArming(null)}>
                      keep going
                    </Button>
                  </>
                ) : (
                  <Button variant="quiet" onClick={() => setArming(s.id)}>
                    end epoch
                  </Button>
                )}
              </ButtonRow>
            </Card>
          );
        })}
      </Stack>
      {societies.data && societies.data.length === 0 ? <p className="text-muted mt-4 text-sm">No societies loaded.</p> : null}
    </div>
  );
}
