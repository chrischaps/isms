# The Companion refresh — kickoff plan (SB.1–SB.5)

Written 2026-09-22, before SB.1. The design is `docs/style.md` (the bible) with `docs/style/reference.html`; the decision is ADR-0013. Five web-only cards, committed to main, run before S2.11 so that S2.11's kit pass lands once on the new kit.

## Context

The client is one level deep: every screen presents its facts as label/value rows in a plain paper-and-ink shell with no shared primitives, no fonts, no dark mode and no phone navigation. The bible keeps every datum and adds a hierarchy — Verdict, Glance, Facts, Detail, Explain — with a phone-first layout, a token-only palette in two themes, and a named component catalog.

What the code is, going in:

- **Tailwind v4** (`@tailwindcss/vite`, `@theme` in `web/src/styles/tokens.css`, no config file). The bible's `tailwind.preset.js` is v3 and is kept as reference; ADR-0013 §4 says how the tokens are reproduced.
- One CSS file. Old names across the screens: `text-muted` (98 uses), `text-bad` (30), `text-warn`, `bg-paper-2`, `text-ink-2`, `border-line`, `rounded-sm` (151), the classes `num`/`rule`/`explain`, and an unlayered `table … padding-right` rule.
- Components: `Num` (`?` → `role="dialog"`), `Meter` (`role="meter"`, `data-testid="meter-fill"`, tooltip), `Ledger`, `OrderBook`, `TimeSeries` (uPlot), `Countdown`, `WorldClock`, `DiffSinceLastSeen`, `Provenance`; `Stat`/`StatTiles` in `screens/SocietyScreen.tsx`. Shell in `screens/Society.tsx`; page wrapper in `router.tsx`; `Gallery.tsx` at `/gallery`.
- Tests: vitest (`components.test.tsx`, `lib.test.ts`, `roles.test.ts`); eleven Playwright specs in `web/e2e/` keyed on roles, testids, labels and text. No snapshot tests. `make e2e-agents` is API-only.
- Shell-voice copy precedent: `web/src/lib/needs.ts` (`needHints(t, caps)`). Verdicts are shell voice (§8.1) and live in `web/src/lib/verdict.ts`; society voice stays in `presets/copy/`, served by the server.

## The cards

| card | what | done gate |
|---|---|---|
| SB.1 | tokens in v4 form, fonts, theme switch, the §7 primitives, `Icon`, Gallery catalog, `style-guard`, docs moved | `ui.test.tsx` + `theme.test.ts`; guard green; all 11 e2e green (shell untouched); Gallery reviewed at 390/720/1100 in both themes |
| SB.2 | `TopBar`/`ScreenNav`/`TabBar` + More sheet; `lib/verdict.ts` with `homeVerdict`; Home per §10 | `onboarding`, `society`, `commune`, `login` specs; `homeVerdict` tests; Home reviewed at 390 against reference.html's "Assembled" | — done 2026-09-22
| SB.3 | Work, Standing plan, Market per §10 | `work-plan`, `market` specs; three verdicts tested | — done 2026-09-22
| SB.4 | Organizations, Org, Contracts, Society, Archive, Talk, Event | `orgs`, `contracts`, `society` specs; verdicts tested | — done 2026-09-22
| SB.5 | Assembly, BallotBuilder, Coordinator, Store, Ledger; Profile, Societies, Public, Login, Admin, Onboarding; the legacy sweep | `assembly`, `commune`, `coordinator`, `admin`, `login` specs; `make check` with an empty allowlist |

Every card: `pnpm --dir web check` (typecheck, oxlint, style-guard, vitest); `make e2e-web`; the card's screens opened at 390 / 720 / 1100 in light and dark, tabbed through with a keyboard, no page horizontal scroll (§3), at most one `crit` per screen (§4.2); a SESSIONS.md entry; `make push` in the background.

## Rules of the series

- Nothing is removed (§2 rule 3). Each screen card lists every datum the screen shows today and assigns it a layer; a reviewer can check the list against the old screenshot in `reference/`.
- DOM contracts the specs rely on are kept (ADR-0013 §6). A card that must change one changes the spec in the same commit.
- Labels via `useLexicon()`; widgets via `useCapabilities()`; a screen never lists screens (the nav components take items).
- No new mechanics and no server changes. A screen that needs a figure the API does not carry writes the question in `docs/QUESTIONS.md` and shows what it has.
- `Org.tsx` (783 lines) and `Contracts.tsx` (595) are the two big rewrites; SB.4 is the longest session.

## Risks noted

- `--color-*: initial` removes `text-white`/`bg-black`: use `text-accent-on`/`text-bg`.
- No opacity modifiers on tokens (`bg-accent/50`); use the `-soft` tokens.
- `@fontsource` — import the weight files, not `index.css`.
- `<details>` height is not animated (no JS); the bible permits it under reduced motion anyway.
- Dynamic class names (`bg-${tone}-soft`) do not compile under v4's scanner; tones map to literal strings.
