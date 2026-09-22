// Tabs (docs/style.md §3, §7.15): on a phone the book, the tape and the chart
// are tabs in one card, one at a time; from md the tab row goes away and every
// panel is laid out at once under its own caps heading, so nothing is hidden
// where there is room. Every panel stays in the DOM at every width.

import { useState, type ReactNode } from "react";

export type Tab = { key: string; label: string; node: ReactNode; /** Extra classes for the panel from md (e.g. `md:col-span-2`). */ className?: string };

export function Tabs({ tabs, label, layout = "md:grid-cols-2", testId }: { tabs: Tab[]; label: string; /** The md+ grid the panels take. */ layout?: string; testId?: string }) {
  const [active, setActive] = useState(tabs[0]?.key);
  return (
    <div data-testid={testId}>
      <div role="tablist" aria-label={label} className="border-line mb-3 flex gap-1 border-b md:hidden">
        {tabs.map((t) => {
          const on = t.key === active;
          return (
            <button
              key={t.key}
              type="button"
              role="tab"
              id={`tab-${t.key}`}
              aria-selected={on}
              aria-controls={`panel-${t.key}`}
              onClick={() => setActive(t.key)}
              className={[
                "min-h-touch -mb-px border-b-2 px-3 text-sm font-bold",
                on ? "border-accent text-accent" : "border-transparent text-muted",
              ].join(" ")}
            >
              {t.label}
            </button>
          );
        })}
      </div>
      <div className={["md:grid md:gap-6", layout].join(" ")}>
        {tabs.map((t) => (
          <section
            key={t.key}
            id={`panel-${t.key}`}
            role="tabpanel"
            aria-labelledby={`tab-${t.key}`}
            className={[t.key === active ? "" : "hidden md:block", t.className ?? ""].join(" ")}
          >
            <h3 className="text-muted mb-1.5 hidden text-xs font-bold tracking-caps uppercase md:block">{t.label}</h3>
            {t.node}
          </section>
        ))}
      </div>
    </div>
  );
}
