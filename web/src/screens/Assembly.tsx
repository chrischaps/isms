// The Assembly (GDD 6.2 Governance, 8.1, 12; S2.6): what is before the
// assembly tonight, with the roll and the quorum drawn where it is; what it
// decided this epoch and what each decision did; the offices, who holds
// them and the election open for each; and the builder for a motion of
// your own. Every proposal has a floor, which is a Talk channel with a
// record. Mounted where `caps.governance` is not "none".

import { useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import {
  useApprove,
  useBallot,
  useOffices,
  useProposals,
  useStand,
  useWithdraw,
  type Ballot,
  type OfficeView,
  type ProposalKind,
  type ProposalView,
} from "../api/assembly";
import { ApiError, type CapabilitiesView, type Clock, type EventRef } from "../api/client";
import { useCitizens } from "../api/civic";
import { useCapabilities, useHome, useLexicon, useSociety } from "../api/hooks";
import { Countdown } from "../components/Countdown";
import { Meter } from "../components/Meter";
import { useNames, type Names } from "../lib/names";
import { diffText, fieldName, kindSummary, kindTitle, policyDiff, quorumBar, wouldCarry } from "../lib/policy";
import { dayOf, whenOfTick } from "../lib/when";
import { BallotBuilder } from "./roles/BallotBuilder";
import { Talk } from "./Talk";

/** When the end of the engine's 0-based `cycle` falls, on the wall clock. */
function endOfCycleAt(clock: Clock, cycle: number, nextTickAt: string | null | undefined, tickSeconds: number): string | null {
  if (!nextTickAt || tickSeconds === 0) return null;
  const remaining = (cycle - (clock.cycle - 1)) * clock.ticks_per_cycle + (clock.ticks_per_cycle - clock.tick);
  return new Date(new Date(nextTickAt).getTime() + Math.max(0, remaining) * tickSeconds * 1000).toISOString();
}

function KindBadge({ tag }: { tag: string }) {
  return <span className="border-line text-muted rounded-sm border px-1 text-xs uppercase tracking-wide">{kindTitle(tag)}</span>;
}

function TallyLine({ p }: { p: ProposalView }) {
  const t = p.tally;
  return (
    <span className="num text-sm" data-testid="tally">
      <span className="text-ink">{t.yes}</span> yes · <span className="text-ink">{t.no}</span> no · {t.abstain} abstain
      <span className="text-muted">
        {" "}
        · {t.cast} of {t.eligible} voting
      </span>
    </span>
  );
}

function OpenProposal({
  id,
  p,
  clock,
  society,
  me,
  names,
}: {
  id: number;
  p: ProposalView;
  clock: Clock;
  society: { next_tick_at?: string | null; tick_seconds: number } | undefined;
  me: number;
  names: Names;
}) {
  const ballot = useBallot(id);
  const [floor, setFloor] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const q = quorumBar(p.tally);
  const closesAt = society ? endOfCycleAt(clock, p.closes_cycle, society.next_tick_at, society.tick_seconds) : null;
  const cast = (b: Ballot) => {
    setError(null);
    ballot.mutate({ pid: p.id, ballot: b }, { onError: (e) => setError(e.message) });
  };
  const members = p.org != null;
  return (
    <article className="rule flex flex-col gap-3 pt-4" data-testid={`proposal-${p.id}`}>
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h4 className="text-lg">
          {p.title} <KindBadge tag={p.kind_tag} />
        </h4>
        <span className="text-muted text-sm">
          moved by {names.citizen(p.by)}, {whenOfTick(p.opened_tick, clock.ticks_per_cycle)}
        </span>
      </header>
      <p className="text-sm">{kindSummary(p.kind as unknown as ProposalKind, names.citizen, names.org)}</p>
      {p.text ? <p className="text-muted whitespace-pre-wrap text-sm">{p.text}</p> : null}
      {members ? <p className="text-muted text-xs">A members' vote of {names.org(p.org!)}: a majority of the membership carries it.</p> : null}

      <div className="grid gap-3 md:grid-cols-[1fr_16rem]">
        <div className="flex flex-col gap-2">
          <TallyLine p={p} />
          {!members ? (
            <Meter
              label="Quorum"
              value={q.value}
              threshold={q.threshold}
              hint={`${p.tally.quorum} of ${p.tally.eligible} ballots make a quorum; ${p.tally.cast} cast so far. Abstentions count, and so does the ballot your standing plan casts for you at the close.`}
            />
          ) : null}
          <p className="text-muted text-xs">
            {wouldCarry(p.tally) ? "Carries as it stands" : q.met ? "Fails as it stands" : "Short of a quorum"} · closes at the end of{" "}
            {dayOf(p.closes_cycle)}
            {closesAt ? (
              <>
                {" "}
                · <Countdown at={closesAt} label="in" />
              </>
            ) : null}
          </p>
        </div>
        <div className="flex flex-col gap-2">
          <span className="text-muted text-xs uppercase tracking-wide">Your ballot</span>
          <div className="flex gap-2" role="group" aria-label={`Ballot on ${p.title}`}>
            {(["yes", "no", "abstain"] as const).map((b) => (
              <button
                key={b}
                type="button"
                aria-pressed={p.my_ballot === b}
                disabled={ballot.isPending}
                className={`rounded-sm border px-3 py-1 text-sm capitalize ${p.my_ballot === b ? "bg-ink text-paper border-ink" : "border-line"}`}
                onClick={() => cast(b)}
              >
                {b}
              </button>
            ))}
          </div>
          {p.my_ballot ? (
            <span className="text-muted text-xs">Cast. You may change it until the close.</span>
          ) : (
            <span className="text-muted text-xs">Uncast: your standing plan's default acts at the close.</span>
          )}
          {error ? (
            <span className="text-bad text-xs" role="alert">
              {error}
            </span>
          ) : null}
        </div>
      </div>

      {p.ballots.length > 0 ? (
        <p className="text-muted text-xs" data-testid="roll">
          The roll:{" "}
          {p.ballots.map((b, i) => (
            <span key={b.citizen}>
              {i > 0 ? ", " : ""}
              <span className={b.citizen === me ? "text-ink" : ""}>{b.citizen === me ? "you" : b.handle}</span> {b.ballot}
            </span>
          ))}
        </p>
      ) : null}

      <div className="flex items-baseline gap-3 text-xs">
        <button type="button" className="text-muted underline" aria-expanded={floor} onClick={() => setFloor((f) => !f)}>
          {floor ? "Close the floor" : "Open the floor"}
        </button>
        <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `assembly:${p.id}` }} className="text-muted">
          full page
        </Link>
      </div>
      {floor ? (
        <div className="border-line rounded-sm border p-3">
          <Talk id={id} channel={`assembly:${p.id}`} embedded />
        </div>
      ) : null}
    </article>
  );
}

/** An event payload is externally tagged (`{ PolicyChanged: { policy, by, proposal } }`): the fields inside. */
function fieldsOf(e: EventRef): Record<string, unknown> {
  return ((e.payload[e.kind] ?? e.payload) as Record<string, unknown>) ?? {};
}

/** What a closed proposal did, from its effects: the policy diff, the honor, the disbursement. */
function Effects({ id, p, before, names }: { id: number; p: ProposalView; before: Record<string, unknown> | null; names: Names }) {
  const effects = (p.outcome?.effects ?? []) as EventRef[];
  if (effects.length === 0) return <span className="text-muted">{p.outcome?.passed ? "no effect on the rules" : "nothing"}</span>;
  const kind = p.kind as unknown as ProposalKind;
  return (
    <ul className="flex flex-col gap-1">
      {effects.map((e) => {
        let line: string;
        if (e.kind === "PolicyChanged" && typeof kind === "object" && "policy_change" in kind) {
          const after = (fieldsOf(e).policy ?? {}) as Record<string, unknown>;
          const rows = policyDiff(kind.policy_change.patch, before, after);
          line = rows.length > 0 ? rows.map(diffText).join("; ") : "policy rewritten";
        } else if (e.kind === "Honored") {
          line = `${names.citizen(Number(fieldsOf(e).citizen))} honored`;
        } else if (e.kind === "Disbursed") {
          line = `disbursed to ${names.party(fieldsOf(e).to)}`;
        } else {
          line = e.kind.replace(/([a-z])([A-Z])/g, "$1 $2");
        }
        return (
          <li key={e.seq}>
            <Link to="/s/$id/events/$seq" params={{ id: String(id), seq: String(e.seq) }} className="underline">
              {e.kind.replace(/([a-z])([A-Z])/g, "$1 $2")}
            </Link>
            : <span data-testid="effect">{line}</span>
          </li>
        );
      })}
    </ul>
  );
}

function ClosedLedger({ id, closed, names }: { id: number; closed: ProposalView[]; names: Names }) {
  // Each carried policy change is read against the policy the previous one left (or none on record).
  const befores = useMemo(() => {
    const changes = closed
      .flatMap((p) => (p.outcome?.effects ?? []) as EventRef[])
      .filter((e) => e.kind === "PolicyChanged")
      .sort((a, b) => a.seq - b.seq);
    const map = new Map<number, Record<string, unknown> | null>();
    let prev: Record<string, unknown> | null = null;
    for (const e of changes) {
      map.set(e.seq, prev);
      prev = (fieldsOf(e).policy ?? null) as Record<string, unknown> | null;
    }
    return map;
  }, [closed]);
  if (closed.length === 0) return <p className="text-muted text-sm">Nothing has closed this epoch.</p>;
  const newestFirst = closed.slice().reverse();
  return (
    <table className="w-full text-sm" data-testid="closed-ledger">
      <thead className="text-muted text-left text-xs uppercase tracking-wide">
        <tr>
          <th className="py-1 font-normal">Closed</th>
          <th className="py-1 font-normal">Proposal</th>
          <th className="py-1 font-normal">Outcome</th>
          <th className="py-1 font-normal">Tally</th>
          <th className="py-1 font-normal">Did</th>
        </tr>
      </thead>
      <tbody>
        {newestFirst.map((p) => {
          const o = p.outcome!;
          const first = ((o.effects ?? []) as EventRef[]).find((e) => e.kind === "PolicyChanged");
          return (
            <tr key={p.id} className="rule align-top" data-testid={`closed-${p.id}`}>
              <td className="num py-2 pr-3 whitespace-nowrap">{dayOf(o.closed_cycle)}</td>
              <td className="py-2 pr-3">
                <span>{p.title}</span> <KindBadge tag={p.kind_tag} />
                <div className="text-muted text-xs">
                  moved by {names.citizen(p.by)}
                  {p.floor_open ? (
                    <>
                      {" "}
                      ·{" "}
                      <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `assembly:${p.id}` }} className="underline">
                        the floor is still open
                      </Link>
                    </>
                  ) : (
                    <>
                      {" "}
                      ·{" "}
                      <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `assembly:${p.id}` }} className="underline">
                        minutes
                      </Link>
                    </>
                  )}
                </div>
              </td>
              <td className={`py-2 pr-3 ${o.passed ? "text-ink" : "text-muted"}`} data-testid="outcome">
                {o.passed ? "carried" : o.tally.cast < o.tally.quorum ? "no quorum" : "failed"}
              </td>
              <td className="num py-2 pr-3 whitespace-nowrap">
                {o.tally.yes}–{o.tally.no}
                {o.tally.abstain > 0 ? <span className="text-muted"> ({o.tally.abstain} abstain)</span> : null}
              </td>
              <td className="py-2">
                <Effects id={id} p={p} before={first ? (befores.get(first.seq) ?? null) : null} names={names} />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function Office({
  id,
  o,
  clock,
  society,
  me,
}: {
  id: number;
  o: OfficeView;
  clock: Clock;
  society: { next_tick_at?: string | null; tick_seconds: number } | undefined;
  me: number;
}) {
  const stand = useStand(id);
  const withdraw = useWithdraw(id);
  const approve = useApprove(id);
  const [error, setError] = useState<string | null>(null);
  const e = o.election;
  const closesAt = e && society ? endOfCycleAt(clock, e.closes_cycle, society.next_tick_at, society.tick_seconds) : null;
  const fail = (err: Error) => setError(err.message);
  const toggle = (citizen: number, on: boolean) => {
    if (!e) return;
    const set = new Set(e.my_approvals);
    if (on) set.add(citizen);
    else set.delete(citizen);
    setError(null);
    approve.mutate({ kind: o.kind, candidates: [...set].sort((a, b) => a - b) }, { onError: fail });
  };
  return (
    <section className="flex flex-col gap-2" data-testid={`office-${o.kind}`}>
      <h4 className="text-base capitalize">
        {fieldName(o.kind)}
        {o.i_hold ? <span className="text-accent ml-2 text-xs uppercase tracking-wide">you hold this</span> : null}
      </h4>
      <p className="text-muted text-xs">
        {o.seats} {o.seats === 1 ? "seat" : "seats"} · {o.term_cycles}-day terms{o.consecutive ? "" : ", none consecutive"} · recall by{" "}
        {fieldName(o.recall)}
      </p>
      <ul className="text-sm" data-testid="holders">
        {o.holders.length === 0 ? <li className="text-muted">Nobody sits here.</li> : null}
        {o.holders.map((h) => (
          <li key={h.citizen}>
            <span className={h.citizen === me ? "text-ink" : ""}>{h.citizen === me ? "you" : h.handle}</span>
            <span className="text-muted"> through {dayOf(h.term_ends_cycle)}</span>
          </li>
        ))}
        {o.short_since != null && o.holders.length < o.seats ? (
          <li className="text-muted text-xs">
            {o.seats - o.holders.length} {o.seats - o.holders.length === 1 ? "seat" : "seats"} short since {dayOf(o.short_since)}
          </li>
        ) : null}
      </ul>
      {e ? (
        <div className="bg-paper-2 flex flex-col gap-2 rounded-sm p-2 text-sm" data-testid="election">
          <p className="text-xs">
            Election open for {e.seats} {e.seats === 1 ? "seat" : "seats"}: approve any you would seat. Closes at the end of {dayOf(e.closes_cycle)}
            {closesAt ? (
              <>
                {" "}
                · <Countdown at={closesAt} label="in" />
              </>
            ) : null}
            <span className="text-muted"> · {e.ballots_cast} ballots cast</span>
          </p>
          {e.candidates.length === 0 ? <p className="text-muted text-xs">No candidates yet.</p> : null}
          <ul className="flex flex-col gap-1">
            {e.candidates.map((c) => (
              <li key={c.citizen} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  aria-label={`Approve ${c.handle}`}
                  checked={e.my_approvals.includes(c.citizen)}
                  disabled={approve.isPending}
                  onChange={(ev) => toggle(c.citizen, ev.target.checked)}
                />
                <span className={c.citizen === me ? "text-ink" : ""}>{c.citizen === me ? "you" : c.handle}</span>
                <span className="num text-muted text-xs">
                  {c.approvals} {c.approvals === 1 ? "approval" : "approvals"}
                </span>
              </li>
            ))}
          </ul>
          <div className="flex flex-wrap items-baseline gap-3">
            {e.i_stand ? (
              <button
                type="button"
                disabled={withdraw.isPending}
                className="border-line rounded-sm border px-2 py-0.5 text-xs"
                onClick={() => {
                  setError(null);
                  withdraw.mutate(o.kind, { onError: fail });
                }}
              >
                Withdraw
              </button>
            ) : e.stand_refusal ? (
              <span className="text-muted text-xs">You cannot stand: {e.stand_refusal}</span>
            ) : (
              <button
                type="button"
                disabled={stand.isPending}
                className="border-line rounded-sm border px-2 py-0.5 text-xs"
                onClick={() => {
                  setError(null);
                  stand.mutate(o.kind, { onError: fail });
                }}
              >
                Stand for {fieldName(o.kind)}
              </button>
            )}
            {e.i_stand ? <span className="text-muted text-xs">You stand.</span> : null}
            {error ? (
              <span className="text-bad text-xs" role="alert">
                {error}
              </span>
            ) : null}
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function Assembly({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const society = useSociety(id);
  const citizen = home.data !== undefined;
  const proposals = useProposals(id, citizen);
  const offices = useOffices(id, citizen);
  const citizens = useCitizens(id);
  const names = useNames(id, home.data?.citizen.id, citizen);

  if (caps.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error) return <p className="text-bad">Could not load: {String(caps.error)}</p>;
  // No governance, no assembly: said before anyone is asked to join.
  if ((caps.data as CapabilitiesView).governance === "none") return <p className="text-muted">There is no assembly in this society.</p>;
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
  const c = caps.data as CapabilitiesView;
  if (proposals.isPending || offices.isPending) return <p className="text-muted">Loading.</p>;
  if (proposals.error || offices.error) return <p className="text-bad">Could not load: {String(proposals.error ?? offices.error)}</p>;
  const v = proposals.data!;
  const me = home.data!.citizen.id;
  const mine = v.open.filter((p) => p.by === me).length;
  const quorumOf = Math.max(1, Math.ceil(v.electorate * v.quorum_fraction));
  const people = (citizens.data?.citizens ?? []).map((z) => ({ id: z.id, handle: z.handle }));

  return (
    <div className="flex flex-col gap-10">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{t("assembly")}</h2>
        <p className="text-muted text-sm" data-testid="assembly-rule">
          Proposals close at the end of the day. {quorumOf} of {v.electorate} {v.electorate === 1 ? "voter makes" : "voters make"} a quorum; a
          simple majority carries.
        </p>
      </header>

      <div className="grid gap-10 lg:grid-cols-[1fr_20rem]">
        <div className="flex flex-col gap-10">
          <section className="flex flex-col gap-2" data-testid="open-proposals">
            <h3 className="text-lg">Before the assembly</h3>
            {v.open.length === 0 ? (
              <p className="text-muted text-sm">Nothing is before the assembly tonight. Move something below.</p>
            ) : (
              v.open.map((p) => <OpenProposal key={p.id} id={id} p={p} clock={v.clock} society={society.data} me={me} names={names} />)
            )}
          </section>

          <section className="flex flex-col gap-3" data-testid="move-proposal">
            <h3 className="text-lg">Move a proposal</h3>
            <p className="text-muted text-xs">
              {c.proposers === "office_holders" ? "Office-holders move proposals here." : "Any citizen may move one."} You hold {mine} of the{" "}
              {v.open_per_citizen} open proposals a citizen may have at once.
            </p>
            <BallotBuilder id={id} caps={c} citizens={people} offices={offices.data!.offices} />
          </section>

          <section className="flex flex-col gap-2" data-testid="closed-proposals">
            <h3 className="text-lg">Decided this epoch</h3>
            <ClosedLedger id={id} closed={v.closed} names={names} />
          </section>
        </div>

        <aside className="flex flex-col gap-6" data-testid="offices">
          <h3 className="text-lg">Offices</h3>
          {offices.data!.offices.length === 0 ? <p className="text-muted text-sm">This society has no offices.</p> : null}
          {offices.data!.offices.map((o) => (
            <Office key={o.kind} id={id} o={o} clock={offices.data!.clock} society={society.data} me={me} />
          ))}
          <p className="text-muted text-xs">Everything said on a floor is logged and may be published, pseudonymously, as research data.</p>
        </aside>
      </div>
    </div>
  );
}
