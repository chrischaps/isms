// The shell's navigation (docs/style.md §3, §5, §7.1, §7.2): a TopBar with
// the society, the clock cluster and the account links; a ScreenNav row of
// text links from md; below md a fixed bottom TabBar of five slots, the fifth
// a More button that opens a Sheet holding the rest, the account links and
// the theme control. Every item comes in from the screen that mounts these —
// a nav component never lists screens itself, so a widget's absence stays a
// design statement (GDD 15).

import { useState, type ReactNode } from "react";
import { Link, useMatchRoute, type LinkProps } from "@tanstack/react-router";
import { Icon, type IconName } from "./Icon";
import { Sheet, SheetRow } from "./Sheet";
import { ThemeControl } from "./ThemeControl";

export type NavItem = Pick<LinkProps, "to" | "params"> & {
  key: string;
  label: string;
  /** A shorter word for an 11px tab label ("Home" for "Your Accounts"). */
  short?: string;
  icon: IconName;
  /** Highlight only on this exact path (the index route). */
  exact?: boolean;
  /** One of the four screens that earn a TabBar slot; the rest live in More. */
  tab?: boolean;
};

const CURRENT = "bg-accent-soft text-accent";

/** §7.1: society name + preset chip, the clock cluster, the account links. */
export function TopBar({
  name,
  preset,
  clock,
  phoneClock,
  balance,
  account,
}: {
  name: string;
  preset: string;
  /** The full clock cluster (Epoch · Day · time · bar · next hour), from md. */
  clock: ReactNode;
  /** The countdown alone, below md: the game's heartbeat stays visible (§3). */
  phoneClock: ReactNode;
  /** Mounted only where money exists and the caller is a citizen. */
  balance?: ReactNode;
  account: NavItem[];
}) {
  return (
    <div className="bg-surface border-line border-b">
      <div className="mx-auto flex max-w-page-wide items-center gap-3 px-gutter py-2.5 md:gap-5">
        <div className="flex min-w-0 items-baseline gap-2">
          <span className="font-display truncate text-[20px] leading-tight font-bold">{name}</span>
          <span className="bg-surface-2 text-muted hidden rounded-[4px] px-1.5 py-0.5 text-xs font-bold tracking-caps uppercase sm:inline">{preset}</span>
        </div>
        <div className="text-muted ml-auto flex items-center gap-3 text-sm whitespace-nowrap md:gap-4">
          {balance}
          <span className="hidden md:inline-flex">{clock}</span>
          <span className="inline-flex md:hidden">{phoneClock}</span>
        </div>
        <nav aria-label="Account" className="hidden items-center gap-3 text-sm md:flex">
          {account.map((a) => (
            <Link key={a.key} to={a.to} params={a.params} className="text-muted font-normal hover:text-ink">
              {a.label}
            </Link>
          ))}
        </nav>
      </div>
    </div>
  );
}

/** §7.2, desktop: a row of text links, the current one in an accent-soft pill. Hidden below md. */
export function ScreenNav({ items }: { items: NavItem[] }) {
  return (
    <nav aria-label="Sections" className="bg-surface border-line hidden border-b md:block">
      <div className="mx-auto flex max-w-page-wide flex-wrap gap-1 px-gutter py-1.5">
        {items.map((n) => (
          <Link
            key={n.key}
            to={n.to}
            params={n.params}
            className="text-muted rounded-pill px-3 py-1.5 text-sm font-bold hover:text-ink hover:no-underline"
            activeOptions={{ exact: n.exact }}
            activeProps={{ className: `${CURRENT} hover:text-accent` }}
          >
            {n.label}
          </Link>
        ))}
      </div>
    </nav>
  );
}

/**
 * §7.2, phone: a fixed bottom bar with five slots — the items marked `tab`
 * (icon + label) and More, which opens a Sheet listing the rest, the account
 * links and the theme. Hidden from md. `inline` renders it in flow, for the
 * Gallery.
 */
export function TabBar({ items, account, inline }: { items: NavItem[]; account: NavItem[]; inline?: boolean }) {
  const [open, setOpen] = useState(false);
  const matchRoute = useMatchRoute();
  const tabs = items.filter((n) => n.tab).slice(0, 4);
  const rest = items.filter((n) => !tabs.includes(n));
  // The current screen lives in More: light the More slot as if it were a tab.
  const moreCurrent = rest.some((n) => Boolean(matchRoute({ to: n.to, params: n.params, fuzzy: !n.exact })));
  const slot = "flex min-h-touch flex-1 flex-col items-center justify-center gap-0.5 rounded-md px-1 py-1 text-[11px] leading-none font-bold";
  const close = () => setOpen(false);
  return (
    <nav
      aria-label="Sections"
      className={[
        "tabbar bg-surface border-line border-t md:hidden",
        inline ? "" : "fixed inset-x-0 bottom-0 z-20",
      ].join(" ")}
    >
      <div className="mx-auto flex max-w-page-wide gap-1 px-2 pt-1.5">
        {tabs.map((n) => (
          <Link
            key={n.key}
            to={n.to}
            params={n.params}
            className={`${slot} text-muted hover:no-underline`}
            activeOptions={{ exact: n.exact }}
            activeProps={{ className: `${slot} ${CURRENT}` }}
            title={n.label}
          >
            <Icon name={n.icon} />
            <span className="max-w-full truncate">{n.short ?? n.label}</span>
          </Link>
        ))}
        <button
          type="button"
          aria-haspopup="dialog"
          aria-expanded={open}
          onClick={() => setOpen(true)}
          className={`${slot} border-0 bg-transparent ${moreCurrent ? CURRENT : "text-muted"}`}
        >
          <Icon name="more" />
          <span>More</span>
        </button>
      </div>
      <Sheet open={open} onClose={close} title="More" testId="more-sheet">
        <div className="max-h-[70vh] overflow-y-auto">
          {rest.map((n) => (
            <Link
              key={n.key}
              to={n.to}
              params={n.params}
              onClick={close}
              className="text-ink block font-normal hover:no-underline"
              activeOptions={{ exact: n.exact }}
              activeProps={{ className: "text-accent block font-bold" }}
            >
              <SheetRow>
                <Icon name={n.icon} className="text-muted" />
                {n.label}
              </SheetRow>
            </Link>
          ))}
          {account.map((a) => (
            <Link key={a.key} to={a.to} params={a.params} onClick={close} className="text-ink block font-normal hover:no-underline">
              <SheetRow>
                <Icon name={a.icon} className="text-muted" />
                {a.label}
              </SheetRow>
            </Link>
          ))}
          <div className="text-muted flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
            <span>Theme</span>
            <ThemeControl />
          </div>
        </div>
      </Sheet>
    </nav>
  );
}
