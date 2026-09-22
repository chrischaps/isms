// NeedCard (docs/style.md §7.6): the glance-layer tile for one need. Top to
// bottom: icon + name + `?` · the value in display type with "/ 100" · the
// 8px bar · a status line in the meter's tone · the hidden Explain note. The
// bar and the status line follow the same thresholds (§4.2), and a bar never
// ships without its sentence (§11).

import { useId, useState, type ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import { Meter, meterTone } from "./Meter";
import { Note, Why } from "./Explain";
import type { Tone } from "./Verdict";

const STATUS: Record<Tone, string> = { good: "text-muted", attn: "text-attn", crit: "text-crit" };

export function NeedCard({
  label,
  icon,
  value,
  threshold = 20,
  status,
  note,
  tone,
  testId,
}: {
  label: string;
  icon?: IconName;
  value: number;
  threshold?: number;
  /** One short sentence: what is happening and, if it is changing, the rate (§8.4). */
  status: ReactNode;
  /** The Explain note (§8.3), opened by the `?`. */
  note?: ReactNode;
  /** Override the tone the value alone would give: a falling meter with no rule to stop it is attn (§4.2). */
  tone?: Tone;
  testId?: string;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const v = Math.max(0, Math.min(100, value));
  const t = tone ?? meterTone(v, threshold);
  return (
    <div data-testid={testId} className="border-line grid gap-1.5 rounded-md border px-3.5 py-3">
      <div className="flex items-center gap-2 font-bold">
        {icon ? <Icon name={icon} className="text-muted" /> : null}
        <span>{label}</span>
        {note ? <Why open={open} controls={id} onClick={() => setOpen((o) => !o)} label={`Explain ${label}`} /> : null}
      </div>
      <div className="font-display text-2xl leading-none font-bold">
        {v.toFixed(0)}
        <small className="text-muted ml-1 font-body text-xs font-normal">/ 100</small>
      </div>
      <Meter label={label} value={v} threshold={threshold} tone={t} bare />
      <p className={`m-0 text-[13.5px] ${STATUS[t]}`}>{status}</p>
      {open && note ? <Note id={id}>{note}</Note> : null}
    </div>
  );
}

/** Three NeedCards in a row from 620px of container width; a stack below. */
export function Needs({ children }: { children: ReactNode }) {
  return <div className="grid gap-3 @min-[620px]:grid-cols-3">{children}</div>;
}
