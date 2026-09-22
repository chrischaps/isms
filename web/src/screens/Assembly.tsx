// The Assembly (GDD 6.2 Governance, 8.1, 12; S2.6; docs/style.md §10 by
// analogy): what is before the assembly tonight, with the roll and the
// quorum drawn where it is; the builder for a motion of your own; the
// offices, who holds them and the election open for each; and what the
// assembly decided this epoch and what each decision did. The Verdict says
// how many motions are up and whether a ballot of yours is owed. Every
// proposal has a floor, which is a Talk channel with a record. Mounted where
// `caps.governance` is not "none".

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
import { Button } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { Countdown } from "../components/Countdown";
import { TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { Meter } from "../components/Meter";
import { PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { useNames, type Names } from "../lib/names";
import { diffText, fieldName, kindSummary, kindTitle, policyDiff, quorumBar, wouldCarry } from "../lib/policy";
import { assemblyVerdict } from "../lib/verdict";
import { dayOf, whenOfTick } from "../lib/when";
import { BallotBuilder } from "./roles/BallotBuilder";
import { Talk } from "./Talk";

/** When the end of the engine's 0-based `cycle` falls, on the wall clock. */
function endOfCycleAt(clock: Clock, cycle: number, nextTickAt: string | null | undefined, tickSeconds: number): string | null {
  if (!nextTickAt || tickSeconds === 0) return null;
  const remaining = (cycle - (clock.cycle - 1)) * clock.ticks_per_cycle + (clock.ticks_per_cycle - clock.tick);
  return new Date(new Date(nextTickAt).getTime() + Math.max(0, remaining) * tickSeconds * 1000).toISOString();
}

type Society = { next_tick_at?: string | null; tick_seconds: number } | undefined;

function TallyLine({ p }: { p: ProposalView }) {
  const t = p.tally;
  return (
    <span className="text-sm tabular-nums" data-testid="tally">
      <b>{t.yes}</b> yes · <b>{t.no}</b> no · {t.abstain} abstain
      <span className="text-muted"> · {t.cast} of {t.eligible} voting</span>
    </span>
  );
}

const BALLOTS: Ballot[] = ["yes", "no", "abstain"];

function OpenProposal({ id, p, clock, society, me, names }: { id: number; p: ProposalView; clock: Clock; society: Society; me: number; names: Names }) {
  const ballot = useBallot(id);
  const { t } = useLexicon(id);
  const [floor, setFloor] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const q = quorumBar(p.tally);
  const closesAt = society ? endOfCycleAt(clock, p.closes_cycle, society.next_tick_at, society.tick_seconds) : null;
  const cast = (b: Ballot) => {
    setError(null);
    ballot.mutate({ pid: p.id, ballot: b }, { onError: (e) => setError(e.message) });
  };
  const members = p.org != null;
  const standing = wouldCarry(p.tally) ? "Carries as it stands" : q.met ? "Fails as it stands" : "Short of a quorum";
  const standingTone = wouldCarry(p.tally) ? "text-good" : q.met ? "text-crit" : "text-attn";
  return (
    <Tile className="grid gap-3">
      <article className="grid gap-3" data-testid={`proposal-${p.id}`}>
        <header className="grid gap-1">
          <h3 className="flex flex-wrap items-center gap-2 text-lg">
            <span>{p.title}</span>
            <Pill>{kindTitle(p.kind_tag)}</Pill>
          </h3>
          <span className="text-muted text-sm">
            moved by {names.citizen(p.by)}, {whenOfTick(p.opened_tick, clock.ticks_per_cycle)}
          </span>
        </header>
        <p className="m-0">{kindSummary(p.kind as unknown as ProposalKind, names.citizen, names.org)}</p>
        {p.text ? <p className="text-muted m-0 text-sm whitespace-pre-wrap">{p.text}</p> : null}
        {members ? <p className="text-muted m-0 text-sm">A members' vote of {names.org(p.org!)}: a majority of the membership carries it.</p> : null}

        <div className="grid gap-4 md:grid-cols-[1fr_16rem]">
          <div className="grid content-start gap-2">
            <TallyLine p={p} />
            {!members ? (
              <Meter
                label="Quorum"
                value={q.value}
                threshold={q.threshold}
                hint={`${p.tally.quorum} of ${p.tally.eligible} ballots make a quorum; ${p.tally.cast} cast so far. Abstentions count, and so does the ballot your standing plan casts for you at the close.`}
              />
            ) : null}
            <p className="text-muted m-0 text-sm">
              <b className={standingTone}>{standing}</b> · closes at the end of {dayOf(p.closes_cycle)}
              {closesAt ? (
                <>
                  {" "}
                  · <Countdown at={closesAt} label="in" />
                </>
              ) : null}
            </p>
          </div>
          <div className="grid content-start gap-2">
            <span className="text-muted text-xs font-bold tracking-caps uppercase">Your {t("ballot").toLowerCase()}</span>
            <div className="flex gap-2" role="group" aria-label={`Ballot on ${p.title}`}>
              {BALLOTS.map((b) => {
                const on = p.my_ballot === b;
                return (
                  <button
                    key={b}
                    type="button"
                    aria-pressed={on}
                    disabled={ballot.isPending}
                    className={[
                      "min-h-touch flex-1 rounded-md border px-3 text-sm font-bold capitalize transition-colors",
                      on ? "bg-ink text-bg border-ink" : "bg-surface border-line hover:border-line-strong",
                    ].join(" ")}
                    onClick={() => cast(b)}
                  >
                    {b}
                  </button>
                );
              })}
            </div>
            <span className="text-muted text-sm">{p.my_ballot ? "Cast. You may change it until the close." : "Uncast: your standing plan's default acts at the close."}</span>
            {error ? (
              <span className="text-crit text-sm" role="alert">
                {error}
              </span>
            ) : null}
          </div>
        </div>

        {p.ballots.length > 0 ? (
          <p className="text-muted m-0 text-sm" data-testid="roll">
            The roll:{" "}
            {p.ballots.map((b, i) => (
              <span key={b.citizen}>
                {i > 0 ? ", " : ""}
                <span className={b.citizen === me ? "text-ink font-bold" : ""}>{b.citizen === me ? "you" : b.handle}</span> {b.ballot}
              </span>
            ))}
          </p>
        ) : null}

        <div className="flex flex-wrap items-center gap-2.5">
          <Button variant="quiet" inline aria-expanded={floor} onClick={() => setFloor((f) => !f)}>
            {floor ? "Close the floor" : "Open the floor"}
          </Button>
          <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `assembly:${p.id}` }} className="text-muted text-sm font-normal">
            full page
          </Link>
        </div>
        {floor ? (
          <div className="border-line rounded-md border p-3">
            <Talk id={id} channel={`assembly:${p.id}`} embedded />
          </div>
        ) : null}
      </article>
    </Tile>
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
    <ul className="m-0 grid list-none gap-1 p-0">
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
            <Link to="/s/$id/events/$seq" params={{ id: String(id), seq: String(e.seq) }}>
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
  const { t } = useLexicon(id);
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
  if (closed.length === 0) return <p className="text-muted m-0">Nothing has closed this epoch.</p>;
  const newestFirst = closed.slice().reverse();
  return (
    <table className="w-full border-collapse text-[15px]" data-testid="closed-ledger">
      <thead>
        <tr>
          <th className={`${TH} hidden md:table-cell`}>Closed</th>
          <th className={TH}>{t("proposal")}</th>
          <th className={TH}>Outcome</th>
          <th className={`${TH_NUM} hidden md:table-cell`}>Tally</th>
          <th className={TH}>Did</th>
        </tr>
      </thead>
      <tbody>
        {newestFirst.map((p) => {
          const o = p.outcome!;
          const first = ((o.effects ?? []) as EventRef[]).find((e) => e.kind === "PolicyChanged");
          const tally = `${o.tally.yes}–${o.tally.no}${o.tally.abstain > 0 ? ` (${o.tally.abstain} abstain)` : ""}`;
          const floor = (
            <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `assembly:${p.id}` }} className="font-normal underline">
              {p.floor_open ? "the floor is still open" : "minutes"}
            </Link>
          );
          return (
            <tr key={p.id} className="hover:bg-surface-2" data-testid={`closed-${p.id}`}>
              <td className={`${TD} hidden whitespace-nowrap md:table-cell`}>{dayOf(o.closed_cycle)}</td>
              <td className={TD}>
                <span>{p.title}</span> <Pill>{kindTitle(p.kind_tag)}</Pill>
                <span className="text-muted block text-sm">
                  moved by {names.citizen(p.by)} · {floor}
                </span>
                <span className="text-muted block text-sm md:hidden">
                  closed {dayOf(o.closed_cycle)} · {tally}
                </span>
              </td>
              <td className={TD}>
                <span className={o.passed ? "font-bold" : "text-muted"} data-testid="outcome">
                  {o.passed ? "carried" : o.tally.cast < o.tally.quorum ? "no quorum" : "failed"}
                </span>
              </td>
              <td className={`${TD_NUM} hidden md:table-cell`}>{tally}</td>
              <td className={TD}>
                <Effects id={id} p={p} before={first ? (befores.get(first.seq) ?? null) : null} names={names} />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function Office({ id, o, clock, society, me }: { id: number; o: OfficeView; clock: Clock; society: Society; me: number }) {
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
  const short = o.seats - o.holders.length;
  return (
    <Tile className="grid gap-2" testId={`office-${o.kind}`}>
      <h3 className="flex flex-wrap items-center gap-2 text-base capitalize">
        <span>{fieldName(o.kind)}</span>
        {o.i_hold ? <Pill tone="good">you hold this</Pill> : null}
      </h3>
      <p className="text-muted m-0 text-sm">
        {o.seats} {o.seats === 1 ? "seat" : "seats"} · {o.term_cycles}-day terms{o.consecutive ? "" : ", none consecutive"} · recall by {fieldName(o.recall)}
      </p>
      <ul className="m-0 grid list-none gap-0.5 p-0" data-testid="holders">
        {o.holders.length === 0 ? <li className="text-muted">Nobody sits here.</li> : null}
        {o.holders.map((h) => (
          <li key={h.citizen}>
            <span className={h.citizen === me ? "font-bold" : ""}>{h.citizen === me ? "you" : h.handle}</span>
            <span className="text-muted"> through {dayOf(h.term_ends_cycle)}</span>
          </li>
        ))}
        {o.short_since != null && short > 0 ? (
          <li className="text-attn text-sm">
            {short} {short === 1 ? "seat" : "seats"} short since {dayOf(o.short_since)}
          </li>
        ) : null}
      </ul>
      {e ? (
        <div className="bg-surface-2 grid gap-2 rounded-md p-3 text-sm" data-testid="election">
          <p className="m-0">
            <Pill tone="info">election open</Pill> for {e.seats} {e.seats === 1 ? "seat" : "seats"}: approve any you would seat. Closes at the end of {dayOf(e.closes_cycle)}
            {closesAt ? (
              <>
                {" "}
                · <Countdown at={closesAt} label="in" />
              </>
            ) : null}
            <span className="text-muted"> · {e.ballots_cast} ballots cast</span>
          </p>
          {e.candidates.length === 0 ? <p className="text-muted m-0">No candidates yet.</p> : null}
          <ul className="m-0 grid list-none gap-1 p-0">
            {e.candidates.map((c) => (
              <li key={c.citizen} className="flex min-h-touch items-center gap-2">
                <input
                  type="checkbox"
                  aria-label={`Approve ${c.handle}`}
                  checked={e.my_approvals.includes(c.citizen)}
                  disabled={approve.isPending}
                  onChange={(ev) => toggle(c.citizen, ev.target.checked)}
                />
                <span className={c.citizen === me ? "font-bold" : ""}>{c.citizen === me ? "you" : c.handle}</span>
                <span className="text-muted text-xs tabular-nums">
                  {c.approvals} {c.approvals === 1 ? "approval" : "approvals"}
                </span>
              </li>
            ))}
          </ul>
          <div className="flex flex-wrap items-center gap-2.5">
            {e.i_stand ? (
              <Button
                disabled={withdraw.isPending}
                onClick={() => {
                  setError(null);
                  withdraw.mutate(o.kind, { onError: fail });
                }}
              >
                Withdraw
              </Button>
            ) : e.stand_refusal ? (
              <span className="text-muted">You cannot stand: {e.stand_refusal}</span>
            ) : (
              <Button
                variant="primary"
                disabled={stand.isPending}
                onClick={() => {
                  setError(null);
                  stand.mutate(o.kind, { onError: fail });
                }}
              >
                Stand for {fieldName(o.kind)}
              </Button>
            )}
            {e.i_stand ? <span className="text-muted">You stand.</span> : null}
            {error ? (
              <span className="text-crit" role="alert">
                {error}
              </span>
            ) : null}
          </div>
        </div>
      ) : null}
    </Tile>
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
  if (caps.error) return <p className="text-crit">Could not load: {String(caps.error)}</p>;
  // No governance, no assembly: said before anyone is asked to join.
  if ((caps.data as CapabilitiesView).governance === "none") return <p className="text-muted">There is no assembly in this society.</p>;
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
  const c = caps.data as CapabilitiesView;
  if (proposals.isPending || offices.isPending) return <p className="text-muted">Loading.</p>;
  if (proposals.error || offices.error) return <p className="text-crit">Could not load: {String(proposals.error ?? offices.error)}</p>;
  const v = proposals.data!;
  const os = offices.data!.offices;
  const me = home.data!.citizen.id;
  const mine = v.open.filter((p) => p.by === me).length;
  const quorumOf = Math.max(1, Math.ceil(v.electorate * v.quorum_fraction));
  const people = (citizens.data?.citizens ?? []).map((z) => ({ id: z.id, handle: z.handle }));
  const verdict = assemblyVerdict({
    open: v.open.length,
    uncast: v.open.filter((p) => !p.my_ballot).length,
    mine,
    offices: os.map((o) => ({
      kind: fieldName(o.kind),
      seats: o.seats,
      holders: o.holders.length,
      iHold: o.i_hold,
      election: o.election != null,
      iStand: o.election?.i_stand ?? false,
    })),
  });

  return (
    <div>
      <PageHeader
        title={t("assembly")}
        meta={
          <span data-testid="assembly-rule">
            Proposals close at the end of the day. <b>{quorumOf}</b> of {v.electorate} {v.electorate === 1 ? "voter makes" : "voters make"} a quorum; a simple majority carries.
          </span>
        }
      />
      <Stack>
        <Card title="Before the assembly" icon="assembly" testId="open-proposals" aside={v.open.length > 0 ? <span className="text-muted text-sm font-normal">{v.open.length}</span> : null}>
          <Verdict parts={verdict} />
          {v.open.length === 0 ? (
            <p className="text-muted m-0">Nothing is before the assembly tonight. Move something below.</p>
          ) : (
            <div className="grid gap-3">
              {v.open.map((p) => (
                <OpenProposal key={p.id} id={id} p={p} clock={v.clock} society={society.data} me={me} names={names} />
              ))}
            </div>
          )}
        </Card>

        <Two>
          <Card
            title="Move a proposal"
            icon="page"
            testId="move-proposal"
            subtitle={
              <>
                {c.proposers === "office_holders" ? "Office-holders move proposals here." : "Any citizen may move one."} You hold {mine} of the {v.open_per_citizen} open proposals a citizen may have at once.
              </>
            }
          >
            <BallotBuilder id={id} caps={c} citizens={people} offices={os} />
          </Card>

          <Card title="Offices" icon="office" testId="offices" subtitle="Who sits, for how long, and the election open for each.">
            {os.length === 0 ? <p className="text-muted m-0">This society has no offices.</p> : null}
            <div className="grid gap-3">
              {os.map((o) => (
                <Office key={o.kind} id={id} o={o} clock={offices.data!.clock} society={society.data} me={me} />
              ))}
            </div>
            <p className="text-muted mt-3 mb-0 text-xs">Everything said on a floor is logged and may be published, pseudonymously, as research data.</p>
          </Card>
        </Two>

        <Card title="Decided this epoch" icon="archive" testId="closed-proposals" subtitle="Every motion that closed, newest first, with what it did.">
          <ClosedLedger id={id} closed={v.closed} names={names} />
        </Card>
      </Stack>
    </div>
  );
}
