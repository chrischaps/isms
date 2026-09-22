// Pill (docs/style.md §7.8): an inline status chip — none · open · in hardship
// · you — never a category. Tones map to literal classes: Tailwind only
// compiles class names it can read in the source.

import type { ReactNode } from "react";

export type PillTone = "good" | "attn" | "crit" | "info" | "neutral";

const TONE: Record<PillTone, string> = {
  good: "bg-good-soft text-good",
  attn: "bg-attn-soft text-attn",
  crit: "bg-crit-soft text-crit",
  info: "bg-info-soft text-info",
  neutral: "bg-surface-2 text-muted",
};

export function Pill({ tone = "neutral", children, className, testId }: { tone?: PillTone; children: ReactNode; className?: string; testId?: string }) {
  return (
    <span data-testid={testId} className={["inline-block rounded-pill px-2 py-0.5 text-[12.5px] leading-[1.4] font-bold", TONE[tone], className ?? ""].join(" ")}>
      {children}
    </span>
  );
}
