// The archive (GDD 11.5, S1.15; docs/style.md §10 Archive): every epoch this
// society has finished, each with the summary the engine froze as it ended
// and the closing statements its citizens left. The Verdict says how the
// last one ended; each epoch is a card with its closing figures at a glance,
// the rest as facts, where everyone stood, and what people said. A citizen
// reads the same page inside the society and, while the window is open,
// leaves one statement of their own. The public page is the same record
// without the box: an ending is meant to be read.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { credits } from "../api/client";
import { useArchives, useClosingStatement, usePublicArchives, usePublicStats, type ArchiveView } from "../api/civic";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack } from "../components/Card";
import { Countdown } from "../components/Countdown";
import { FactList } from "../components/FactList";
import { Figure, Figures } from "../components/Figure";
import { TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { storeStockFigure } from "../components/StatTiles";
import { Verdict } from "../components/Verdict";
import { archiveVerdict, fedTone, needTone } from "../lib/verdict";
import { epochEndingText } from "../lib/when";

type Aggregates = Record<string, unknown>;
type Standing = { citizen: number; handle: string; kind: string; dormant: boolean; net_worth: number; self_made: number; honors?: number };
type Clock = { cycle: number; epoch: number; epoch_ending?: number | null; epoch_ended: boolean };

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

/** The closing statement: four figures at a glance, the rest as facts. */
function Summary({ a, money, t }: { a: Aggregates; money: boolean; t: (k: string) => string }) {
  const fed = typeof a.need_fulfillment_rate === "number" ? a.need_fulfillment_rate : null;
  const store = money ? null : storeStockFigure(a);
  const hardship = typeof a.hardship_count === "number" ? a.hardship_count : null;
  const wellbeing = typeof a.median_wellbeing === "number" ? a.median_wellbeing : null;
  const population = typeof a.population === "number" ? a.population : 0;
  return (
    <div data-testid="archive-summary">
      <Figures>
        <Figure label="Need fulfillment" value={fed === null ? "—" : (fed * 100).toFixed(0)} unit={fed === null ? undefined : "%"} tone={fedTone(fed)} status="Citizen-days never under the line." />
        <Figure
          label="In hardship"
          value={hardship === null ? "—" : String(hardship)}
          tone={hardship === null || hardship === 0 ? "good" : hardship * 10 >= population ? "crit" : "attn"}
          status="On the last day."
        />
        {money ? (
          <Figure label={t("society_stat")} value={num(a.price_index, 2)} status="Reference basket, Food = 1." />
        ) : store ? (
          <Figure label={t("society_stat")} value={store.value} unit="food" status={store.status} />
        ) : null}
        <Figure label="Median wellbeing" value={wellbeing === null ? "—" : String(Math.round(wellbeing))} unit={wellbeing === null ? undefined : "/ 100"} tone={wellbeing === null ? "good" : needTone(wellbeing)} status="The middle citizen." />
      </Figures>
      <FactList
        className="mt-4"
        items={[
          { key: "citizens", label: "Citizens", value: num(a.population), gloss: `${num(a.active_humans)} people, ${num(a.householders)} householders` },
          { key: "firms", label: "Firms", value: num(a.firm_count), gloss: `${num(a.unemployed)} without work` },
          { key: "output", label: "Real output", value: num(a.real_output, 1), gloss: "the last day, in basket units" },
          { key: "gini", label: "Consumption Gini", value: num(a.consumption_gini, 2), gloss: "of what is eaten, worn and housed" },
        ]}
      />
    </div>
  );
}

/**
 * Where everyone stood. The engine ranks by net worth (S1.15); where no money
 * exists that column is 0 cr for everyone and says nothing, so a money-less
 * society's table is the honors on the record, in the engine's order, and
 * the rank is not shown (S2.11, Q157: the archive carries no contribution
 * record yet).
 */
function Standings({ rows, money }: { rows: Standing[]; money: boolean }) {
  const top = rows.slice(0, 10);
  const rest = rows.length - top.length;
  return (
    <div>
      <table className="w-full border-collapse text-[15px]" data-testid="archive-standings">
        <thead>
          <tr>
            <th className={TH}>Citizen</th>
            {money ? (
              <>
                <th className={TH_NUM}>Net worth</th>
                <th className={TH_NUM}>Self-made</th>
              </>
            ) : (
              <th className={TH_NUM}>Honors</th>
            )}
          </tr>
        </thead>
        <tbody>
          {top.map((s, i) => (
            <tr key={s.citizen} className="hover:bg-surface-2">
              <td className={TD}>
                {money ? <span className="text-muted mr-2 tabular-nums">{i + 1}.</span> : null}
                {s.handle}
                {s.kind === "householder" ? <span className="text-muted text-sm"> householder</span> : null}
                {s.dormant ? <span className="text-muted text-sm"> away</span> : null}
              </td>
              {money ? (
                <>
                  <td className={TD_NUM}>{credits(s.net_worth)} cr</td>
                  <td className={TD_NUM}>{credits(s.self_made)} cr</td>
                </>
              ) : (
                <td className={TD_NUM}>{(s.honors ?? 0) > 0 ? s.honors : <span className="text-muted">—</span>}</td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
      {rest > 0 ? <p className="text-muted mt-2 mb-0 text-sm">and {rest} more{money ? ", ranked the same way" : ""}.</p> : null}
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
      className="grid gap-2"
      data-testid="closing-statement"
      onSubmit={(e) => {
        e.preventDefault();
        setError(null);
        say.mutate(text.trim(), { onError: (err) => setError(err instanceof Error ? err.message : String(err)) });
      }}
    >
      <label className="grid gap-1.5">
        <span className="text-sm font-bold">{archive.mine ? "Your closing statement" : "Leave a closing statement"}</span>
        <span className="text-muted text-sm">
          {archive.mine ? "Change it if you like; " : "It goes into the archive; "}
          <Countdown at={archive.closes_at} label="the window closes in" />
        </span>
        <textarea
          className="border-line bg-surface text-ink focus:border-accent focus:ring-accent-soft min-h-24 rounded-sm border px-3 py-2 focus:ring-2 focus:outline-none"
          maxLength={2000}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="What this epoch was, from where you stood."
        />
      </label>
      <ButtonRow>
        <Button type="submit" variant="primary" disabled={say.isPending || text.trim() === "" || !changed}>
          {archive.mine ? "Change it" : "Leave it"}
        </Button>
        {say.isSuccess && !changed ? <span className="text-good text-sm">Kept.</span> : null}
        {error ? (
          <span className="text-crit text-sm" role="alert">
            {error}
          </span>
        ) : null}
      </ButtonRow>
    </form>
  );
}

export function ArchiveCard({ id, archive, canWrite, money, t }: { id: number; archive: ArchiveView; canWrite: boolean; money: boolean; t: (k: string) => string }) {
  const summary = archive.summary as { aggregates?: Aggregates; standings?: Standing[] };
  return (
    <Card
      title={`Epoch ${archive.epoch}`}
      icon="archive"
      testId={`archive-${archive.epoch}`}
      aside={archive.open ? <Pill tone="info">statements open</Pill> : null}
      subtitle={
        <>
          {endingOf(archive)} · ended <span className="tabular-nums">{when(archive.ended_at)}</span>
        </>
      }
    >
      {summary.aggregates ? <Summary a={summary.aggregates} money={money} t={t} /> : <p className="text-muted m-0">No summary was kept.</p>}
      <div className="mt-5 grid gap-5 md:grid-cols-2">
        <div>
          <h3 className="text-muted mb-1.5 text-xs font-bold tracking-caps uppercase">Where everyone stood</h3>
          {summary.standings?.length ? <Standings rows={summary.standings} money={money} /> : <p className="text-muted m-0">Nobody was counted.</p>}
        </div>
        <div className="grid content-start gap-3">
          <h3 className="text-muted m-0 text-xs font-bold tracking-caps uppercase">Closing statements</h3>
          {archive.closing_statements.length === 0 ? (
            <p className="text-muted m-0">{archive.open ? "Nobody has spoken yet." : "Nobody left a word."}</p>
          ) : (
            <ul className="m-0 grid list-none gap-3 p-0" data-testid="closing-statements">
              {archive.closing_statements.map((s) => (
                <li key={s.citizen} className="border-accent border-l-2 pl-3">
                  <p className="m-0 whitespace-pre-wrap">{s.text}</p>
                  <p className="text-muted mt-1 mb-0 text-sm">
                    <b className="text-ink">{s.handle}</b> · {when(s.written_at)}
                  </p>
                </li>
              ))}
            </ul>
          )}
          {archive.open ? (
            canWrite ? (
              <StatementBox id={id} archive={archive} />
            ) : (
              <p className="text-muted m-0 text-sm">
                Citizens may still leave a word: <Countdown at={archive.closes_at} label="the window closes in" />
              </p>
            )
          ) : null}
        </div>
      </div>
    </Card>
  );
}

function ArchiveList({ id, archives, clock, canWrite, money, t }: { id: number; archives: ArchiveView[]; clock: Clock; canWrite: boolean; money: boolean; t: (k: string) => string }) {
  const ending = epochEndingText(clock);
  const newest = archives[archives.length - 1] ?? null;
  const verdict = archiveVerdict({
    latest: newest ? { epoch: newest.epoch, reason: newest.reason, final_cycle: newest.final_cycle, open: newest.open } : null,
    clock,
  });
  return (
    <Stack>
      <Card title="The record" icon="archive">
        <Verdict parts={verdict} />
        <p className="text-muted m-0">
          {archives.length === 0
            ? (ending ?? "What each epoch leaves behind: the numbers as they closed, and what people said.")
            : (ending ?? `${archives.length === 1 ? "One epoch has" : `${archives.length} epochs have`} closed here, the newest first.`)}
        </p>
      </Card>
      {[...archives].reverse().map((a) => (
        <ArchiveCard key={a.epoch} id={id} archive={a} canWrite={canWrite} money={money} t={t} />
      ))}
    </Stack>
  );
}

/** Inside the society: the citizen sees their own statement and the box while the window is open. */
export function SocietyArchives({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const archives = useArchives(id);
  const home = useHome(id);
  const caps = useCapabilities(id);
  if (archives.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
  if (archives.error) return <p className="text-crit">Could not load: {String(archives.error)}</p>;
  const v = archives.data!;
  return (
    <div>
      <PageHeader
        title="Archive"
        meta={
          <span>
            Epoch <b>{v.clock.epoch}</b> · Day <b>{v.clock.cycle}</b>
          </span>
        }
      />
      <ArchiveList id={id} archives={v.archives} clock={v.clock} canWrite={home.data !== undefined} money={caps.data?.money !== false} t={t} />
    </div>
  );
}

/** For anyone: the same record, without the box. */
export function PublicArchives({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const archives = usePublicArchives(id);
  const stats = usePublicStats(id);
  if (archives.isPending || stats.isPending) return <p className="text-muted">Loading.</p>;
  if (archives.error) return <p className="text-crit">Could not load: {String(archives.error)}</p>;
  const v = archives.data!;
  return (
    <div>
      <PageHeader
        title="Archive"
        meta={
          <>
            <Link to="/public/s/$id" params={{ id: String(id) }}>
              Society
            </Link>
            <span>
              Epoch <b>{v.clock.epoch}</b> · Day <b>{v.clock.cycle}</b>
            </span>
          </>
        }
      />
      <ArchiveList id={id} archives={v.archives} clock={v.clock} canWrite={false} money={stats.data?.money !== false} t={t} />
    </div>
  );
}
