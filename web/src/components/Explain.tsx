// Explain (docs/style.md §7.9, §8.3): the `?`. An 18px circle with a 44px hit
// area; tapping toggles a Note directly beneath the thing it belongs to. Notes
// are two sentences in plain words and never interrupt unless asked for;
// closing one leaves no trace. `Num` uses the same button for a rule.

import { useId, useState, type ReactNode } from "react";

/** The `?` button alone, for components that render their own note (Num). */
export function Why({ open, controls, onClick, label = "Explain" }: { open: boolean; controls: string; onClick: () => void; label?: string }) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-expanded={open}
      aria-controls={controls}
      onClick={onClick}
      className={[
        "why ml-1.5 inline-flex h-[18px] w-[18px] items-center justify-center rounded-full border-[1.5px] border-current bg-transparent p-0 align-middle text-[11px] leading-none font-bold",
        open ? "opacity-100" : "opacity-60 hover:opacity-100",
      ].join(" ")}
    >
      ?
    </button>
  );
}

/** The note itself: accent-soft fill, small radius, ≤ 2 sentences. */
export function Note({ id, children, className }: { id?: string; children: ReactNode; className?: string }) {
  return (
    <span id={id} role="note" className={["bg-accent-soft text-ink mt-2 block rounded-sm px-3 py-2.5 text-[13.5px] leading-[1.45]", className ?? ""].join(" ")}>
      {children}
    </span>
  );
}

/**
 * A term with its `?`: `<Explain note="…">Shelter</Explain>` renders the
 * term, the button, and — when open — the note after them in DOM order.
 * Inline by default (spans), since it often sits inside a heading or a dd.
 */
export function Explain({ note, children, label, className }: { note: ReactNode; children?: ReactNode; label?: string; className?: string }) {
  const [open, setOpen] = useState(false);
  const id = useId();
  return (
    <span className={["inline", className ?? ""].join(" ")}>
      {children}
      <Why open={open} controls={id} onClick={() => setOpen((o) => !o)} label={label} />
      {open ? <Note id={id}>{note}</Note> : null}
    </span>
  );
}
