// The spectator views (GDD 9.2, TDD 10.2; S1.13): the society list, one
// society's numbers and Chronicle, readable with no login. What a society
// keeps in public is its own choice; these show what the API publishes.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useLexicon } from "../api/hooks";
import { usePublicChronicle, usePublicSocieties, usePublicStats } from "../api/civic";
import { ChronicleReader, StatTiles } from "./SocietyScreen";

export function PublicSocieties() {
  const societies = usePublicSocieties();
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h1 className="text-2xl">Isms</h1>
        <nav className="flex gap-4 text-sm">
          <Link to="/" className="text-muted underline">
            Sign in
          </Link>
        </nav>
      </header>
      <p className="text-muted text-sm">Societies running on this server, each under its own constitution. Anyone may read; citizens act.</p>
      {societies.isPending ? (
        <p className="text-muted">Loading.</p>
      ) : societies.error ? (
        <p className="text-bad">Could not load: {String(societies.error)}</p>
      ) : (
        <ul className="flex flex-col gap-2" data-testid="public-society-list">
          {societies.data!.map((s) => (
            <li key={s.id} className="rule flex flex-wrap items-baseline justify-between gap-2 pt-2">
              <Link to="/public/s/$id" params={{ id: String(s.id) }} className="text-xl underline decoration-dotted">
                {s.display}
              </Link>
              <span className="num text-muted text-sm">
                {s.preset} · epoch {s.clock.epoch}, cycle {s.clock.cycle} · {s.population} citizens, {s.active_humans} people
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function PublicSociety({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const stats = usePublicStats(id);
  const [cycle, setCycle] = useState<number | null>(null);
  const chronicle = usePublicChronicle(id, cycle);
  if (stats.isPending) return <p className="text-muted">Loading.</p>;
  if (stats.error) return <p className="text-bad">Could not load: {String(stats.error)}</p>;
  const s = stats.data!;
  const current = s.clock.cycle;
  const shown = chronicle.data?.cycle ?? cycle ?? current;
  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <div className="flex items-baseline gap-3">
          <Link to="/public" className="text-muted text-sm">
            Societies
          </Link>
          <h1 className="text-2xl">{s.name}</h1>
          <span className="text-muted text-xs uppercase tracking-wide">{s.preset}</span>
        </div>
        <span className="num text-muted text-sm">
          epoch {s.clock.epoch} · cycle {s.clock.cycle} · tick {s.clock.tick}/{s.clock.ticks_per_cycle}
        </span>
      </header>
      <section>
        <h2 className="text-lg">The numbers it keeps</h2>
        <div className="mt-3">
          <StatTiles stats={s} money={s.money} credit={s.credit} orgs={s.orgs} t={t} />
        </div>
      </section>
      <section className="max-w-2xl">
        <h2 className="text-lg">{t("chronicle")}</h2>
        <div className="mt-2">
          <ChronicleReader id={id} cycle={shown} setCycle={setCycle} current={current} headlines={chronicle.data?.headlines ?? []} pending={chronicle.isPending} link={false} />
        </div>
      </section>
      <p className="text-muted text-sm">
        To act here,{" "}
        <Link to="/login" className="underline">
          sign in
        </Link>{" "}
        and join.
      </p>
    </div>
  );
}
