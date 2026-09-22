// A need meter (GDD 4.2; docs/style.md §7.6): 0 to 100 with the hardship
// line drawn where it is. Fill colour follows the engine's thresholds — crit
// under the hardship line, attn under 50, good above — never taste (§4.2).
// With a `hint`, the label opens a tooltip saying what moves the meter: on
// hover, on keyboard focus, and on a tap (touch has no hover). `bare` renders
// the bar alone, for a NeedCard that carries its own label and status line.

import { useId, useState } from "react";
import type { Tone } from "./Verdict";

export function meterTone(value: number, threshold = 20, attn = 50): Tone {
  if (value < threshold) return "crit";
  if (value < attn) return "attn";
  return "good";
}

const FILL: Record<Tone, string> = { good: "bg-good-fill", attn: "bg-attn-fill", crit: "bg-crit-fill" };
const TEXT: Record<Tone, string> = { good: "", attn: "text-attn", crit: "text-crit" };

export function Meter({
  label,
  value,
  threshold = 20,
  hint,
  bare,
  tone: forced,
}: {
  label: string;
  value: number;
  threshold?: number;
  hint?: string;
  bare?: boolean;
  /** Override the tone the value alone would give (a falling meter with no rule to stop it is attn, §4.2). */
  tone?: Tone;
}) {
  const [pinned, setPinned] = useState(false);
  const tip = useId();
  const v = Math.max(0, Math.min(100, value));
  const tone = forced ?? meterTone(v, threshold);
  const bar = (
    <span
      role="meter"
      aria-valuenow={v}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={label}
      className="bg-line relative block h-2 flex-1 overflow-hidden rounded-[4px]"
    >
      <span data-testid="meter-fill" className={`absolute inset-y-0 left-0 rounded-[4px] ${FILL[tone]}`} style={{ width: `${v}%` }} />
      <span className="bg-ink absolute inset-y-0 w-px opacity-40" style={{ left: `${threshold}%` }} aria-hidden />
    </span>
  );
  if (bare) return bar;
  return (
    <div className="flex items-center gap-3">
      {hint ? (
        // The label sits outside the meter role, whose children a screen reader does not enter.
        <span className="group relative w-20 text-sm">
          <button
            type="button"
            aria-describedby={tip}
            aria-expanded={pinned}
            onClick={() => setPinned((p) => !p)}
            onBlur={() => setPinned(false)}
            className="decoration-muted cursor-help underline decoration-dotted underline-offset-4"
          >
            {label}
          </button>
          <span
            id={tip}
            role="tooltip"
            className={`bg-accent-soft text-ink absolute top-full left-0 z-10 mt-1 w-72 rounded-sm px-3 py-2.5 text-[13.5px] leading-[1.45] shadow-2 ${pinned ? "block" : "hidden group-focus-within:block group-hover:block"}`}
          >
            {hint}
          </span>
        </span>
      ) : (
        <span className="w-20 text-sm">{label}</span>
      )}
      {bar}
      <span className={`w-12 text-right text-sm tabular-nums ${TEXT[tone]}`}>{v.toFixed(0)}</span>
    </div>
  );
}
