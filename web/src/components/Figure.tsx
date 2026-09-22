// Figure (docs/style.md §2 layer 2, §10 Society): a glance tile for one
// society-wide or firm-wide number — a caps label, the value in display type
// with its unit small beside it, and a status line or gloss beneath. The tone
// colours the status line only, by an engine threshold (§4.2); a figure with
// no state keeps the line muted. Four sit in a row from 620px of container
// width, two-up below (§10 Society, phone).

import type { ReactNode } from "react";
import { Tile } from "./Card";
import type { Tone } from "./Verdict";

const STATUS: Record<Tone, string> = { good: "text-muted", attn: "text-attn", crit: "text-crit" };

export function Figure({
  label,
  value,
  unit,
  status,
  tone = "good",
  testId,
}: {
  label: ReactNode;
  value: string;
  unit?: ReactNode;
  /** A status line or a gloss in lived terms (§8.4, §8.5). */
  status?: ReactNode;
  tone?: Tone;
  testId?: string;
}) {
  return (
    <Tile testId={testId} className="grid content-start gap-1">
      <span className="text-muted text-xs font-bold tracking-caps uppercase">{label}</span>
      <span className="font-display text-2xl leading-none font-bold tabular-nums">
        {value}
        {unit ? <small className="text-muted ml-1 font-body text-xs font-normal">{unit}</small> : null}
      </span>
      {status ? <span className={`text-[13.5px] ${STATUS[tone]}`}>{status}</span> : null}
    </Tile>
  );
}

/** Figures two-up on a phone, four across from 620px of container width. */
export function Figures({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={["grid grid-cols-2 gap-3 @min-[620px]:grid-cols-4", className ?? ""].join(" ")}>{children}</div>;
}
