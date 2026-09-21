// The society shell (TDD 11, GDD 15): the nav is built from capabilities and
// labelled from the lexicon, so the absence of a widget is a design
// statement. The index route is Home (S1.8), which onboards a non-citizen.
// Sections with a screen are links (Work and the plan since S1.9); the rest
// are labels until their card lands.

import { useEffect } from "react";
import { Link, Outlet } from "@tanstack/react-router";
import { credits, setHumanizer } from "../api/client";
import { useCapabilities, useHome, useLexicon, useMe, useSociety, useStream } from "../api/hooks";
import { WorldClock } from "../components/WorldClock";
import { useNames } from "../lib/names";
import { Home } from "./Home";

const SCREENS: Record<
  string,
  | "/s/$id"
  | "/s/$id/work"
  | "/s/$id/plan"
  | "/s/$id/market"
  | "/s/$id/orgs"
  | "/s/$id/contracts"
  | "/s/$id/store"
  | "/s/$id/ledger"
  | "/s/$id/assembly"
  | "/s/$id/society"
  | "/s/$id/talk"
  | "/s/$id/archives"
> = {
  "": "/s/$id",
  work: "/s/$id/work",
  plan: "/s/$id/plan",
  market: "/s/$id/market",
  orgs: "/s/$id/orgs",
  contracts: "/s/$id/contracts",
  store: "/s/$id/store",
  ledger: "/s/$id/ledger",
  assembly: "/s/$id/assembly",
  society: "/s/$id/society",
  talk: "/s/$id/talk",
  archives: "/s/$id/archives",
};

export function SocietyShell({ id }: { id: number }) {
  const society = useSociety(id);
  const caps = useCapabilities(id);
  const { t } = useLexicon(id);
  // The balance in the header: only where money exists, only for a citizen
  // (a non-citizen gets a 403 here and simply sees no balance).
  const home = useHome(id);
  const me = useMe();
  // Every rejection shown inside this society names people and places instead of printing ids.
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);
  useEffect(() => {
    setHumanizer(names.inText);
    return () => setHumanizer(null);
  }, [names]);
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
    // The Common Store and the Ledger of Contribution (S2.7): a moneyless
    // society's day is the shelves and the record, and only there do they exist.
    ...(c.common_store ? [{ to: "store", label: t("store"), built: true }] : []),
    ...(c.labor === "norm" ? [{ to: "ledger", label: t("ledger"), built: true }] : []),
    ...(c.org_kinds.length > 0 ? [{ to: "orgs", label: "Organizations", built: true }] : []),
    { to: "contracts", label: "Contracts", built: true },
    // The assembly exists only where the constitution has governance (S2.6): in
    // Freeport there is no nav item, which is the design statement.
    ...(c.governance !== "none" ? [{ to: "assembly", label: t("assembly"), built: true }] : []),
    { to: "society", label: "Society", built: true },
    { to: "talk", label: "Talk", built: true },
    { to: "archives", label: "Archive", built: true },
  ];
  return (
    <div>
      <header className="rule flex flex-wrap items-baseline justify-between gap-2 pb-3">
        <div className="flex items-baseline gap-3">
          <Link to="/" className="text-muted text-sm">
            Societies
          </Link>
          <Link to="/profile" className="text-muted text-sm">
            Profile
          </Link>
          {me.data?.account.operator ? (
            <Link to="/admin" className="text-warn text-sm">
              Operator
            </Link>
          ) : null}
          <h1 className="text-2xl">{s.display}</h1>
          <span className="text-muted text-xs uppercase tracking-wide">{s.preset}</span>
        </div>
        <div className="num text-muted text-sm">
          {c.money && home.data ? (
            <span className="text-ink" data-testid="header-balance">
              {t("balance")} {credits(home.data.household.balance)} cr ·{" "}
            </span>
          ) : null}
          <WorldClock clock={s.clock} nextTickAt={s.next_tick_at} tickSeconds={s.tick_seconds} />
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
