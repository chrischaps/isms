// Buttons (docs/style.md §7.11, §8.6): verb first, outcome named. One primary
// per card; full width below sm. A disabled button states its reason beside
// it ("Costs 200.00 cr — you have 162.01") rather than going quiet.

import { useId, type ButtonHTMLAttributes, type ReactNode } from "react";
import { Link, type LinkProps } from "@tanstack/react-router";

export type ButtonVariant = "primary" | "secondary" | "danger" | "quiet";

const VARIANT: Record<ButtonVariant, string> = {
  primary: "bg-accent border-accent text-accent-on hover:bg-accent-hover hover:border-accent-hover",
  secondary: "bg-surface border-line text-ink hover:border-line-strong",
  danger: "bg-crit-fill border-crit-fill text-accent-on",
  quiet: "bg-transparent border-transparent text-accent hover:underline",
};

export function Button({
  variant = "secondary",
  disabledReason,
  full,
  inline,
  className,
  children,
  disabled,
  ...rest
}: {
  variant?: ButtonVariant;
  /** Why the action is unavailable; shown beside the button and linked by aria-describedby. */
  disabledReason?: ReactNode;
  /** Full width at every size (the default is full width below sm only). */
  full?: boolean;
  /** Never full width: a button inside a table row. */
  inline?: boolean;
  children: ReactNode;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  const reasonId = useId();
  const off = disabled || Boolean(disabledReason);
  const button = (
    <button
      type="button"
      disabled={off}
      aria-describedby={disabledReason ? reasonId : rest["aria-describedby"]}
      className={[
        buttonClass(variant, full, inline),
        "disabled:bg-surface-2 disabled:text-muted disabled:border-line disabled:cursor-not-allowed",
        className ?? "",
      ].join(" ")}
      {...rest}
    >
      {children}
    </button>
  );
  if (!disabledReason) return button;
  return (
    <span className="inline-flex flex-wrap items-center gap-2 max-sm:w-full">
      {button}
      <span id={reasonId} className="text-muted text-sm">
        {disabledReason}
      </span>
    </span>
  );
}

function buttonClass(variant: ButtonVariant, full?: boolean, inline?: boolean): string {
  return [
    "inline-flex min-h-touch items-center justify-center gap-2 rounded-md border px-4 py-2.5 font-bold transition-colors",
    full ? "w-full" : inline ? "" : "max-sm:w-full",
    VARIANT[variant],
  ].join(" ");
}

/** A route link dressed as a button, for an action that is a navigation ("Edit plan"). */
export function ButtonLink({
  variant = "secondary",
  full,
  className,
  children,
  ...rest
}: { variant?: ButtonVariant; full?: boolean; className?: string; children: ReactNode } & Pick<LinkProps, "to" | "params">) {
  return (
    <Link {...rest} className={[buttonClass(variant, full), "hover:no-underline", className ?? ""].join(" ")}>
      {children}
    </Link>
  );
}

/** A row of actions under a card body: primary first, wrapping; full-width buttons below sm. */
export function ButtonRow({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={["mt-3 flex flex-wrap items-center gap-2.5", className ?? ""].join(" ")}>{children}</div>;
}
