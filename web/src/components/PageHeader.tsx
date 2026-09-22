// Page header (docs/style.md §7.3): the h1 in the display face and a muted
// meta row — countdowns, "18 in all" — that wraps under the title on phones.

import type { ReactNode } from "react";

export function PageHeader({ title, meta, level = 1 }: { title: ReactNode; meta?: ReactNode; level?: 1 | 2 }) {
  const H = level === 1 ? "h1" : "h2";
  return (
    <header className="mb-4 flex flex-wrap items-end justify-between gap-x-4 gap-y-1">
      <H className={level === 1 ? "text-2xl sm:text-3xl" : "text-xl"}>{title}</H>
      {meta ? <div className="text-muted flex flex-wrap gap-x-3.5 gap-y-1 text-sm [&_b]:text-ink">{meta}</div> : null}
    </header>
  );
}

/** A footer strip (§5): society-wide figures, layer 3, never removed. */
export function FooterStrip({ children }: { children: ReactNode }) {
  return <p className="text-muted flex flex-wrap gap-x-3.5 pt-1 text-xs">{children}</p>;
}
