# 0013 — The Companion style bible governs `web/`; the refresh runs as SB.1–SB.5 before S2.11

**Status:** accepted · **Date:** 2026-09-22 · **Session:** SB.1

## Context

TDD §11 and GDD §15 gave the client a deliberately plain, print-like shell — paper, ink, one accent, a serif display face — and every screen presented its facts at one level. The S1.16 cohort's journals and Chris's own reading of the build found it dense and hard to skim: a newcomer cannot tell from Home whether they are alright. On 2026-09-22 Chris reviewed three directions for the Your Accounts screen and chose "Companion": a five-layer hierarchy (Verdict → Glance → Facts → Detail → Explain), phone-first layout with a bottom TabBar, a token-only palette in light and dark, two self-hosted faces, and a named component catalog. The direction is written up as `docs/style.md` with `docs/style/tokens.css`, `docs/style/tailwind.preset.js` and a rendered `docs/style/reference.html`.

The client is on Tailwind v4, configured in CSS; the bible's `tailwind.preset.js` is a v3 preset. No shared primitives existed — every screen is ad-hoc class strings — and eleven Playwright specs key on roles, `data-testid`s, labels and text.

## Decision

1. **`docs/style.md` governs `web/`.** Its principles (§1), hierarchy (§2), tokens (§4), component catalog (§7) and copy rules (§8) are the client's conventions; TDD §11 points to it. Vocabulary still comes from the lexicon and mounting from the capabilities hook; the bible changes how a screen is arranged, not what it may show (§2 rule 3: nothing is removed, everything has a level).
2. **The refresh is its own card series, SB.1–SB.5, run before S2.11** (docs/plans/style-refresh.md): SB.1 foundation (tokens, fonts, theme, primitives, Gallery, style guard); SB.2 shell and Home; SB.3 Work, Standing plan, Market; SB.4 Organizations, Contracts, Society, Archive, Talk, Event; SB.5 the Commune and account screens and the legacy sweep. S2.11's money-less kit pass then lands once, on the new kit. Every card is web/docs-only and commits to main.
3. **Verdicts are shell voice.** Every screen's Verdict sentence is derived in `web/src/lib/verdict.ts` from the screen's payload, in the neutral second person of §8.1, with the lexicon's nouns. It is not preset copy, which is the society's voice and is served by the server; no server changes are part of the series.
4. **Tokens in Tailwind v4 form.** `tailwind.preset.js` is kept as reference only. `web/src/styles/tokens.css` reproduces its intent: theme-invariant values in `@theme static`; the bible's colour tokens on `:root` under their own names; an `@theme inline` block mapping `--color-*` onto `var(--token)` with the stock palette removed. A `style-guard` script in `pnpm check` fails on a hex, `rgb()`, a `dark:` variant or a pre-Companion class name outside a shrinking allowlist.
5. **One deviation from §4.5:** an explicit dark choice is selected by `[data-theme="dark"]`, not `:root[data-theme="dark"]`, so the Gallery can nest a dark island in a light page and show both themes side by side. The OS-preference rule is unchanged.
6. **The DOM contracts the e2e specs rely on are kept** through every card: `nav aria-label="Sections"`, `data-testid`s, accessible names, `role="dialog"` on a `Num`'s rule, the Gallery's section titles. A card that must change one changes the spec in the same commit.

## Consequences

- TDD §18.5 gains the SB.1–SB.5 cards ahead of S2.11; S2.11's read list adds `docs/style.md`. The Phase 2 exit gate is unchanged.
- `web/src/components/` grows the §7 catalog; `Meter` keeps its role and testid and gains the §4.2 tones; `TimeSeries` reads its colours from the tokens at mount and on theme change, which removes the last hex literals from component code.
- The pre-Companion names (`text-bad`, `bg-paper-2`, `.num`, `.rule`, `.explain`, the unlayered table gutter) are aliased until SB.5 empties the alias block and the guard's allowlist together.
- `reference/` (screenshots of the old build and the three-direction mockup) stays untracked; the bible's own files moved to `docs/style/`.
