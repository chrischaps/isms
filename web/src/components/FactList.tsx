// FactList (docs/style.md §7.7): the layer-3 workhorse. A two-column dl —
// muted label left, ink value right-aligned — where every current label/value
// pair lives. A value may carry a Pill, a muted gloss in parentheses
// ("22 food (about 22 hours)") and one inline action link ("adjust").

import type { ReactNode } from "react";

export type Fact = {
  label: ReactNode;
  value: ReactNode;
  /** A translation into lived terms, muted, in parentheses. */
  gloss?: ReactNode;
  /** One inline action, e.g. a Link "adjust". */
  action?: ReactNode;
  testId?: string;
  key?: string;
};

export function FactList({ items, className }: { items: Fact[]; className?: string }) {
  return (
    <dl className={["grid grid-cols-[1fr_auto] gap-x-4 gap-y-2 text-[15px]", className ?? ""].join(" ")}>
      {items.map(({ key, ...f }, i) => (
        <FactRow key={key ?? i} {...f} />
      ))}
    </dl>
  );
}

export function FactRow({ label, value, gloss, action, testId }: Omit<Fact, "key">) {
  return (
    <>
      <dt className="text-muted">{label}</dt>
      <dd data-testid={testId} className="m-0 text-right">
        {value}
        {gloss ? <span className="text-muted"> ({gloss})</span> : null}
        {action ? <span className="text-muted"> · {action}</span> : null}
      </dd>
    </>
  );
}
