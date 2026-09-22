// Inputs (docs/style.md §7.18): every field has a visible label; number
// fields show their unit inside the box; a validation message sits beneath in
// crit and says what to change. The engine's rejection text is shown verbatim.
// The label wraps the control, so `getByLabel` finds it without ids.

import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";

const BOX = "border-line bg-surface text-ink flex min-h-touch items-center rounded-sm border px-3 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent-soft";

export function Field({
  label,
  unit,
  error,
  hint,
  children,
  className,
}: {
  label: ReactNode;
  /** A unit shown inside the box after the value: "h", "cr", "food". */
  unit?: ReactNode;
  error?: ReactNode;
  hint?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={["grid max-w-xs gap-1.5", className ?? ""].join(" ")}>
      <span className="text-sm font-bold">{label}</span>
      <span className={[BOX, error ? "border-crit" : ""].join(" ")}>
        {children}
        {unit ? <span className="text-muted ml-2 text-sm">{unit}</span> : null}
      </span>
      {error ? (
        <span role="alert" className="text-crit text-sm">
          {error}
        </span>
      ) : null}
      {hint && !error ? <span className="text-muted text-sm">{hint}</span> : null}
    </label>
  );
}

const CONTROL = "min-w-0 flex-1 border-0 bg-transparent py-2 outline-none";

export function Input({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={[CONTROL, className ?? ""].join(" ")} {...rest} />;
}

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={[CONTROL, className ?? ""].join(" ")} {...rest}>
      {children}
    </select>
  );
}

/** A standalone control for places without room for a Field (a table cell); still label it with aria-label. */
export function BareInput({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={["border-line bg-surface text-ink min-h-touch rounded-sm border px-3 focus:border-accent focus:ring-2 focus:ring-accent-soft focus:outline-none", className ?? ""].join(" ")} {...rest} />;
}
