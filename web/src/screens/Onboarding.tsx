// Onboarding (GDD 9.4, "working within two minutes"; docs/style.md §10
// Onboarding): a handle, the consent text, the Welcome Brief in the
// society's voice, a first position from the job board, and the standing
// plan's defaults. Each step is one card with one primary button; six
// interactions to a job. The brief is the one place the society speaks
// before the shell does, so it is set in the body face at text-lg.

import { useState, type ReactNode } from "react";
import { ApiError, credits, type OfferView } from "../api/client";
import { useBoard, useLexicon, useWelcome } from "../api/hooks";
import { useAcceptOffer, useJoin, usePlan, useSetLabor, useSetPlan } from "../api/society";
import { Button, ButtonRow } from "../components/Button";
import { Card, Tile } from "../components/Card";
import { FactList } from "../components/FactList";
import { Field, Input } from "../components/Field";
import { PageHeader } from "../components/PageHeader";
import { Markdown } from "../lib/markdown";
import { useNames } from "../lib/names";

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

/** The step's frame: one narrow column, the title as the page's, the card beneath. */
function StepFrame({ title, subtitle, wide, children }: { title: string; subtitle?: string; wide?: boolean; children: ReactNode }) {
  return (
    <div className={wide ? "mx-auto max-w-2xl" : "mx-auto max-w-lg"}>
      <PageHeader title={title} />
      <Card subtitle={subtitle}>{children}</Card>
    </div>
  );
}

export function Onboarding({ id, onDone }: { id: number; onDone: () => void }) {
  const [step, setStep] = useState<Step>("handle");
  const [handle, setHandle] = useState("");
  const [consent, setConsent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { t } = useLexicon(id);
  const welcome = useWelcome(id);
  const board = useBoard(id);
  const names = useNames(id, undefined, step !== "handle");
  const plan = usePlan(id);
  const join = useJoin(id);
  const accept = useAcceptOffer(id);
  const setLabor = useSetLabor(id);
  const setPlan = useSetPlan(id);

  const fail = (e: unknown) => setError(e instanceof ApiError ? e.message : String(e));
  const alert = error ? (
    <p className="text-crit m-0 text-sm" role="alert">
      {error}
    </p>
  ) : null;

  if (step === "handle") {
    return (
      <StepFrame title="Choose a name" subtitle="A pseudonym. Two to twenty-four letters, digits, _ or -.">
        <form
          className="grid gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            setError(null);
            join.mutate(handle.trim(), { onSuccess: () => setStep("welcome"), onError: fail });
          }}
        >
          <Field label="Handle" className="max-w-none">
            <Input aria-label="Handle" className="font-mono" value={handle} onChange={(e) => setHandle(e.target.value)} autoFocus />
          </Field>
          <label className="flex items-start gap-2.5">
            <input type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} className="mt-1.5" />
            <span className="text-sm">{CONSENT_V1}</span>
          </label>
          {alert}
          <Button type="submit" variant="primary" disabled={!consent || handle.trim().length < 2 || join.isPending}>
            Enter
          </Button>
        </form>
      </StepFrame>
    );
  }

  if (step === "welcome") {
    return (
      <div className="mx-auto max-w-2xl">
        <Card>
          {welcome.data ? <Markdown text={welcome.data.markdown} className="text-lg" /> : <p className="text-muted m-0">Loading.</p>}
          <ButtonRow>
            <Button variant="primary" onClick={() => setStep("job")}>
              Find work
            </Button>
          </ButtonRow>
        </Card>
      </div>
    );
  }

  if (step === "job") {
    const offers = (board.data?.offers ?? []).filter((o) => o.kind === "employment");
    return (
      <StepFrame title="The job board" subtitle={`Open ${t("job").toLowerCase()}s. Take one; you can change your hours and effort any time.`} wide>
        {board.isPending ? <p className="text-muted m-0">Loading.</p> : null}
        <ul className="m-0 grid list-none gap-3 p-0" data-testid="job-board">
          {offers.map((o) => {
            const line = offerLine(o, names.org);
            if (!line) return null;
            return (
              <li key={o.id}>
                <Tile className="grid gap-2 sm:flex sm:items-center sm:justify-between">
                  <span>
                    <b>{names.workplace(line.workplace)}</b>
                    <span className="text-muted block text-sm">
                      <span className="text-ink font-display font-bold">{line.pay}</span> · up to {line.hours} h a day · {line.places} open
                    </span>
                  </span>
                  <Button
                    disabled={accept.isPending}
                    onClick={() => {
                      setError(null);
                      accept.mutate(o.id, {
                        onSuccess: () =>
                          setLabor.mutate([{ workplace: line.workplace, hours: line.hours, effort: "normal" }], { onSuccess: () => setStep("plan"), onError: fail }),
                        onError: fail,
                      });
                    }}
                  >
                    Take this {t("job").toLowerCase()}
                  </Button>
                </Tile>
              </li>
            );
          })}
        </ul>
        {offers.length === 0 && !board.isPending ? <p className="text-muted m-0">Nobody is hiring this hour. Check back after the next one.</p> : null}
        {error ? <div className="mt-3">{alert}</div> : null}
        <ButtonRow>
          <Button variant="quiet" onClick={() => setStep("plan")}>
            Skip for now
          </Button>
        </ButtonRow>
      </StepFrame>
    );
  }

  const p = (plan.data?.plan ?? {}) as Record<string, unknown>;
  return (
    <StepFrame title={`Your ${t("plan").toLowerCase()}`} subtitle={`It runs every hour whether you are here or not. These are the defaults; change them on the ${t("plan")} screen.`}>
      <FactList
        items={[
          { key: "food", label: "Keep Food at least", value: String(p.keep_food_at_least ?? "24"), gloss: "in the pantry" },
          { key: "ceiling", label: "Food price ceiling", value: p.max_food_price == null ? "last price × 1.25" : `${credits(Number(p.max_food_price))} cr` },
          { key: "balance", label: "Keep balance at least", value: `${credits(Number(p.keep_balance_at_least ?? 0))} cr` },
        ]}
      />
      {error ? <div className="mt-3">{alert}</div> : null}
      <ButtonRow>
        <Button variant="primary" disabled={setPlan.isPending || !plan.data} onClick={() => setPlan.mutate(p, { onSuccess: onDone, onError: fail })}>
          Keep these and go home
        </Button>
      </ButtonRow>
    </StepFrame>
  );
}
