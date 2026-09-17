// A need meter (GDD 4.2): 0 to 100, with the hardship line drawn where it is.
// With a `hint`, the label opens a tooltip saying what moves the meter: on
// hover, on keyboard focus, and on a tap (touch has no hover).

import { useId, useState } from "react";

export function Meter({
  label,
  value,
  threshold = 20,
  hint,
}: {
  label: string;
  value: number;
  threshold?: number;
  hint?: string;
}) {
  const [pinned, setPinned] = useState(false);
  const tip = useId();
  const v = Math.max(0, Math.min(100, value));
  const low = v < threshold;
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
            className={`explain absolute top-full left-0 z-10 mt-1 w-72 ${pinned ? "block" : "hidden group-focus-within:block group-hover:block"}`}
          >
            {hint}
          </span>
        </span>
      ) : (
        <span className="w-20 text-sm">{label}</span>
      )}
      <span
        role="meter"
        aria-valuenow={v}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
        className="bg-paper-2 border-line relative h-2 flex-1 overflow-hidden rounded-sm border"
      >
        <span
          data-testid="meter-fill"
          className={`absolute inset-y-0 left-0 ${low ? "bg-bad" : "bg-ink-2"}`}
          style={{ width: `${v}%` }}
        />
        <span
          className="bg-accent absolute inset-y-0 w-px opacity-60"
          style={{ left: `${threshold}%` }}
          aria-hidden
        />
      </span>
      <span className={`num w-12 text-right text-sm ${low ? "text-bad" : ""}`}>{v.toFixed(0)}</span>
    </div>
  );
}
