// Toast (docs/style.md §7.20): bottom-centre above the TabBar, ink fill with
// bg text, four seconds, one at a time. A rejected command toasts in crit
// with the engine's reason, verbatim (§8.7). Mount <Toaster/> once in the
// shell; call `toast()` from anywhere.

import { useEffect, useState } from "react";

export type ToastTone = "ink" | "crit";
type ToastMsg = { id: number; text: string; tone: ToastTone };

const listeners = new Set<(t: ToastMsg | null) => void>();
let seq = 0;

export function toast(text: string, tone: ToastTone = "ink") {
  const t = { id: ++seq, text, tone };
  for (const l of listeners) l(t);
}

export const TOAST_MS = 4000;

export function Toaster() {
  const [cur, setCur] = useState<ToastMsg | null>(null);
  useEffect(() => {
    listeners.add(setCur);
    return () => {
      listeners.delete(setCur);
    };
  }, []);
  useEffect(() => {
    if (!cur) return;
    const h = setTimeout(() => setCur(null), TOAST_MS);
    return () => clearTimeout(h);
  }, [cur]);
  return (
    <div role="status" aria-live="polite" className="pointer-events-none fixed inset-x-0 bottom-[calc(var(--spacing-tabbar)+16px)] z-30 flex justify-center px-4 md:bottom-6">
      {cur ? (
        <div key={cur.id} className={`pointer-events-auto rounded-md px-4 py-2.5 text-sm font-bold shadow-2 ${cur.tone === "crit" ? "bg-crit-fill text-accent-on" : "bg-ink text-bg"}`}>
          {cur.text}
        </div>
      ) : null}
    </div>
  );
}
