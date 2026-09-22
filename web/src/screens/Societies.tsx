// The landing (docs/style.md §10 Societies): the societies on this server,
// one card each with its clock and its people, and where you stand in it —
// your handle, or Join. Signed out, it is the one sentence about the game
// and the way in.

import { Link } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useMe, useSocieties } from "../api/hooks";
import { ButtonLink } from "../components/Button";
import { Card } from "../components/Card";
import { Countdown } from "../components/Countdown";
import { PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
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
      <div className="mx-auto max-w-md pt-6 sm:pt-12">
        <Card title="Isms" icon="globe">
          <p className="mt-0 text-lg">Societies that run on one economic system each. You live in one; the economy is the game.</p>
          <p className="text-muted mt-0">
            Anyone may read a society from the <Link to="/public">public pages</Link>; citizens act.
          </p>
          <ButtonLink to="/login" variant="primary">
            Sign in
          </ButtonLink>
        </Card>
      </div>
    );
  }
  if (societies.error || me.error) {
    return <p className="text-crit">Could not load: {String(societies.error ?? me.error)}</p>;
  }
  const mine = new Map(me.data!.citizenships.map((c) => [c.society_id, c]));
  return (
    <div>
      <PageHeader
        title="Societies"
        meta={
          <>
            <span>{me.data!.account.email}</span>
            <Link to="/profile">Profile</Link>
            {me.data!.account.operator ? <Link to="/admin">Operator</Link> : null}
          </>
        }
      />
      <ul className="m-0 grid list-none gap-4 p-0" data-testid="society-list">
        {societies.data!.map((s) => {
          const c = mine.get(s.id);
          return (
            <li key={s.id}>
              <Card
                title={
                  <Link to="/s/$id" params={{ id: String(s.id) }} className="text-ink hover:text-accent">
                    {s.display}
                  </Link>
                }
                icon="globe"
                aside={
                  <span className="flex flex-wrap items-center gap-2 text-sm font-normal">
                    <span className="text-muted">{s.name}</span>
                    {s.class === "lab" ? (
                      <span title="A lab society: synthetic players may play here; it is not public.">
                        <Pill tone="info">lab</Pill>
                      </span>
                    ) : null}
                  </span>
                }
                subtitle={
                  <>
                    Epoch {s.clock.epoch}, Day {s.clock.cycle}, {hourOfClock(s.clock)} · {s.population} citizens, {s.active_humans} people
                  </>
                }
              >
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <Countdown at={s.next_tick_at} label="next hour" />
                  {c ? (
                    <span>
                      you are <b>{c.handle}</b>
                    </span>
                  ) : (
                    <Link to="/s/$id" params={{ id: String(s.id) }}>
                      Join
                    </Link>
                  )}
                </div>
              </Card>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
