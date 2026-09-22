// A Storybook-free gallery of the shared components with fixed data, for
// visual checks (TDD 11, S1.7; docs/style.md §12 step 8): the catalog at a
// chosen width, in the page's theme or both themes side by side.

import { useState } from "react";
import { ThemeControl } from "../components/ThemeControl";
import { SECTIONS } from "./gallery/catalog";

const WIDTHS = [
  { label: "Phone", px: 390 },
  { label: "Tablet", px: 720 },
  { label: "Desktop", px: 1100 },
  { label: "Fit", px: 0 },
];

function Catalog() {
  return (
    <div className="grid gap-10">
      {SECTIONS.map((s) => (
        <section key={s.id} id={s.id} className="scroll-mt-4">
          <h2 className="mb-1 text-2xl">{s.title}</h2>
          {s.spec ? <p className="text-muted mt-0 mb-4 max-w-[70ch] text-sm">{s.spec}</p> : null}
          {s.node}
        </section>
      ))}
    </div>
  );
}

export function Gallery() {
  const [width, setWidth] = useState(0);
  const [both, setBoth] = useState(false);
  const frame = (theme?: "light" | "dark") => (
    <div
      data-theme={theme}
      className="bg-bg text-ink @container border-line-strong min-w-0 max-w-full justify-self-center rounded-lg border border-dashed p-4"
      style={width ? { width } : { width: "100%" }}
    >
      <Catalog />
    </div>
  );
  return (
    <div className="grid gap-6">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1>Companion</h1>
          <p className="text-muted mt-2 mb-0 max-w-[62ch]">
            The rendered reference for <code className="bg-surface-2 rounded-[4px] px-1 font-mono text-[0.9em]">docs/style.md</code>. Every colour comes from the tokens, so switching theme here is the test the app passes.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <ThemeControl />
          <fieldset className="bg-surface border-line m-0 inline-flex rounded-pill border p-[3px]" aria-label="Width">
            {WIDTHS.map((w) => (
              <label key={w.px} className={`cursor-pointer rounded-pill px-3 py-1.5 text-sm font-bold ${width === w.px ? "bg-ink text-bg" : "text-muted hover:text-ink"}`}>
                <input type="radio" name="width" className="sr-only" checked={width === w.px} onChange={() => setWidth(w.px)} />
                {w.label}
              </label>
            ))}
          </fieldset>
          <label className="flex items-center gap-2 text-sm font-bold">
            <input type="checkbox" checked={both} onChange={(e) => setBoth(e.target.checked)} />
            Both themes
          </label>
        </div>
      </header>
      <nav aria-label="Catalog" className="text-muted flex flex-wrap gap-x-3 gap-y-1 text-sm">
        {SECTIONS.map((s) => (
          <a key={s.id} href={`#${s.id}`} className="font-normal">
            {s.title}
          </a>
        ))}
      </nav>
      {both ? (
        <div className="grid gap-6 lg:grid-cols-2">
          {frame("light")}
          {frame("dark")}
        </div>
      ) : (
        frame()
      )}
    </div>
  );
}
