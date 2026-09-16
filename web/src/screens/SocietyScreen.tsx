// Society (GDD 10, 14.1; TDD 13; S1.13): the stats this society keeps
// (the subset its capabilities select), the Chronicle read by cycle, the
// scoreboard its constitution names, the citizens with their flags, and
// the published householder script. Every headline links to its event.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { credits } from "../api/client";
import { useCapabilities, useLexicon, useSociety } from "../api/hooks";
import { useChronicle, useCitizens, useHouseholders, useScoreboard, useStats, type HeadlineView } from "../api/civic";
import { Markdown } from "../lib/markdown";

type Aggregates = Record<string, unknown>;

function num(v: unknown, digits = 0): string {
  return typeof v === "number" ? v.toFixed(digits) : "—";
}

export function Stat({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <div className="flex flex-col">
      <span className="text-muted text-xs uppercase tracking-wide">{label}</span>
      <span className="num text-lg">{value}</span>
      {note ? <span className="text-muted text-xs">{note}</span> : null}
    </div>
  );
}

/** The dashboard's tiles, chosen by capability: money systems get prices and
 *  wages, credit systems the credit outstanding, everyone the needs figures. */
export function StatTiles({
  stats,
  money,
  credit,
  orgs,
  t,
}: {
  stats: { live: { population: number; active_humans: number; unemployed: number; price_index?: number | null }; firm_count: number; credit_outstanding: number; last_cycle?: Aggregates | null };
  money: boolean;
  credit: boolean;
  orgs: boolean;
  t: (k: string) => string;
}) {
  const a = stats.last_cycle ?? {};
  const live = stats.live;
  return (
    <div className="grid grid-cols-2 gap-4 md:grid-cols-4" data-testid="stat-tiles">
      <Stat label="Citizens" value={`${live.population}`} note={`${live.active_humans} people here`} />
      <Stat label="Without work" value={`${live.unemployed}`} />
      {money ? <Stat label={t("society_stat")} value={num(live.price_index ?? a.price_index, 2)} note="reference basket, Food = 1" /> : null}
      {money ? <Stat label="Mean cycle wage" value={typeof a.mean_cycle_wage === "number" ? `${a.mean_cycle_wage.toFixed(2)} cr` : "—"} note="last cycle, those paid anything" /> : null}
      {orgs ? <Stat label="Firms" value={`${stats.firm_count}`} /> : null}
      {credit ? <Stat label="Credit outstanding" value={`${credits(stats.credit_outstanding)} cr`} /> : null}
      <Stat label="Need fulfillment" value={typeof a.need_fulfillment_rate === "number" ? `${(a.need_fulfillment_rate * 100).toFixed(0)}%` : "—"} note="citizen-cycles never under the line" />
      <Stat label="In hardship" value={num(a.hardship_count)} note="at last cycle end" />
      <Stat label="Median wellbeing" value={num(a.median_wellbeing)} note="of 100" />
      <Stat label="Consumption Gini" value={num(a.consumption_gini, 2)} note="of what is eaten, worn and housed; never of wealth" />
    </div>
  );
}

export function ChronicleReader({
  id,
  cycle,
  setCycle,
  current,
  headlines,
  pending,
  link,
}: {
  id: number;
  cycle: number;
  setCycle: (c: number) => void;
  current: number;
  headlines: HeadlineView[];
  pending: boolean;
  link: boolean;
}) {
  return (
    <div data-testid="chronicle">
      <div className="flex items-baseline gap-3 text-sm">
        <button type="button" disabled={cycle <= 1} className="text-muted underline disabled:opacity-40" onClick={() => setCycle(cycle - 1)}>
          earlier
        </button>
        <span className="num">cycle {cycle}</span>
        <button type="button" disabled={cycle >= current} className="text-muted underline disabled:opacity-40" onClick={() => setCycle(cycle + 1)}>
          later
        </button>
      </div>
      {pending ? (
        <p className="text-muted mt-2 text-sm">Loading.</p>
      ) : headlines.length === 0 ? (
        <p className="text-muted mt-2 text-sm">Nothing to report that cycle.</p>
      ) : (
        <ol className="mt-2 flex flex-col gap-1 text-sm">
          {headlines.map((h) => (
            <li key={h.seq} className="flex gap-3">
              <span className="num text-muted w-10 shrink-0">t{h.tick}</span>
              {link ? (
                <Link to="/s/$id/events/$seq" params={{ id: String(id), seq: String(h.seq) }} className="underline decoration-dotted">
                  {h.text}
                </Link>
              ) : (
                <span>{h.text}</span>
              )}
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

export function SocietyScreen({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const society = useSociety(id);
  const stats = useStats(id);
  const [cycle, setCycle] = useState<number | null>(null);
  const chronicle = useChronicle(id, cycle);
  const scoreboard = useScoreboard(id);
  const citizens = useCitizens(id);
  const script = useHouseholders(id);
  const [showScript, setShowScript] = useState(false);

  if (caps.isPending || stats.isPending || society.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error || stats.error || society.error) return <p className="text-bad">Could not load: {String(caps.error ?? stats.error ?? society.error)}</p>;
  const c = caps.data!;
  const s = stats.data!;
  const current = society.data!.clock.cycle;
  const shown = chronicle.data?.cycle ?? cycle ?? current;

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{society.data!.display}</h2>
        <Link to="/public/s/$id" params={{ id: String(id) }} className="text-muted text-sm underline">
          the public view
        </Link>
      </header>

      <section>
        <h3 className="text-lg">The numbers this society keeps</h3>
        <div className="mt-3">
          <StatTiles stats={s} money={c.money} credit={c.contracts.includes("credit")} orgs={c.org_kinds.length > 0} t={t} />
        </div>
      </section>

      <section className="grid gap-8 md:grid-cols-2">
        <div>
          <h3 className="text-lg">{t("chronicle")}</h3>
          <div className="mt-2">
            <ChronicleReader id={id} cycle={shown} setCycle={setCycle} current={current} headlines={chronicle.data?.headlines ?? []} pending={chronicle.isPending} link />
          </div>
        </div>
        <div>
          <h3 className="text-lg">{t("scoreboard")}</h3>
          {scoreboard.data && scoreboard.data.rows.length > 0 ? (
            <table className="mt-2 w-full text-sm" data-testid="scoreboard">
              <thead className="text-muted text-left text-xs uppercase tracking-wide">
                <tr>
                  <th className="py-1 font-normal">Citizen</th>
                  <th className="py-1 text-right font-normal">Net worth</th>
                  <th className="py-1 text-right font-normal">Self-made</th>
                  <th className="py-1 font-normal">Firms</th>
                </tr>
              </thead>
              <tbody>
                {scoreboard.data.rows.slice(0, 20).map((r) => (
                  <tr key={r.citizen} className="rule">
                    <td className="py-1 pr-2">{r.handle}</td>
                    <td className="num py-1 pr-2 text-right">{credits(r.net_worth)}</td>
                    <td className={`num py-1 pr-2 text-right ${r.self_made < 0 ? "text-bad" : ""}`}>{credits(r.self_made)}</td>
                    <td className="py-1 text-xs">{r.firms.map((f) => `${f.name} (${credits(f.book_value)})`).join(", ")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <p className="text-muted mt-2 text-sm">{scoreboard.isPending ? "Loading." : "No scoreboard here: this society does not keep one."}</p>
          )}
          <p className="text-muted mt-2 text-xs">What "doing well" means is the constitution's claim, not ours; other societies keep other scores.</p>
        </div>
      </section>

      <section>
        <h3 className="text-lg">Citizens</h3>
        {citizens.data ? (
          <table className="mt-2 w-full text-sm" data-testid="citizens">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">Handle</th>
                <th className="py-1 font-normal">Kind</th>
                <th className="py-1 font-normal">Since</th>
                <th className="py-1 font-normal">Flags</th>
              </tr>
            </thead>
            <tbody>
              {citizens.data.citizens.map((z) => {
                const flags = Object.entries(z.flags as Record<string, boolean>)
                  .filter(([, v]) => v)
                  .map(([k]) => k.replace(/_/g, " "));
                return (
                  <tr key={z.id} className={`rule ${z.dormant ? "text-muted" : ""}`}>
                    <td className="py-1 pr-2">
                      <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `dm:${z.id}` }} className="underline decoration-dotted">
                        {z.handle}
                      </Link>
                    </td>
                    <td className="py-1 pr-2">{z.kind}{z.dormant ? ", dormant" : ""}</td>
                    <td className="num py-1 pr-2">tick {z.joined_tick}</td>
                    <td className="py-1 text-xs">{flags.join(", ")}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        ) : (
          <p className="text-muted mt-2 text-sm">Loading.</p>
        )}
        <p className="text-muted mt-2 text-xs">
          Householders follow a published script.{" "}
          <button type="button" className="underline" onClick={() => setShowScript((v) => !v)}>
            {showScript ? "Hide it" : "Read it"}
          </button>
        </p>
        {showScript ? (
          <div className="bg-paper-2 mt-2 max-w-2xl rounded-sm p-3 text-sm" data-testid="householder-script">
            {script.data ? <Markdown text={script.data.markdown} /> : <p className="text-muted">Loading.</p>}
          </div>
        ) : null}
      </section>
    </div>
  );
}
