// The archive (GDD 11.5, S1.15): every epoch this society has finished, each
// with the summary the engine froze as it ended and the closing statements
// its citizens left. A citizen reads the same page inside the society and,
// while the window is open, leaves one statement of their own. The public
// page is the same record without the box: an ending is meant to be read.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { credits } from "../api/client";
import { useArchives, useClosingStatement, usePublicArchives, type ArchiveView } from "../api/civic";
import { useHome, useLexicon } from "../api/hooks";
import { Countdown } from "../components/Countdown";
import { epochEndingText } from "../lib/when";
import { Stat } from "./SocietyScreen";

type Aggregates = Record<string, unknown>;
type Standing = { citizen: number; handle: string; kind: string; dormant: boolean; net_worth: number; self_made: number };

/** How the epoch ended, as a clause after "Epoch N". */
export function endingOf(a: { reason: string; final_cycle: number }): string {
  switch (a.reason) {
    case "scheduled":
      return `ran its course: ${a.final_cycle} days`;
    case "collapse":
      return `collapsed on Day ${a.final_cycle}`;
    default:
      return `ended by the operator on Day ${a.final_cycle}`;
  }
}

function num(v: unknown, digits = 0): string {
  return typeof v === "number" ? v.toFixed(digits) : "—";
}

function when(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

function Summary({ a, t }: { a: Aggregates; t: (k: string) => string }) {
  return (
    <div className="grid grid-cols-2 gap-4 md:grid-cols-4" data-testid="archive-summary">
      <Stat label="Citizens" value={num(a.population)} note={`${num(a.active_humans)} people, ${num(a.householders)} householders`} />
      <Stat label="Firms" value={num(a.firm_count)} note={`${num(a.unemployed)} without work`} />
      <Stat label={t("society_stat")} value={num(a.price_index, 2)} note="reference basket, Food = 1" />
      <Stat label="Real output" value={num(a.real_output, 1)} note="the last day, in basket units" />
      <Stat label="Need fulfillment" value={typeof a.need_fulfillment_rate === "number" ? `${(a.need_fulfillment_rate * 100).toFixed(0)}%` : "—"} note="citizen-days never under the line" />
      <Stat label="In hardship" value={num(a.hardship_count)} note="on the last day" />
      <Stat label="Median wellbeing" value={num(a.median_wellbeing)} note="of 100" />
      <Stat label="Consumption Gini" value={num(a.consumption_gini, 2)} note="of what is eaten, worn and housed" />
    </div>
  );
}

function Standings({ rows }: { rows: Standing[] }) {
  const top = rows.slice(0, 10);
  const rest = rows.length - top.length;
  return (
    <div>
      <table className="w-full text-sm" data-testid="archive-standings">
        <thead className="text-muted text-left text-xs uppercase tracking-wide">
          <tr>
            <th className="py-1 font-normal">#</th>
            <th className="py-1 font-normal">Citizen</th>
            <th className="py-1 text-right font-normal">Net worth</th>
            <th className="py-1 text-right font-normal">Self-made</th>
          </tr>
        </thead>
        <tbody>
          {top.map((s, i) => (
            <tr key={s.citizen} className="rule">
              <td className="num py-1 pr-2">{i + 1}</td>
              <td className="py-1 pr-2">
                {s.handle}
                {s.kind === "householder" ? <span className="text-muted text-xs"> householder</span> : null}
                {s.dormant ? <span className="text-muted text-xs"> away</span> : null}
              </td>
              <td className="num py-1 text-right">{credits(s.net_worth)} cr</td>
              <td className="num py-1 text-right">{credits(s.self_made)} cr</td>
            </tr>
          ))}
        </tbody>
      </table>
      {rest > 0 ? <p className="text-muted mt-1 text-xs">and {rest} more, ranked the same way.</p> : null}
    </div>
  );
}

function StatementBox({ id, archive }: { id: number; archive: ArchiveView }) {
  const say = useClosingStatement(id);
  const [text, setText] = useState(archive.mine ?? "");
  const [error, setError] = useState<string | null>(null);
  const changed = text.trim() !== (archive.mine ?? "");
  return (
    <form
      className="flex max-w-2xl flex-col gap-2"
      data-testid="closing-statement"
      onSubmit={(e) => {
        e.preventDefault();
        setError(null);
        say.mutate(text.trim(), { onError: (err) => setError(err instanceof Error ? err.message : String(err)) });
      }}
    >
      <label className="flex flex-col gap-1 text-sm">
        <span>
          {archive.mine ? "Your closing statement. Change it if you like; " : "Leave a closing statement. It goes into the archive; "}
          <Countdown at={archive.closes_at} label="the window closes in" />
        </span>
        <textarea
          className="border-line min-h-24 rounded-sm border px-2 py-1"
          maxLength={2000}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="What this epoch was, from where you stood."
        />
      </label>
      <div className="flex items-center gap-3">
        <button type="submit" disabled={say.isPending || text.trim() === "" || !changed} className="bg-ink text-paper rounded-sm px-3 py-1 text-sm disabled:opacity-50">
          {archive.mine ? "Change it" : "Leave it"}
        </button>
        {say.isSuccess && !changed ? <span className="text-good text-sm">Kept.</span> : null}
        {error ? (
          <span className="text-bad text-sm" role="alert">
            {error}
          </span>
        ) : null}
      </div>
    </form>
  );
}

export function ArchiveCard({ id, archive, canWrite, t }: { id: number; archive: ArchiveView; canWrite: boolean; t: (k: string) => string }) {
  const summary = archive.summary as { aggregates?: Aggregates; standings?: Standing[] };
  return (
    <article className="flex flex-col gap-5" data-testid={`archive-${archive.epoch}`}>
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h2 className="text-xl">
          Epoch {archive.epoch} <span className="text-muted text-base">{endingOf(archive)}</span>
        </h2>
        <span className="text-muted num text-xs">ended {when(archive.ended_at)}</span>
      </header>
      {summary.aggregates ? <Summary a={summary.aggregates} t={t} /> : <p className="text-muted text-sm">No summary was kept.</p>}
      <section className="grid gap-6 md:grid-cols-2">
        <div>
          <h3 className="text-muted mb-1 text-xs uppercase tracking-wide">Where everyone stood</h3>
          {summary.standings?.length ? <Standings rows={summary.standings} /> : <p className="text-muted text-sm">Nobody was counted.</p>}
        </div>
        <div className="flex flex-col gap-3">
          <h3 className="text-muted text-xs uppercase tracking-wide">Closing statements</h3>
          {archive.closing_statements.length === 0 ? (
            <p className="text-muted text-sm">{archive.open ? "Nobody has spoken yet." : "Nobody left a word."}</p>
          ) : (
            <ul className="flex flex-col gap-3" data-testid="closing-statements">
              {archive.closing_statements.map((s) => (
                <li key={s.citizen} className="border-line border-l-2 pl-3">
                  <p className="whitespace-pre-wrap text-sm">{s.text}</p>
                  <p className="text-muted mt-1 text-xs">
                    {s.handle} · {when(s.written_at)}
                  </p>
                </li>
              ))}
            </ul>
          )}
          {archive.open ? (
            canWrite ? (
              <StatementBox id={id} archive={archive} />
            ) : (
              <p className="text-muted text-sm">
                Citizens may still leave a word: <Countdown at={archive.closes_at} label="the window closes in" />
              </p>
            )
          ) : null}
        </div>
      </section>
    </article>
  );
}

function ArchiveList({ id, archives, clock, canWrite, t }: { id: number; archives: ArchiveView[]; clock: { cycle: number; epoch: number; epoch_ending?: number | null; epoch_ended: boolean }; canWrite: boolean; t: (k: string) => string }) {
  const ending = epochEndingText(clock);
  return (
    <div className="flex flex-col gap-10">
      {archives.length === 0 ? (
        <p className="text-muted">No epoch has closed here yet. {ending ?? `This is epoch ${clock.epoch}, Day ${clock.cycle}.`}</p>
      ) : (
        [...archives].reverse().map((a) => <ArchiveCard key={a.epoch} id={id} archive={a} canWrite={canWrite} t={t} />)
      )}
      {archives.length > 0 && ending ? <p className="text-muted text-sm">{ending}</p> : null}
    </div>
  );
}

/** Inside the society: the citizen sees their own statement and the box while the window is open. */
export function SocietyArchives({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const archives = useArchives(id);
  const home = useHome(id);
  if (archives.isPending) return <p className="text-muted">Loading.</p>;
  if (archives.error) return <p className="text-bad">Could not load: {String(archives.error)}</p>;
  const v = archives.data!;
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">Archive</h2>
        <p className="text-muted text-sm">What each epoch left behind: the numbers as they closed, and what people said.</p>
      </header>
      <ArchiveList id={id} archives={v.archives} clock={v.clock} canWrite={home.data !== undefined} t={t} />
    </div>
  );
}

/** For anyone: the same record, without the box. */
export function PublicArchives({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const archives = usePublicArchives(id);
  if (archives.isPending) return <p className="text-muted">Loading.</p>;
  if (archives.error) return <p className="text-bad">Could not load: {String(archives.error)}</p>;
  const v = archives.data!;
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <div className="flex items-baseline gap-3">
          <Link to="/public/s/$id" params={{ id: String(id) }} className="text-muted text-sm">
            Society
          </Link>
          <h1 className="text-2xl">Archive</h1>
        </div>
        <span className="num text-muted text-sm">
          Epoch {v.clock.epoch} · Day {v.clock.cycle}
        </span>
      </header>
      <ArchiveList id={id} archives={v.archives} clock={v.clock} canWrite={false} t={t} />
    </div>
  );
}
