// A number with its Explain (TDD 5.6, 11): every value a rule produced can
// open a footnote naming the rule, its inputs, and the formula.

import { useId, useState } from "react";

export type Explain = {
  rule: string;
  inputs: [string, unknown][] | Record<string, unknown>;
  formula: string;
  result: unknown;
};

function show(v: unknown): string {
  if (v === null || v === undefined) return "—";
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : v.toFixed(2);
  if (typeof v === "object") return JSON.stringify(v);
  return String(v);
}

export function Num({
  value,
  unit,
  explain,
  className,
}: {
  value: number | string;
  unit?: string;
  explain?: Explain | null;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const inputs: [string, unknown][] = Array.isArray(explain?.inputs)
    ? explain.inputs
    : Object.entries(explain?.inputs ?? {});
  return (
    <span className={`num inline-flex items-baseline gap-1 ${className ?? ""}`}>
      <span>
        {value}
        {unit ? <span className="text-muted text-sm"> {unit}</span> : null}
      </span>
      {explain ? (
        <span className="relative">
          <button
            type="button"
            aria-label="Explain"
            aria-expanded={open}
            aria-controls={id}
            onClick={() => setOpen((o) => !o)}
            className="text-accent border-line rounded-sm border px-1 text-xs leading-none"
          >
            ?
          </button>
          {open ? (
            <div id={id} role="dialog" className="explain absolute left-0 z-10 mt-1">
              <div className="mb-2 font-mono text-xs">{explain.rule}</div>
              <dl>
                {inputs.map(([k, v]) => (
                  <div key={k} className="flex justify-between gap-4">
                    <dt>{k}</dt>
                    <dd className="num">{show(v)}</dd>
                  </div>
                ))}
              </dl>
              <div className="rule mt-2 pt-2 font-mono text-xs">{explain.formula}</div>
              <div className="mt-1 text-right">= {show(explain.result)}</div>
            </div>
          ) : null}
        </span>
      ) : null}
    </span>
  );
}
