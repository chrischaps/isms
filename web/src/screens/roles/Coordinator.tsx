// The Coordinator workspace (GDD 6.2 Governance, 8.2; TDD 4 roles/; S2.8):
// the three powers of the Commune's elected office on one screen. The Plan
// editor sets a target per workplace beside what each made yesterday and so
// far today; the land shows every slot and opens or closes a workplace of
// the collective at the Materials cost, against what the Store holds; the
// rationing rule is moved from here alone, through the assembly's builder.
// Mounted when the offices list the caller as a holder: anyone else gets the
// refusal page, which names who does sit and where to stand. The check is
// the pattern every later role workspace mounts on.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useOffices, type OfficeView } from "../../api/assembly";
import { ApiError, type CapabilitiesView } from "../../api/client";
import { useStore } from "../../api/commons";
import { useCloseWorkplace, useOpenWorkplace, usePublishPlan, usePublishedPlan, type PlanTargetView, type PublishedPlanView } from "../../api/coordinator";
import { useCapabilities, useHome, useLexicon } from "../../api/hooks";
import { ruleText } from "../../lib/draws";
import { useNames, type Names } from "../../lib/names";
import { fieldName } from "../../lib/policy";
import { dayOf, whenOfTick } from "../../lib/when";
import { BallotBuilder } from "./BallotBuilder";

export const OFFICE = "coordinator";

/** Every workplace kind the land knows: the limited ones by their slots, the rest unlimited. */
export function kindsOfLand(plan: Pick<PublishedPlanView, "slots" | "unlimited_kinds">): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const k of [...plan.slots.map((s) => s.kind as string), ...(plan.unlimited_kinds as string[])]) {
    if (!seen.has(k)) {
      seen.add(k);
      out.push(k);
    }
  }
  return out;
}

/** The targets to publish: every row with a number in it, in units a day. */
export function targetsToPublish(edits: Record<number, string>): Record<string, number> {
  const out: Record<string, number> = {};
  for (const [wid, v] of Object.entries(edits)) {
    if (v.trim() === "") continue;
    const n = Number(v);
    if (Number.isFinite(n) && n >= 0) out[wid] = n;
  }
  return out;
}

function fulfilment(t: PlanTargetView): string {
  if (t.last_fulfillment == null) return "";
  return ` (${(t.last_fulfillment * 100).toFixed(0)}% of the target then)`;
}

function PlanEditor({ id, plan, names }: { id: number; plan: PublishedPlanView; names: Names }) {
  const publish = usePublishPlan(id);
  const [edits, setEdits] = useState<Record<number, string> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [published, setPublished] = useState(false);
  const rows = plan.targets.slice().sort((a, b) => Number(b.collective) - Number(a.collective) || a.workplace - b.workplace);
  const current: Record<number, string> =
    edits ?? Object.fromEntries(rows.map((t) => [t.workplace, t.target == null ? "" : String(t.target)]));
  const set = (wid: number, v: string) => setEdits({ ...current, [wid]: v });
  const submit = () => {
    setError(null);
    setPublished(false);
    publish.mutate(targetsToPublish(current), {
      onSuccess: () => {
        setPublished(true);
        setEdits(null);
      },
      onError: (e) => setError(e.message),
    });
  };
  return (
    <section className="flex flex-col gap-3" data-testid="plan-editor">
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-lg">The Plan</h3>
        <span className="text-muted text-sm" data-testid="plan-signature">
          {plan.published_cycle != null && plan.published_tick != null
            ? `Last published ${whenOfTick(plan.published_tick, plan.clock.ticks_per_cycle)}${plan.published_by != null ? ` by ${names.citizen(plan.published_by)}` : ""}`
            : "No Plan has been published this epoch."}
        </span>
      </header>
      <p className="text-muted max-w-prose text-xs">
        {plan.advisory
          ? "Advisory: a target here pays no bonus and moves no rule. It is the number the Ledger of Contribution is read against, and everyone sees it on their Work screen."
          : "The Committee's targets."}{" "}
        Blank rows keep the target they have.
      </p>
      <table className="w-full text-sm">
        <thead className="text-muted text-left text-xs uppercase tracking-wide">
          <tr>
            <th className="py-1 font-normal">Workplace</th>
            <th className="py-1 text-right font-normal">Working</th>
            <th className="py-1 text-right font-normal">Yesterday</th>
            <th className="py-1 text-right font-normal">Today so far</th>
            <th className="py-1 text-right font-normal">Target a day</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr key={t.workplace} className={`rule align-top ${t.collective ? "" : "text-muted"}`} data-testid={`target-${t.workplace}`}>
              <td className="py-1 pr-3">
                {names.workplace(t.workplace)}
                {!t.collective ? <span className="block text-xs">not the collective&apos;s; the target is a request</span> : null}
              </td>
              <td className="num py-1 pr-3 text-right">{t.workers}</td>
              <td className="num py-1 pr-3 text-right">
                {t.last_cycle_output.toFixed(0)}
                <span className="text-muted text-xs">{fulfilment(t)}</span>
              </td>
              <td className="num py-1 pr-3 text-right">{t.cycle_output.toFixed(0)}</td>
              <td className="py-1 text-right">
                <input
                  type="number"
                  inputMode="decimal"
                  min={0}
                  step={1}
                  aria-label={`Target at ${names.workplace(t.workplace)}`}
                  className="border-line num w-24 rounded-sm border px-1 text-right"
                  value={current[t.workplace] ?? ""}
                  onChange={(e) => set(t.workplace, e.target.value)}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="flex flex-wrap items-baseline gap-3 text-sm">
        <button type="button" disabled={publish.isPending} className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50" onClick={submit} data-testid="publish-plan">
          Publish the Plan
        </button>
        {published ? <span className="text-muted">Published. It is on the record and on every Work screen.</span> : null}
        {error ? (
          <span className="text-bad" role="alert">
            {error}
          </span>
        ) : null}
      </div>
    </section>
  );
}

function Land({ id, plan, names }: { id: number; plan: PublishedPlanView; names: Names }) {
  const open = useOpenWorkplace(id);
  const close = useCloseWorkplace(id);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [closing, setClosing] = useState<number | null>(null);
  const kinds = kindsOfLand(plan);
  const canPay = plan.store_materials >= plan.founding_materials;
  const collective = plan.targets.filter((t) => t.collective);
  const fail = (e: Error) => setError(e.message);
  return (
    <section className="flex flex-col gap-3" data-testid="land">
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-lg">The land</h3>
        <span className={`num text-sm ${canPay ? "text-muted" : "text-warn"}`} data-testid="materials">
          A workplace costs {plan.founding_materials} Materials; the Store holds {plan.store_materials}.
        </span>
      </header>
      <p className="text-muted max-w-prose text-xs">
        Opening is your word, not a vote: the Materials leave the Store the same hour. Closing sends its workers home this hour, its machines back to the
        Store, and frees the slot. Both go on the record with your name.
      </p>
      <table className="w-full text-sm" data-testid="slots">
        <thead className="text-muted text-left text-xs uppercase tracking-wide">
          <tr>
            <th className="py-1 font-normal">Kind</th>
            <th className="py-1 text-right font-normal">Open</th>
            <th className="py-1 text-right font-normal">Slots</th>
            <th className="py-1 font-normal" />
          </tr>
        </thead>
        <tbody>
          {kinds.map((k) => {
            const slots = plan.slots.filter((s) => (s.kind as string) === k);
            const limited = slots.length > 0;
            const free = slots.filter((s) => s.workplace == null).length;
            const openHere = plan.targets.filter((t) => (t.kind as string) === k).length;
            const blocked = (limited && free === 0) || !canPay;
            return (
              <tr key={k} className="rule" data-testid={`land-${k}`}>
                <td className="py-1 pr-3">{fieldName(k)}</td>
                <td className="num py-1 pr-3 text-right">{openHere}</td>
                <td className="num py-1 pr-3 text-right">{limited ? `${free} of ${slots.length} free` : <span className="text-muted">no limit</span>}</td>
                <td className="py-1 text-right">
                  <button
                    type="button"
                    disabled={open.isPending || blocked}
                    title={!canPay ? "The Store holds too few Materials." : limited && free === 0 ? "No free slot of this kind." : undefined}
                    className="border-line rounded-sm border px-2 py-0.5 text-xs disabled:opacity-50"
                    onClick={() => {
                      setError(null);
                      setNote(null);
                      open.mutate({ kind: k }, { onSuccess: () => setNote(`Opened a ${fieldName(k)}; ${plan.founding_materials} Materials left the Store.`), onError: fail });
                    }}
                  >
                    Open a {fieldName(k)}
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>

      <h4 className="text-base">The collective&apos;s workplaces</h4>
      {collective.length === 0 ? (
        <p className="text-muted text-sm">The collective has no workplace open.</p>
      ) : (
        <ul className="flex flex-col gap-1 text-sm" data-testid="collective-workplaces">
          {collective.map((t) => (
            <li key={t.workplace} className="flex flex-wrap items-baseline justify-between gap-2" data-testid={`workplace-${t.workplace}`}>
              <span>
                {names.workplace(t.workplace)}
                <span className="text-muted text-xs">
                  {t.slot != null ? ` · slot ${t.slot}` : ""} · {t.machines} machines · {t.workers} working
                </span>
              </span>
              {closing === t.workplace ? (
                <span className="flex items-baseline gap-2">
                  <button
                    type="button"
                    disabled={close.isPending}
                    className="bg-bad text-paper rounded-sm px-2 py-0.5 text-xs disabled:opacity-50"
                    onClick={() => {
                      setError(null);
                      setNote(null);
                      close.mutate(t.workplace, {
                        onSuccess: () => {
                          setClosing(null);
                          setNote(`Closed the ${names.workplace(t.workplace)}.`);
                        },
                        onError: fail,
                      });
                    }}
                  >
                    Close it{t.workers > 0 ? `: ${t.workers} ${t.workers === 1 ? "worker loses" : "workers lose"} this hour` : ""}
                  </button>
                  <button type="button" className="text-muted text-xs underline" onClick={() => setClosing(null)}>
                    keep it
                  </button>
                </span>
              ) : (
                <button type="button" className="border-line rounded-sm border px-2 py-0.5 text-xs" onClick={() => setClosing(t.workplace)}>
                  Close
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {note ? (
        <p className="text-muted text-sm" data-testid="land-note">
          {note}
        </p>
      ) : null}
      {error ? (
        <p className="text-bad text-sm" role="alert">
          {error}
        </p>
      ) : null}
    </section>
  );
}

function Rationing({ id, caps, office }: { id: number; caps: CapabilitiesView; office: OfficeView }) {
  const store = useStore(id, caps.common_store);
  return (
    <section className="flex flex-col gap-3" data-testid="rationing">
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-lg">The rationing rule</h3>
        {store.data ? (
          <span className="text-muted text-sm" data-testid="rule-in-force">
            In force: {fieldName(store.data.rule)}
          </span>
        ) : null}
      </header>
      {store.data ? <p className="text-muted max-w-prose text-xs">{ruleText(store.data.rule)}</p> : null}
      <p className="text-muted max-w-prose text-xs">
        Only a coordinator may move the rule; the assembly decides it like any other motion, at the end of the day. Take the argument to the floor on the{" "}
        <Link to="/s/$id/assembly" params={{ id: String(id) }} className="underline">
          assembly
        </Link>{" "}
        screen.
      </p>
      <BallotBuilder id={id} caps={caps} citizens={[]} offices={[office]} only={["policy_change"]} fields={["rationing"]} />
    </section>
  );
}

export function Coordinator({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const citizen = home.data !== undefined;
  const governed = caps.data !== undefined && (caps.data as CapabilitiesView).governance !== "none";
  const offices = useOffices(id, citizen && governed);
  const office = offices.data?.offices.find((o) => o.kind === OFFICE);
  const holder = office?.i_hold === true;
  const plan = usePublishedPlan(id, citizen && holder);
  const names = useNames(id, home.data?.citizen.id, citizen);

  if (caps.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error) return <p className="text-bad">Could not load: {String(caps.error)}</p>;
  // No governance, no office to sit in: said before anyone is asked to join.
  if (!governed) return <p className="text-muted">There is no {OFFICE} in this society.</p>;
  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }} className="underline">
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  if (offices.isPending) return <p className="text-muted">Loading.</p>;
  if (offices.error) return <p className="text-bad">Could not load: {String(offices.error)}</p>;
  if (!office) return <p className="text-muted">This society has no {OFFICE}s.</p>;
  const me = home.data!.citizen.id;
  const fellows = office.holders.filter((h) => h.citizen !== me);

  // The refusal page (the done gate): who does sit, and where to stand.
  if (!holder) {
    return (
      <section className="flex flex-col gap-3" data-testid="not-a-coordinator">
        <h2 className="text-2xl capitalize">{fieldName(OFFICE)}</h2>
        <p>You do not sit as {fieldName(OFFICE)}, so this workspace is not yours.</p>
        <p className="text-muted text-sm">
          {office.holders.length === 0
            ? "Nobody sits; the seats are open."
            : `Sitting now: ${office.holders.map((h) => `${h.handle} through ${dayOf(h.term_ends_cycle)}`).join(", ")}.`}{" "}
          {office.election ? "An election is open: " : "The next election opens when a seat empties: "}
          stand on the{" "}
          <Link to="/s/$id/assembly" params={{ id: String(id) }} className="underline">
            {t("assembly")}
          </Link>{" "}
          screen.
        </p>
      </section>
    );
  }
  if (plan.isPending) return <p className="text-muted">Loading.</p>;
  if (plan.error) return <p className="text-bad">Could not load: {String(plan.error)}</p>;
  const p = plan.data!;
  const myTerm = office.holders.find((h) => h.citizen === me);
  const c = caps.data as CapabilitiesView;

  return (
    <div className="flex flex-col gap-10" data-testid="coordinator">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl capitalize">{fieldName(OFFICE)}</h2>
        <p className="text-muted text-sm" data-testid="fellows">
          {myTerm ? `Your term runs through ${dayOf(myTerm.term_ends_cycle)}` : "You sit"}
          {fellows.length > 0 ? `, with ${fellows.map((h) => h.handle).join(" and ")}` : office.seats > 1 ? `; ${office.seats - 1} ${office.seats - 1 === 1 ? "seat is" : "seats are"} empty` : ""}
          . Recall is by {fieldName(office.recall)}, any day.
        </p>
      </header>
      <PlanEditor id={id} plan={p} names={names} />
      <Land id={id} plan={p} names={names} />
      <Rationing id={id} caps={c} office={office} />
    </div>
  );
}
