// Line icons (docs/style.md §6): 20px on a 24 grid, 2px stroke, currentColor.
// One icon per concept, fixed across the app; an icon always sits beside a
// label and never replaces one, so it is hidden from readers unless titled.

import type { SVGProps } from "react";

type Shape = { d: string; fill?: boolean; join?: boolean; cap?: boolean; extra?: string; width?: number };

const SHAPES = {
  food: { d: "M12 22V10M12 10c-3 0-5-2-5-5 3 0 5 2 5 5zM12 10c3 0 5-2 5-5-3 0-5 2-5 5zM12 15c-3 0-5-2-5-5 3 0 5 2 5 5zM12 15c3 0 5-2 5-5-3 0-5 2-5 5z", cap: true },
  home: { d: "M3 11l9-8 9 8v10a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z", join: true },
  spark: { d: "M12 3l2.2 5.8L20 11l-5.8 2.2L12 19l-2.2-5.8L4 11l5.8-2.2z", cap: true, join: true },
  coin: { d: "M14.5 9.5a3 3 0 1 0 0 5", cap: true, extra: "circle" },
  work: { d: "M3 21h18M5 21V8l7-5 7 5v13M9 21v-5h6v5", cap: true, join: true },
  plan: { d: "M4 6h16M4 12h10M4 18h7", cap: true, join: true, extra: "gear" },
  page: { d: "M5 3h10l4 4v14H5zM8 11h8M8 15h8", cap: true, join: true },
  clock: { d: "M12 7v5l3 2", cap: true, extra: "circle" },
  warn: { d: "M12 9v4M12 17h.01M10.3 3.9L2.5 17.5A2 2 0 0 0 4.2 20.5h15.6a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z", cap: true, join: true },
  market: { d: "M4 7h13l-3-3M20 17H7l3 3", cap: true, join: true },
  org: { d: "M4 21V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v16M15 10h4a1 1 0 0 1 1 1v10M8 8h3M8 12h3M8 16h3M3 21h18", cap: true, join: true },
  contract: { d: "M5 3h10l4 4v14H5zM8 17c1-2 2-2 3 0s2 2 3 0 2-2 3 0", cap: true, join: true },
  people: { d: "M3 20c0-3.5 2.5-6 6-6s6 2.5 6 6M15 15c3 0 5 2 5 5", cap: true, extra: "people" },
  talk: { d: "M4 5h16v11H9l-5 4z", cap: true, join: true },
  more: { d: "", fill: true, extra: "dots" },
  check: { d: "M5 12l4 4L19 6", cap: true, join: true, width: 2.5 },
  archive: { d: "M3 7h18v3H3zM5 10v11h14V10M10 14h4", cap: true, join: true },
  store: { d: "M3 9l2-5h14l2 5M3 9h18v3a2 2 0 0 1-4 0 2 2 0 0 1-4 0 2 2 0 0 1-4 0 2 2 0 0 1-4 0 2 2 0 0 1-2-2V9zM5 13v8h14v-8", cap: true, join: true },
  ledger: { d: "M5 3h14v18H5zM9 8h6M9 12h6M9 16h4", cap: true, join: true },
  assembly: { d: "M4 20h16M6 20V10M12 20V4M18 20V13", cap: true, join: true },
  office: { d: "M12 3l7 4v5c0 5-3 8-7 9-4-1-7-4-7-9V7z", cap: true, join: true },
  chevron: { d: "M9 6l6 6-6 6", cap: true, join: true },
  person: { d: "M4 21c0-4 3.5-7 8-7s8 3 8 7", cap: true, extra: "head" },
  globe: { d: "M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18", cap: true, extra: "circle" },
  gear: { d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z", cap: true, join: true },
} satisfies Record<string, Shape>;

export type IconName = keyof typeof SHAPES;

export const ICON_NAMES = Object.keys(SHAPES) as IconName[];

export function Icon({
  name,
  size = 20,
  title,
  className,
  ...rest
}: { name: IconName; size?: number; title?: string; className?: string } & Omit<SVGProps<SVGSVGElement>, "name">) {
  const s: Shape = SHAPES[name];
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill={s.fill ? "currentColor" : "none"}
      stroke={s.fill ? undefined : "currentColor"}
      strokeWidth={s.fill ? undefined : (s.width ?? 2)}
      strokeLinecap={s.cap ? "round" : undefined}
      strokeLinejoin={s.join ? "round" : undefined}
      aria-hidden={title ? undefined : true}
      role={title ? "img" : undefined}
      className={["shrink-0", className].filter(Boolean).join(" ")}
      {...rest}
    >
      {title ? <title>{title}</title> : null}
      {s.extra === "circle" ? <circle cx="12" cy="12" r="9" /> : null}
      {s.extra === "gear" ? <circle cx="18" cy="16" r="3" /> : null}
      {s.extra === "head" ? <circle cx="12" cy="8" r="4" /> : null}
      {s.extra === "people" ? (
        <>
          <circle cx="9" cy="8" r="3" />
          <circle cx="17" cy="9" r="2.5" />
        </>
      ) : null}
      {s.extra === "dots" ? (
        <>
          <circle cx="5" cy="12" r="2" />
          <circle cx="12" cy="12" r="2" />
          <circle cx="19" cy="12" r="2" />
        </>
      ) : null}
      {s.d ? <path d={s.d} /> : null}
    </svg>
  );
}
