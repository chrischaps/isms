// The Verdict (docs/style.md §7.5, §8.2): one sentence at the top of the first
// card, written from the player's side of the screen. Emphasised phrases are
// bold and coloured by status; the sentence itself is real text, so a screen
// reader hears it plain. A screen has exactly one.

import type { ReactNode } from "react";

export type Tone = "good" | "attn" | "crit";

/** One piece of a verdict: plain text, or a state word with its tone; `ink` is emphasis with no state (a price, a figure). */
export type VerdictPart = string | { text: string; tone: Tone | "ink" };

const TONE: Record<Tone | "ink", string> = {
  good: "text-good font-bold",
  attn: "text-attn font-bold",
  crit: "text-crit font-bold",
  ink: "font-bold",
};

export function Verdict({ parts, children, testId }: { parts?: VerdictPart[]; children?: ReactNode; testId?: string }) {
  return (
    <p data-testid={testId ?? "verdict"} className="mb-4 text-lg leading-[1.4]">
      {parts?.map((p, i) =>
        typeof p === "string" ? <span key={i}>{p}</span> : <b key={i} className={TONE[p.tone]}>{p.text}</b>,
      )}
      {children}
    </p>
  );
}

/** A state word inside a hand-written verdict. */
export function State({ tone, children }: { tone: Tone; children: ReactNode }) {
  return <b className={TONE[tone]}>{children}</b>;
}

/** The plain sentence, for tests and titles. */
export function verdictText(parts: VerdictPart[]): string {
  return parts.map((p) => (typeof p === "string" ? p : p.text)).join("");
}
