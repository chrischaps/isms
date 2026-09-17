// The landing: the societies on this server, and where you stand in each.

import { Link } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useMe, useSocieties } from "../api/hooks";
import { Countdown } from "../components/Countdown";
import { hourOfClock } from "../lib/when";

export function Societies() {
  const me = useMe();
  const societies = useSocieties();

  if (me.isPending || societies.isPending) {
    return <p className="text-muted">Loading.</p>;
  }
  const signedOut =
    (me.error instanceof ApiError && me.error.status === 401) ||
    (societies.error instanceof ApiError && societies.error.status === 401);
  if (signedOut) {
    return (
      <section>
        <h1 className="text-3xl">Isms</h1>
        <p className="mt-3 max-w-prose">
          Societies that run on one economic system each. You live in one; the economy is the game.
        </p>
        <p className="mt-6">
          <Link to="/login" className="bg-ink text-paper rounded-sm px-3 py-1">
            Sign in
          </Link>
        </p>
      </section>
    );
  }
  if (societies.error || me.error) {
    return <p className="text-bad">Could not load: {String(societies.error ?? me.error)}</p>;
  }
  const mine = new Map(me.data!.citizenships.map((c) => [c.society_id, c]));
  return (
    <section>
      <header className="flex items-baseline justify-between">
        <h1 className="text-3xl">Societies</h1>
        <span className="text-muted text-sm">{me.data!.account.email}</span>
      </header>
      <ul className="mt-6 flex flex-col gap-4" data-testid="society-list">
        {societies.data!.map((s) => {
          const c = mine.get(s.id);
          return (
            <li key={s.id} className="rule flex flex-wrap items-baseline justify-between gap-2 pt-4">
              <div>
                <Link to="/s/$id" params={{ id: String(s.id) }} className="text-xl">
                  {s.display}
                </Link>
                <span className="text-muted ml-2 text-sm">{s.name}</span>
                <div className="text-muted text-sm">
                  Epoch {s.clock.epoch}, Day {s.clock.cycle}, {hourOfClock(s.clock)};{" "}
                  {s.population} citizens, {s.active_humans} people
                </div>
              </div>
              <div className="flex items-baseline gap-4">
                <Countdown at={s.next_tick_at} label="next hour" />
                {c ? (
                  <span className="text-sm">
                    you are <span className="font-mono">{c.handle}</span>
                  </span>
                ) : (
                  <Link to="/s/$id" params={{ id: String(s.id) }} className="text-sm">
                    Join
                  </Link>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
