// Feed (docs/style.md §7.13): a list with a dot per item, for the Chronicle
// and for "While you were away". Hardship, default and rejection items are
// hot — ink text, attention dot; housekeeping items are muted with a line dot.
// Empty-state copy comes from the preset, never a blank card.

import type { ReactNode } from "react";

export type FeedItem = { key: string | number; hot?: boolean; node: ReactNode };

export function Feed({ items, empty, testId, className }: { items: FeedItem[]; empty?: ReactNode; testId?: string; className?: string }) {
  if (items.length === 0) return <p className="text-muted m-0">{empty ?? "Nothing to report yet."}</p>;
  return (
    <ul data-testid={testId} className={["feed m-0 grid list-none gap-1.5 p-0 text-[14.5px]", className ?? ""].join(" ")}>
      {items.map((it) => (
        <li key={it.key} className={it.hot ? "hot text-ink" : "text-muted"}>
          {it.node}
        </li>
      ))}
    </ul>
  );
}

/** RuleList (§7.12): the Standing Plan as a checklist. A rule that cannot run swaps its check for an attention mark and says why. */
export function RuleList({ rules }: { rules: { key: string; node: ReactNode; blocked?: ReactNode }[] }) {
  return (
    <ul className="m-0 grid list-none gap-1.5 p-0">
      {rules.map((r) => (
        <li key={r.key} className="flex items-start gap-2.5">
          <span className={`mt-[3px] shrink-0 ${r.blocked ? "text-attn" : "text-good"}`} aria-hidden>
            {r.blocked ? <WarnGlyph /> : <CheckGlyph />}
          </span>
          <span>
            {r.node}
            {r.blocked ? <span className="text-attn"> — {r.blocked}</span> : null}
          </span>
        </li>
      ))}
    </ul>
  );
}

function CheckGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5 12l4 4L19 6" />
    </svg>
  );
}
function WarnGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 9v4M12 17h.01M10.3 3.9L2.5 17.5A2 2 0 0 0 4.2 20.5h15.6a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
    </svg>
  );
}
