// The society's clock in familiar units (S1.13e): a cycle is a day, a tick
// is an hour when the cycle has 24 of them, and the minutes come from how
// far through the current tick the wall clock is. A thin bar shows that
// elapsed share; the countdown to the next tick sits beside it. When the
// clock is held (no next tick due) the bar is empty and says so. `phone`
// renders the bar and the countdown alone, for the compact top bar (§3).

import { useEffect, useState } from "react";
import { formatUntil } from "./Countdown";

export type ClockLike = { epoch: number; cycle: number; tick: number; ticks_per_cycle: number };

/** Share of the current tick already elapsed, 0 to 1; `null` when no tick is due. */
export function tickProgress(nextTickAt: string | null | undefined, tickSeconds: number, now = Date.now()): number | null {
  if (!nextTickAt || tickSeconds <= 0) return null;
  const remaining = (new Date(nextTickAt).getTime() - now) / 1000;
  if (Number.isNaN(remaining)) return null;
  return Math.min(1, Math.max(0, 1 - remaining / tickSeconds));
}

/** "2:35 PM" for tick 15 of 24 halfway through; "tick 3/12" when a cycle is not a day of hours. */
export function formatWorldTime(clock: ClockLike, progress: number | null): string {
  const t = Math.max(1, clock.tick);
  if (clock.ticks_per_cycle !== 24) return `tick ${t}/${clock.ticks_per_cycle}`;
  const hour = (t - 1) % 24;
  const minutes = Math.floor((progress ?? 0) * 60);
  const h12 = hour % 12 === 0 ? 12 : hour % 12;
  return `${h12}:${String(minutes).padStart(2, "0")} ${hour < 12 ? "AM" : "PM"}`;
}

export function WorldClock({
  clock,
  nextTickAt,
  tickSeconds,
  compact = false,
  phone = false,
}: {
  clock: ClockLike;
  nextTickAt: string | null | undefined;
  tickSeconds: number;
  compact?: boolean;
  phone?: boolean;
}) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  const progress = tickProgress(nextTickAt, tickSeconds, now);
  const held = progress === null;
  return (
    <span className="text-muted inline-flex flex-wrap items-center gap-x-2.5 gap-y-1 text-sm tabular-nums" data-testid={phone ? "world-clock-phone" : "world-clock"} aria-live="off">
      {phone ? null : (
        <span>
          {compact ? "" : `Epoch ${clock.epoch} · `}Day {clock.cycle} · <b className="text-ink">{formatWorldTime(clock, progress)}</b>
        </span>
      )}
      <span
        className={`bg-line relative inline-block h-1.5 ${phone ? "w-10" : "w-[70px]"} overflow-hidden rounded-[3px] align-middle`}
        role="progressbar"
        aria-label="This hour"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round((progress ?? 0) * 100)}
      >
        <span className="bg-accent absolute inset-y-0 left-0" style={{ width: `${(progress ?? 0) * 100}%` }} />
      </span>
      <span className="whitespace-nowrap">
        {held ? (
          tickSeconds === 0 ? (phone ? "fast" : "as fast as it can") : phone ? "held" : "clock held"
        ) : (
          <>
            {phone ? "" : "next hour "}
            <b className="text-ink inline-block min-w-[3ch]">{formatUntil(nextTickAt, now)}</b>
          </>
        )}
      </span>
    </span>
  );
}
