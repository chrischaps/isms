// The society shell (TDD 11, GDD 15; docs/style.md §3, §5): the nav is built
// from capabilities and labelled from the lexicon, so the absence of a widget
// is a design statement. A TopBar and a ScreenNav row from md; below md the
// TopBar shrinks and the ScreenNav becomes the bottom TabBar with a More
// sheet. The index route is Home (S1.8), which onboards a non-citizen.

import { useEffect } from "react";
import { Outlet } from "@tanstack/react-router";
import { useOffices } from "../api/assembly";
import { credits, setHumanizer } from "../api/client";
import { useCapabilities, useHome, useLexicon, useMe, useSociety, useStream } from "../api/hooks";
import { ScreenNav, TabBar, TopBar, type NavItem } from "../components/Nav";
import { WorldClock } from "../components/WorldClock";
import { useNames } from "../lib/names";
import { Home } from "./Home";

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
  // A role workspace mounts on holding the office, not on a capability (S2.8):
  // the offices are read once the caller is a citizen of a governed society.
  const offices = useOffices(id, home.data !== undefined && caps.data !== undefined && caps.data.governance !== "none");
  const coordinates = offices.data?.offices.some((o) => o.kind === "coordinator" && o.i_hold) ?? false;
  useStream(id);
  if (society.isPending || caps.isPending) return <p className="text-muted px-gutter pt-4">Loading.</p>;
  if (society.error || caps.error) {
    return <p className="text-crit px-gutter pt-4">Could not load: {String(society.error ?? caps.error)}</p>;
  }
  const s = society.data!;
  const c = caps.data!;
  const params = { id: String(id) };
  // The four `tab` items are this society's most-used screens (§3): Home, the
  // hours, the place goods change hands, and the Society. The rest go to More.
  const nav: NavItem[] = [
    { key: "home", to: "/s/$id", params, label: t("home_title"), short: "Home", icon: "home", exact: true, tab: true },
    { key: "work", to: "/s/$id/work", params, label: t("work_screen"), icon: "work", tab: true },
    { key: "plan", to: "/s/$id/plan", params, label: t("plan"), icon: "plan" },
    ...(c.order_books ? [{ key: "market", to: "/s/$id/market", params, label: t("store_screen"), icon: "market", tab: true } as NavItem] : []),
    // The Common Store and the Ledger of Contribution (S2.7): a moneyless
    // society's day is the shelves and the record, and only there do they exist.
    ...(c.common_store ? [{ key: "store", to: "/s/$id/store", params, label: t("store_screen"), icon: "store", tab: true } as NavItem] : []),
    ...(c.labor === "norm" ? [{ key: "ledger", to: "/s/$id/ledger", params, label: t("ledger_screen"), icon: "ledger" } as NavItem] : []),
    ...(c.org_kinds.length > 0 ? [{ key: "orgs", to: "/s/$id/orgs", params, label: "Organizations", icon: "org" } as NavItem] : []),
    { key: "contracts", to: "/s/$id/contracts", params, label: "Contracts", icon: "contract" },
    // The assembly exists only where the constitution has governance (S2.6): in
    // Freeport there is no nav item, which is the design statement.
    ...(c.governance !== "none" ? [{ key: "assembly", to: "/s/$id/assembly", params, label: t("assembly"), icon: "assembly" } as NavItem] : []),
    // The office's workspace, for its holders alone (S2.8; TDD 4 roles/).
    ...(coordinates ? [{ key: "coordinator", to: "/s/$id/coordinator", params, label: t("office"), icon: "office" } as NavItem] : []),
    { key: "society", to: "/s/$id/society", params, label: "Society", icon: "people", tab: true },
    { key: "talk", to: "/s/$id/talk", params, label: "Talk", icon: "talk" },
    { key: "archives", to: "/s/$id/archives", params, label: "Archive", icon: "archive" },
  ];
  const account: NavItem[] = [
    { key: "societies", to: "/", label: "Societies", icon: "globe" },
    { key: "profile", to: "/profile", label: "Profile", icon: "person" },
    ...(me.data?.account.operator ? [{ key: "admin", to: "/admin", label: "Operator", icon: "gear" } as NavItem] : []),
  ];
  const balance =
    c.money && home.data ? (
      <span className="text-ink" data-testid="header-balance">
        <span className="text-muted hidden sm:inline">{t("balance")} </span>
        <b>{credits(home.data.household.balance)} cr</b>
      </span>
    ) : null;
  return (
    <div>
      <TopBar
        name={s.display}
        preset={s.preset}
        balance={balance}
        clock={<WorldClock clock={s.clock} nextTickAt={s.next_tick_at} tickSeconds={s.tick_seconds} />}
        phoneClock={<WorldClock clock={s.clock} nextTickAt={s.next_tick_at} tickSeconds={s.tick_seconds} phone />}
        account={account}
      />
      <ScreenNav items={nav} />
      <main className="mx-auto max-w-page-wide px-gutter pt-4 md:pt-6">
        <Outlet />
      </main>
      <TabBar items={nav} account={account} />
    </div>
  );
}

export function SocietyHome({ id }: { id: number }) {
  return <Home id={id} />;
}
