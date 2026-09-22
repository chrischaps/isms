// The spectator views (GDD 9.2, TDD 10.2; S1.13; docs/style.md §10): the
// society list, one society's numbers and Chronicle, readable with no login.
// What a society keeps in public is its own choice; these show what the API
// publishes, on the same tiles and reader the citizen's Society screen uses.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useLexicon } from "../api/hooks";
import { usePublicChronicle, usePublicSocieties, usePublicStats } from "../api/civic";
import { Card, Stack, Two } from "../components/Card";
import { ChronicleReader } from "../components/ChronicleReader";
import { PageHeader } from "../components/PageHeader";
import { StatTiles } from "../components/StatTiles";
import { hourOfClock } from "../lib/when";

const FALLBACK: Record<string, string> = { chronicle: "Chronicle", society_stat: "Price index" };

export function PublicSocieties() {
  const societies = usePublicSocieties();
  return (
    <div>
      <PageHeader title="Isms" meta={<Link to="/">Sign in</Link>} />
      <Card title="Societies" icon="globe" subtitle="Running on this server, each under its own constitution. Anyone may read; citizens act.">
        {societies.isPending ? (
          <p className="text-muted m-0">Loading.</p>
        ) : societies.error ? (
          <p className="text-crit m-0">Could not load: {String(societies.error)}</p>
        ) : (
          <ul className="m-0 grid list-none gap-3 p-0" data-testid="public-society-list">
            {societies.data!.map((s) => (
              <li key={s.id} className="border-line flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-t pt-3 first:border-t-0 first:pt-0">
                <Link to="/public/s/$id" params={{ id: String(s.id) }} className="text-lg">
                  {s.display}
                </Link>
                <span className="text-muted text-sm tabular-nums">
                  {s.preset} · Epoch {s.clock.epoch}, Day {s.clock.cycle} · {s.population} citizens, {s.active_humans} people
                </span>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}

export function PublicSociety({ id }: { id: number }) {
  // The lexicon is served to citizens; a reader with no session gets the shell's own words (Q155).
  const lexicon = useLexicon(id);
  const t = (k: string) => (lexicon.t(k) === k ? (FALLBACK[k] ?? k) : lexicon.t(k));
  const stats = usePublicStats(id);
  const [cycle, setCycle] = useState<number | null>(null);
  const chronicle = usePublicChronicle(id, cycle);
  if (stats.isPending) return <p className="text-muted">Loading.</p>;
  if (stats.error) return <p className="text-crit">Could not load: {String(stats.error)}</p>;
  const s = stats.data!;
  const current = s.clock.cycle;
  const shown = chronicle.data?.cycle ?? cycle ?? current;
  return (
    <div>
      <PageHeader
        title={s.name}
        meta={
          <>
            <span className="tracking-caps uppercase">{s.preset}</span>
            <span className="tabular-nums">
              Epoch <b>{s.clock.epoch}</b> · Day <b>{s.clock.cycle}</b> · {hourOfClock(s.clock)}
            </span>
            <Link to="/public">Societies</Link>
            <Link to="/public/s/$id/archives" params={{ id: String(id) }}>
              Past epochs
            </Link>
          </>
        }
      />
      <Stack>
        <Card title="The numbers it keeps" icon="people">
          <StatTiles stats={s} money={s.money} credit={s.credit} orgs={s.orgs} t={t} />
        </Card>
        <Two>
          <Card title={t("chronicle")} icon="page" subtitle="What the society says about itself, day by day.">
            <ChronicleReader id={id} cycle={shown} setCycle={setCycle} current={current} headlines={chronicle.data?.headlines ?? []} pending={chronicle.isPending} link={false} />
          </Card>
          <Card title="To act here" icon="person">
            <p className="m-0">
              <Link to="/login">Sign in</Link> and join. A citizen works, trades and votes; a reader only watches.
            </p>
          </Card>
        </Two>
      </Stack>
    </div>
  );
}
