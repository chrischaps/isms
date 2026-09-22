// A number with its Explain (TDD 5.6, 11; docs/style.md §7.9): every value a
// rule produced can open the rule — its inputs as a fact list and the formula
// beneath. The `?` is the shared Why button; the note keeps role="dialog".

import { useId, useState } from "react";
import { Why } from "./Explain";

export type Explain = {
  rule: string;
  inputs: [string, unknown][] | Record<string, unknown>;
  formula: string;
  result: unknown;
};

function show(v: unknown): string {
  if (v === null || v === undefined) return "—";
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : v.toFixed(2);
  if (typeof v === "object") {
    // The engine's `Num`: {"Float": x}, {"Int": n}, or {"Money": cents}.
    const o = v as Record<string, unknown>;
    if (typeof o.Money === "number") return `${(o.Money / 100).toFixed(2)} cr`;
    if (typeof o.Float === "number") return o.Float.toFixed(2);
    if (typeof o.Int === "number") return String(o.Int);
    return JSON.stringify(v);
  }
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
    <span className={`inline-flex items-baseline gap-1 tabular-nums ${className ?? ""}`}>
      <span>
        {value}
        {unit ? <span className="text-muted text-sm"> {unit}</span> : null}
      </span>
      {explain ? (
        <span className="relative">
          <Why open={open} controls={id} onClick={() => setOpen((o) => !o)} />
          {open ? (
            // Spans throughout: a Num often sits inside a <p>, where block elements are invalid.
            <span
              id={id}
              role="dialog"
              className="bg-accent-soft text-ink absolute left-0 z-10 mt-1 block w-72 max-w-[calc(100vw-2rem)] rounded-sm px-3 py-2.5 text-[13.5px] leading-[1.45] shadow-2"
            >
              <span className="mb-2 block font-mono text-xs">{explain.rule}</span>
              {inputs.map(([k, v]) => (
                <span key={k} className="flex justify-between gap-4">
                  <span className="text-muted">{k.replaceAll("_", " ")}</span>
                  <span className="tabular-nums">{show(v)}</span>
                </span>
              ))}
              <span className="border-line mt-2 block border-t border-dashed pt-2 font-mono text-xs">{explain.formula}</span>
              <span className="mt-1 block text-right">= {show(explain.result)}</span>
            </span>
          ) : null}
        </span>
      ) : null}
    </span>
  );
}
