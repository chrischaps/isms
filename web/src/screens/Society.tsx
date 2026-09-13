// The society shell (TDD 11, GDD 15): the nav is built from capabilities and
// labelled from the lexicon, so the absence of a widget is a design
// statement. S1.7 mounts the shell and a first Situation summary; S1.8
// builds Onboarding and the full Home.

import { Link, Outlet } from "@tanstack/react-router";
import { ApiError, credits } from "../api/client";
import { useCapabilities, useHome, useLexicon, useSociety, useStream } from "../api/hooks";
import { Countdown } from "../components/Countdown";
import { DiffSinceLastSeen } from "../components/DiffSinceLastSeen";
import { Meter } from "../components/Meter";
import { Num } from "../components/Num";

export function SocietyShell({ id }: { id: number }) {
  const society = useSociety(id);
  const caps = useCapabilities(id);
  const { t } = useLexicon(id);
  useStream(id);
  if (society.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
  if (society.error || caps.error) {
    return <p className="text-bad">Could not load: {String(society.error ?? caps.error)}</p>;
  }
  const s = society.data!;
  const c = caps.data!;
  const nav: { to: string; label: string }[] = [
    { to: "", label: t("home_title") },
    { to: "work", label: t("work_screen") },
    ...(c.order_books ? [{ to: "market", label: t("store") }] : []),
    ...(c.org_kinds.length > 0 ? [{ to: "orgs", label: "Organizations" }] : []),
    { to: "contracts", label: "Contracts" },
    { to: "society", label: "Society" },
    { to: "talk", label: "Talk" },
  ];
  return (
    <div>
      <header className="rule flex flex-wrap items-baseline justify-between gap-2 pb-3">
        <div className="flex items-baseline gap-3">
          <Link to="/" className="text-muted text-sm">
            Societies
          </Link>
          <h1 className="text-2xl">{s.display}</h1>
          <span className="text-muted text-xs uppercase tracking-wide">{s.preset}</span>
        </div>
        <div className="num text-muted text-sm">
          epoch {s.clock.epoch} · cycle {s.clock.cycle} · tick {s.clock.tick}/{s.clock.ticks_per_cycle} ·{" "}
          <Countdown at={s.next_tick_at} label="next tick" />
        </div>
      </header>
      <nav className="mt-3 flex flex-wrap gap-4 text-sm" aria-label="Sections">
        {nav.map((n) => (
          <span key={n.to} className={n.to === "" ? "text-ink" : "text-muted"}>
            {n.label}
          </span>
        ))}
      </nav>
      <main className="mt-6">
        <Outlet />
      </main>
    </div>
  );
}

export function SocietyHome({ id }: { id: number }) {
  const home = useHome(id);
  const { t } = useLexicon(id);
  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <section>
        <p>You are not a citizen here yet.</p>
        <p className="text-muted mt-2 text-sm">Joining arrives with the next card (S1.8).</p>
      </section>
    );
  }
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  return (
    <section className="grid gap-8 md:grid-cols-2">
      <div>
        <h2 className="text-xl">{t("home_title")}</h2>
        <div className="mt-3 flex flex-col gap-2">
          <Meter label="Food" value={h.needs.food} />
          <Meter label="Shelter" value={h.needs.shelter} />
          <Meter label="Comfort" value={h.needs.comfort} />
        </div>
        <dl className="mt-4 grid grid-cols-2 gap-y-1 text-sm">
          <dt className="text-muted">{t("balance")}</dt>
          <dd className="num">
            <Num value={credits(h.household.balance)} unit="cr" />
          </dd>
          <dt className="text-muted">{t("pantry")}</dt>
          <dd className="num">
            {Object.entries(h.household.pantry)
              .map(([g, n]) => `${n} ${g}`)
              .join(", ") || "empty"}
          </dd>
          <dt className="text-muted">{t("dwelling")}</dt>
          <dd>{h.household.dwelling ? `#${h.household.dwelling.id}` : "none"}</dd>
        </dl>
      </div>
      <div>
        <h2 className="text-xl">While you were away</h2>
        <div className="mt-3">
          <DiffSinceLastSeen events={h.since_last_seen.events} t={t} me={h.citizen.id} />
        </div>
        {h.headlines.length > 0 ? (
          <>
            <h2 className="mt-6 text-xl">{t("chronicle")}</h2>
            <ul className="mt-2 text-sm">
              {h.headlines.map((hl) => (
                <li key={hl.seq}>{hl.text}</li>
              ))}
            </ul>
          </>
        ) : null}
      </div>
    </section>
  );
}
