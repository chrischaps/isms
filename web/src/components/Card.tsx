// Card (docs/style.md §7.4): surface, one line, no shadow, no coloured border.
// A card's state is said by its content — pills and status lines — never by
// tinting the card. Never nest a card in a card: use a Tile inside.

import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

export function Card({
  title,
  icon,
  aside,
  subtitle,
  children,
  className,
  wide,
  testId,
}: {
  title?: ReactNode;
  icon?: IconName;
  /** Sits at the end of the title row: an Explain, a Pill, a count. */
  aside?: ReactNode;
  subtitle?: ReactNode;
  children?: ReactNode;
  className?: string;
  /** Spans both columns of a `.two` grid. */
  wide?: boolean;
  testId?: string;
}) {
  return (
    <section
      data-testid={testId}
      className={["bg-surface border-line rounded-lg border p-5 @min-[720px]:p-6", wide ? "md:col-span-2" : "", className ?? ""].join(" ")}
    >
      {title ? (
        <h2 className="mb-1 flex items-center gap-2 text-[17px] @min-[620px]:text-xl">
          {icon ? <Icon name={icon} className="text-accent" /> : null}
          <span className="min-w-0 flex-1">{title}</span>
          {aside}
        </h2>
      ) : null}
      {subtitle ? <p className="text-muted mb-3 text-sm">{subtitle}</p> : null}
      {children}
    </section>
  );
}

/** A lighter inner tile: the only thing that may sit "inside" a card with its own border. */
export function Tile({ children, className, testId }: { children: ReactNode; className?: string; testId?: string }) {
  return (
    <div data-testid={testId} className={["border-line rounded-md border px-3.5 py-3", className ?? ""].join(" ")}>
      {children}
    </div>
  );
}

/** A vertical stack of cards with the bible's gap. */
export function Stack({ children, className, testId }: { children: ReactNode; className?: string; testId?: string }) {
  return (
    <div data-testid={testId} className={["grid gap-4", className ?? ""].join(" ")}>
      {children}
    </div>
  );
}

/** Two cards side by side from md, stacked below. */
export function Two({ children, className, testId }: { children: ReactNode; className?: string; testId?: string }) {
  return (
    <div data-testid={testId} className={["grid gap-4 md:grid-cols-2", className ?? ""].join(" ")}>
      {children}
    </div>
  );
}
