// The society shell (TDD 11, GDD 15): the nav is built from capabilities and
// labelled from the lexicon, so the absence of a widget is a design
// statement. The index route is Home (S1.8), which onboards a non-citizen.
// Sections with a screen are links (Work and the plan since S1.9); the rest
// are labels until their card lands.

import { Link, Outlet } from "@tanstack/react-router";
import { useCapabilities, useLexicon, useSociety, useStream } from "../api/hooks";
import { Countdown } from "../components/Countdown";
import { Home } from "./Home";

const SCREENS: Record<string, "/s/$id" | "/s/$id/work" | "/s/$id/plan" | "/s/$id/market"> = {
  "": "/s/$id",
  work: "/s/$id/work",
  plan: "/s/$id/plan",
  market: "/s/$id/market",
};

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
  const nav: { to: string; label: string; built?: boolean }[] = [
    { to: "", label: t("home_title"), built: true },
    { to: "work", label: t("work_screen"), built: true },
    { to: "plan", label: t("plan"), built: true },
    ...(c.order_books ? [{ to: "market", label: t("store"), built: true }] : []),
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
        {nav.map((n) =>
          n.built ? (
            <Link
              key={n.to}
              to={SCREENS[n.to] ?? "/s/$id"}
              params={{ id: String(id) }}
              className="text-muted"
              activeOptions={{ exact: n.to === "" }}
              activeProps={{ className: "text-ink" }}
            >
              {n.label}
            </Link>
          ) : (
            <span key={n.to} className="text-muted" title="Not built yet">
              {n.label}
            </span>
          ),
        )}
      </nav>
      <main className="mt-6">
        <Outlet />
      </main>
    </div>
  );
}

export function SocietyHome({ id }: { id: number }) {
  return <Home id={id} />;
}
