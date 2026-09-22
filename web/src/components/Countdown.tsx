// Time to the next tick or cycle end (TDD 11; docs/style.md §7.17): "next
// hour 7 s". Label muted, value bold ink in a fixed-width slot so a tick
// never shifts the layout. Ticks once a second; the region is aria-live=off
// so a reader hears it only on request (§9).

import { useEffect, useState } from "react";

export function formatUntil(at: string | null | undefined, now = Date.now()): string {
  if (!at) return "—";
  const ms = new Date(at).getTime() - now;
  if (Number.isNaN(ms)) return "—";
  if (ms <= 0) return "now";
  const s = Math.round(ms / 1000);
  if (s < 90) return `${s} s`;
  const m = Math.round(s / 60);
  if (m < 90) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${m - h * 60} min`;
}

export function Countdown({ at, label }: { at: string | null | undefined; label: string }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  return (
    <span className="text-sm whitespace-nowrap" aria-live="off">
      <span className="text-muted">{label} </span>
      <b className="text-ink inline-block min-w-[3ch] tabular-nums">{formatUntil(at, now)}</b>
    </span>
  );
}
