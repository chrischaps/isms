// Onboarding (GDD 9.4, "working within two minutes"): a handle, the consent
// text, the Welcome Brief in the society's voice, a first position from the
// job board, and the standing plan's defaults. Six interactions to a job.

import { useState } from "react";
import { ApiError, credits, type OfferView } from "../api/client";
import { useBoard, useLexicon, useWelcome } from "../api/hooks";
import { useAcceptOffer, useJoin, usePlan, useSetLabor, useSetPlan } from "../api/society";
import { Markdown } from "../lib/markdown";

const CONSENT_V1 =
  "Everything you do here is recorded: every trade, contract and message, direct messages " +
  "included. The records are pseudonymous and may be published for research. Your email is " +
  "never part of them.";

type Step = "handle" | "welcome" | "job" | "plan";

function offerLine(o: OfferView, orgName: (id: number) => string) {
  const e = o.body.employment as Record<string, unknown> | undefined;
  if (!e) return null;
  const pay = e.pay as Record<string, number>;
  const hourly = pay.hourly;
  const piece = pay.piece_rate;
  return {
    org: orgName(Number(e.org)),
    workplace: Number(e.workplace),
    hours: Number(e.max_hours),
    places: Number(e.places),
    pay: hourly !== undefined ? `${credits(hourly)} cr/h` : `${credits(piece ?? 0)} cr/unit`,
  };
}

export function Onboarding({ id, onDone }: { id: number; onDone: () => void }) {
  const [step, setStep] = useState<Step>("handle");
  const [handle, setHandle] = useState("");
  const [consent, setConsent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { t } = useLexicon(id);
  const welcome = useWelcome(id);
  const board = useBoard(id);
  const plan = usePlan(id);
  const join = useJoin(id);
  const accept = useAcceptOffer(id);
  const setLabor = useSetLabor(id);
  const setPlan = useSetPlan(id);

  const fail = (e: unknown) => setError(e instanceof ApiError ? e.message : String(e));

  if (step === "handle") {
    return (
      <section className="max-w-lg">
        <h2 className="text-2xl">Choose a name</h2>
        <p className="text-muted mt-2 text-sm">A pseudonym. Two to twenty-four letters, digits, _ or -.</p>
        <form
          className="mt-4 flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            setError(null);
            join.mutate(handle.trim(), { onSuccess: () => setStep("welcome"), onError: fail });
          }}
        >
          <input
            aria-label="Handle"
            value={handle}
            onChange={(e) => setHandle(e.target.value)}
            className="border-line bg-paper-2 rounded-sm border px-2 py-1 font-mono"
            autoFocus
          />
          <label className="flex items-start gap-2 text-sm">
            <input type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} className="mt-1" />
            <span>{CONSENT_V1}</span>
          </label>
          {error ? <p className="text-bad text-sm">{error}</p> : null}
          <button
            type="submit"
            disabled={!consent || handle.trim().length < 2 || join.isPending}
            className="bg-ink text-paper self-start rounded-sm px-3 py-1 disabled:opacity-50"
          >
            Enter
          </button>
        </form>
      </section>
    );
  }

  if (step === "welcome") {
    return (
      <section className="max-w-prose">
        {welcome.data ? <Markdown text={welcome.data.markdown} className="text-lg" /> : <p className="text-muted">Loading.</p>}
        <button type="button" onClick={() => setStep("job")} className="bg-ink text-paper mt-6 rounded-sm px-3 py-1">
          Find work
        </button>
      </section>
    );
  }

  if (step === "job") {
    const offers = (board.data?.offers ?? []).filter((o) => o.kind === "employment");
    const orgName = (oid: number) => `Org #${oid}`;
    return (
      <section>
        <h2 className="text-2xl">The job board</h2>
        <p className="text-muted mt-2 text-sm">
          Open {t("job").toLowerCase()}s. Take one; you can change your hours and effort any time.
        </p>
        {board.isPending ? <p className="text-muted mt-4">Loading.</p> : null}
        <ul className="mt-4 flex flex-col gap-2" data-testid="job-board">
          {offers.map((o) => {
            const line = offerLine(o, orgName);
            if (!line) return null;
            return (
              <li key={o.id} className="rule flex flex-wrap items-baseline justify-between gap-2 pt-2">
                <span>
                  <span className="font-mono text-sm">#{o.id}</span> workplace {line.workplace},{" "}
                  <span className="num">{line.pay}</span>, up to {line.hours} h a cycle, {line.places} open
                </span>
                <button
                  type="button"
                  disabled={accept.isPending}
                  className="border-line rounded-sm border px-2 py-0.5 text-sm"
                  onClick={() => {
                    setError(null);
                    accept.mutate(o.id, {
                      onSuccess: () =>
                        setLabor.mutate(
                          [{ workplace: line.workplace, hours: line.hours, effort: "normal" }],
                          { onSuccess: () => setStep("plan"), onError: fail },
                        ),
                      onError: fail,
                    });
                  }}
                >
                  Take this {t("job").toLowerCase()}
                </button>
              </li>
            );
          })}
        </ul>
        {offers.length === 0 && !board.isPending ? (
          <p className="text-muted mt-4 text-sm">Nobody is hiring this tick. Check back after the next one.</p>
        ) : null}
        {error ? <p className="text-bad mt-3 text-sm">{error}</p> : null}
        <button type="button" onClick={() => setStep("plan")} className="text-muted mt-6 text-sm">
          Skip for now
        </button>
      </section>
    );
  }

  const p = (plan.data?.plan ?? {}) as Record<string, unknown>;
  return (
    <section className="max-w-lg">
      <h2 className="text-2xl">Your {t("plan").toLowerCase()}</h2>
      <p className="text-muted mt-2 text-sm">
        It runs every tick whether you are here or not. These are the defaults; change them on the {t("plan")} screen.
      </p>
      <dl className="mt-4 grid grid-cols-2 gap-y-1 text-sm">
        <dt className="text-muted">Keep Food at least</dt>
        <dd className="num">{String(p.keep_food_at_least ?? "24")}</dd>
        <dt className="text-muted">Food price ceiling</dt>
        <dd className="num">{p.max_food_price == null ? "last price x 1.25" : credits(Number(p.max_food_price))}</dd>
        <dt className="text-muted">Keep balance at least</dt>
        <dd className="num">{credits(Number(p.keep_balance_at_least ?? 0))} cr</dd>
      </dl>
      {error ? <p className="text-bad mt-3 text-sm">{error}</p> : null}
      <button
        type="button"
        disabled={setPlan.isPending || !plan.data}
        className="bg-ink text-paper mt-6 rounded-sm px-3 py-1 disabled:opacity-50"
        onClick={() => setPlan.mutate(p, { onSuccess: onDone, onError: fail })}
      >
        Keep these and go home
      </button>
    </section>
  );
}
