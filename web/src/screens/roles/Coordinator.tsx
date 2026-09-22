// The Coordinator workspace (GDD 6.2 Governance, 8.2; TDD 4 roles/; S2.8;
// docs/style.md §10 by analogy): the three powers of the Commune's elected
// office on one screen. The Verdict says the seat, the term, and whether a
// Plan stands. The Plan editor sets a target per workplace beside what each
// made yesterday and so far today; the land shows every slot and opens or
// closes a workplace of the collective at the Materials cost, against what
// the Store holds; the rationing rule is moved from here alone, through the
// assembly's builder. Mounted when the offices list the caller as a holder:
// anyone else gets the refusal page, which names who does sit and where to
// stand. The check is the pattern every later role workspace mounts on.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useOffices, type OfficeView } from "../../api/assembly";
import { ApiError, type CapabilitiesView } from "../../api/client";
import { useStore } from "../../api/commons";
import { useCloseWorkplace, useOpenWorkplace, usePublishPlan, usePublishedPlan, type PlanTargetView, type PublishedPlanView } from "../../api/coordinator";
import { useCapabilities, useHome, useLexicon } from "../../api/hooks";
import { Button, ButtonRow } from "../../components/Button";
import { Card, Stack, Tile } from "../../components/Card";
import { BareInput } from "../../components/Field";
import { Figure, Figures } from "../../components/Figure";
import { TD, TD_NUM, TH, TH_NUM } from "../../components/Ledger";
import { PageHeader } from "../../components/PageHeader";
import { Pill } from "../../components/Pill";
import { Verdict } from "../../components/Verdict";
import { ruleText } from "../../lib/draws";
import { useNames, type Names } from "../../lib/names";
import { fieldName } from "../../lib/policy";
import { coordinatorVerdict } from "../../lib/verdict";
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
  return ` (${(t.last_fulfillment * 100).toFixed(0)} % of the target then)`;
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
    <Card
      title="The Plan"
      icon="plan"
      testId="plan-editor"
      subtitle={
        <>
          <span data-testid="plan-signature">
            {plan.published_cycle != null && plan.published_tick != null
              ? `Last published ${whenOfTick(plan.published_tick, plan.clock.ticks_per_cycle)}${plan.published_by != null ? ` by ${names.citizen(plan.published_by)}` : ""}.`
              : "No Plan has been published this epoch."}
          </span>{" "}
          {plan.advisory
            ? "Advisory: a target here pays no bonus and moves no rule. It is the number the Ledger of Contribution is read against, and everyone sees it on their Work screen."
            : "The Committee's targets."}{" "}
          Blank rows keep the target they have.
        </>
      }
    >
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr>
            <th className={TH}>Workplace</th>
            <th className={`${TH_NUM} hidden md:table-cell`}>Working</th>
            <th className={`${TH_NUM} hidden md:table-cell`}>Yesterday</th>
            <th className={TH_NUM}>Today so far</th>
            <th className={TH_NUM}>Target a day</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr key={t.workplace} className={`hover:bg-surface-2 ${t.collective ? "" : "text-muted"}`} data-testid={`target-${t.workplace}`}>
              <td className={TD}>
                {names.workplace(t.workplace)}
                {!t.collective ? <span className="text-muted block text-sm">not the collective's; the target is a request</span> : null}
                <span className="text-muted block text-sm md:hidden">
                  {t.workers} working · {t.last_cycle_output.toFixed(0)} yesterday{fulfilment(t)}
                </span>
              </td>
              <td className={`${TD_NUM} hidden md:table-cell`}>{t.workers}</td>
              <td className={`${TD_NUM} hidden md:table-cell`}>
                {t.last_cycle_output.toFixed(0)}
                <span className="text-muted text-sm">{fulfilment(t)}</span>
              </td>
              <td className={TD_NUM}>{t.cycle_output.toFixed(0)}</td>
              <td className={`${TD} text-right`}>
                <BareInput
                  type="number"
                  inputMode="decimal"
                  min={0}
                  step={1}
                  aria-label={`Target at ${names.workplace(t.workplace)}`}
                  className="w-24 text-right"
                  value={current[t.workplace] ?? ""}
                  onChange={(e) => set(t.workplace, e.target.value)}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <ButtonRow>
        <Button variant="primary" disabled={publish.isPending} onClick={submit} data-testid="publish-plan">
          Publish the Plan
        </Button>
        {published ? <span className="text-good text-sm">Published. It is on the record and on every Work screen.</span> : null}
        {error ? (
          <span className="text-crit text-sm" role="alert">
            {error}
          </span>
        ) : null}
      </ButtonRow>
    </Card>
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
    <Card
      title="The land"
      icon="org"
      testId="land"
      subtitle="Opening is your word, not a vote: the Materials leave the Store the same hour. Closing sends its workers home this hour, its machines back to the Store, and frees the slot. Both go on the record with your name."
    >
      <p className={`m-0 mb-3 text-sm tabular-nums ${canPay ? "text-muted" : "text-attn"}`} data-testid="materials">
        A workplace costs <b className="text-ink">{plan.founding_materials} Materials</b>; the Store holds <b className={canPay ? "text-ink" : "text-attn"}>{plan.store_materials}</b>.
      </p>
      <table className="w-full border-collapse text-[15px]" data-testid="slots">
        <thead>
          <tr>
            <th className={TH}>Kind</th>
            <th className={`${TH_NUM} hidden md:table-cell`}>Open</th>
            <th className={`${TH_NUM} hidden md:table-cell`}>Slots</th>
            <th className={TH} />
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
              <tr key={k} className="hover:bg-surface-2" data-testid={`land-${k}`}>
                <td className={TD}>
                  {fieldName(k)}
                  <span className="text-muted block text-sm md:hidden">
                    {openHere} open · {limited ? `${free} of ${slots.length} slots free` : "no limit"}
                  </span>
                </td>
                <td className={`${TD_NUM} hidden md:table-cell`}>{openHere}</td>
                <td className={`${TD_NUM} hidden md:table-cell`}>{limited ? `${free} of ${slots.length} free` : <span className="text-muted">no limit</span>}</td>
                <td className={`${TD} text-right`}>
                  <Button
                    inline
                    disabled={open.isPending || blocked}
                    title={!canPay ? "The Store holds too few Materials." : limited && free === 0 ? "No free slot of this kind." : undefined}
                    onClick={() => {
                      setError(null);
                      setNote(null);
                      open.mutate({ kind: k }, { onSuccess: () => setNote(`Opened a ${fieldName(k)}; ${plan.founding_materials} Materials left the Store.`), onError: fail });
                    }}
                  >
                    Open a {fieldName(k)}
                  </Button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>

      <h3 className="text-muted mt-5 mb-2 text-xs font-bold tracking-caps uppercase">The collective's workplaces</h3>
      {collective.length === 0 ? (
        <p className="text-muted m-0">The collective has no workplace open.</p>
      ) : (
        <ul className="m-0 grid list-none gap-2 p-0" data-testid="collective-workplaces">
          {collective.map((t) => (
            <li key={t.workplace} data-testid={`workplace-${t.workplace}`}>
              <Tile className="flex flex-wrap items-center justify-between gap-2">
                <span>
                  <b>{names.workplace(t.workplace)}</b>
                  <span className="text-muted block text-sm">
                    {t.slot != null ? `slot ${t.slot} · ` : ""}
                    {t.machines} machines · {t.workers} working
                  </span>
                </span>
                {closing === t.workplace ? (
                  <span className="flex flex-wrap items-center gap-2">
                    <Button
                      variant="danger"
                      inline
                      disabled={close.isPending}
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
                    </Button>
                    <Button variant="quiet" inline onClick={() => setClosing(null)}>
                      keep it
                    </Button>
                  </span>
                ) : (
                  <Button inline onClick={() => setClosing(t.workplace)}>
                    Close
                  </Button>
                )}
              </Tile>
            </li>
          ))}
        </ul>
      )}
      {note ? (
        <p className="text-good mt-3 mb-0 text-sm" data-testid="land-note">
          {note}
        </p>
      ) : null}
      {error ? (
        <p className="text-crit mt-3 mb-0 text-sm" role="alert">
          {error}
        </p>
      ) : null}
    </Card>
  );
}

function Rationing({ id, caps, office }: { id: number; caps: CapabilitiesView; office: OfficeView }) {
  const store = useStore(id, caps.common_store);
  return (
    <Card
      title="The rationing rule"
      icon="store"
      testId="rationing"
      aside={
        store.data ? (
          <Pill tone="info" testId="rule-in-force">
            In force: {fieldName(store.data.rule)}
          </Pill>
        ) : null
      }
      subtitle={store.data ? ruleText(store.data.rule) : null}
    >
      <p className="text-muted mt-0 mb-3 text-sm">
        Only a coordinator may move the rule; the assembly decides it like any other motion, at the end of the day. Take the argument to the floor on the{" "}
        <Link to="/s/$id/assembly" params={{ id: String(id) }}>
          assembly
        </Link>{" "}
        screen.
      </p>
      <BallotBuilder id={id} caps={caps} citizens={[]} offices={[office]} only={["policy_change"]} fields={["rationing"]} />
    </Card>
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
  if (caps.error) return <p className="text-crit">Could not load: {String(caps.error)}</p>;
  // No governance, no office to sit in: said before anyone is asked to join.
  if (!governed) return <p className="text-muted">There is no {OFFICE} in this society.</p>;
  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }}>
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error) return <p className="text-crit">Could not load: {String(home.error)}</p>;
  if (offices.isPending) return <p className="text-muted">Loading.</p>;
  if (offices.error) return <p className="text-crit">Could not load: {String(offices.error)}</p>;
  if (!office) return <p className="text-muted">This society has no {OFFICE}s.</p>;
  const me = home.data!.citizen.id;
  const fellows = office.holders.filter((h) => h.citizen !== me);

  // The refusal page (the done gate): who does sit, and where to stand.
  if (!holder) {
    return (
      <div>
        <PageHeader title={<span className="capitalize">{fieldName(OFFICE)}</span>} />
        <Card title="Not your workspace" icon="office" testId="not-a-coordinator">
          <p className="mt-0">You do not sit as {fieldName(OFFICE)}, so this workspace is not yours.</p>
          <p className="text-muted m-0">
            {office.holders.length === 0
              ? "Nobody sits; the seats are open."
              : `Sitting now: ${office.holders.map((h) => `${h.handle} through ${dayOf(h.term_ends_cycle)}`).join(", ")}.`}{" "}
            {office.election ? "An election is open: " : "The next election opens when a seat empties: "}
            stand on the{" "}
            <Link to="/s/$id/assembly" params={{ id: String(id) }}>
              {t("assembly")}
            </Link>{" "}
            screen.
          </p>
        </Card>
      </div>
    );
  }
  if (plan.isPending) return <p className="text-muted">Loading.</p>;
  if (plan.error) return <p className="text-crit">Could not load: {String(plan.error)}</p>;
  const p = plan.data!;
  const myTerm = office.holders.find((h) => h.citizen === me);
  const c = caps.data as CapabilitiesView;
  const measured = p.targets.filter((t) => t.last_fulfillment != null);
  const limited = p.slots.length > 0;
  const freeSlots = limited ? p.slots.filter((s) => s.workplace == null).length : null;
  const verdict = coordinatorVerdict({
    office: fieldName(OFFICE),
    termEnds: myTerm?.term_ends_cycle ?? null,
    planPublished: p.published_cycle != null,
    targets: { set: p.targets.filter((t) => t.target != null).length, met: measured.filter((t) => (t.last_fulfillment ?? 0) >= 1).length, measured: measured.length },
    materials: { held: p.store_materials, cost: p.founding_materials },
    freeSlots,
  });
  const collective = p.targets.filter((t) => t.collective);

  return (
    <div data-testid="coordinator">
      <PageHeader
        title={<span className="capitalize">{fieldName(OFFICE)}</span>}
        meta={
          <span data-testid="fellows">
            {myTerm ? (
              <>
                Your term runs through <b>{dayOf(myTerm.term_ends_cycle)}</b>
              </>
            ) : (
              "You sit"
            )}
            {fellows.length > 0 ? `, with ${fellows.map((h) => h.handle).join(" and ")}` : office.seats > 1 ? `; ${office.seats - 1} ${office.seats - 1 === 1 ? "seat is" : "seats are"} empty` : ""}. Recall is by{" "}
            {fieldName(office.recall)}, any day.
          </span>
        }
      />
      <Stack>
        <Card title="Where you stand" icon="office">
          <Verdict parts={verdict} />
          <Figures>
            <Figure label="Materials in the Store" value={String(p.store_materials)} status={`${p.founding_materials} open a workplace`} tone={p.store_materials >= p.founding_materials ? "good" : "attn"} />
            <Figure label="Collective workplaces" value={String(collective.length)} status={limited ? `${freeSlots} of ${p.slots.length} slots free` : "no limit on the land"} tone={limited && freeSlots === 0 ? "attn" : "good"} />
            <Figure label="Targets set" value={String(p.targets.filter((t) => t.target != null).length)} unit={`/ ${p.targets.length}`} status={measured.length > 0 ? `${measured.filter((t) => (t.last_fulfillment ?? 0) >= 1).length} of ${measured.length} met yesterday` : "nothing measured yet"} />
            <Figure label="Working now" value={String(collective.reduce((n, t) => n + t.workers, 0))} status="at the collective's workplaces" />
          </Figures>
        </Card>
        <PlanEditor id={id} plan={p} names={names} />
        <Land id={id} plan={p} names={names} />
        <Rationing id={id} caps={c} office={office} />
      </Stack>
    </div>
  );
}
