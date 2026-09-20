# 0012 — The human playtests move to a Phase 5, after every society is built

**Status:** accepted · **Date:** 2026-09-19 · **Session:** S1.15d (plan) · supersedes ADR-0009 decision 1 and ADR-0011 decision 4 in part

## Context

ADR-0009 kept two human tests when it shrank the cohort: Chris's own Freeport epoch, written up in the terms of `docs/playtest/phase1.md`, as the last Phase 1 exit criterion; and the "felt difference" week of Phase 2, the same people living a week in Freeport and then a week in the Commune. ADR-0011 restated the first as "the evidence nothing synthetic could replace". Everything else on the Phase 1 gate is met on main as of 2026-09-18: the §17 budget assertions pass, an epoch end writes an archive, the S1.16 cohort's findings are closed with no blocking defect open, and `make e2e-agents` plays its two days clean in CI.

The project's thesis is a comparison between societies (GDD §1, §17). A solo Freeport epoch now, and a Commune epoch months later on a different build with a different guide, is not that comparison; it is two anecdotes with a long gap between them, and the Freeport one would be played against a client that Phase 2's visual pass and Phase 3's Observatory will change. Chris has decided to play once all five societies exist.

## Decision

1. **Phase 1 is closed** on the automated gates already met. The line "Chris has played a Freeport epoch and filled in `docs/playtest/phase1.md`" is struck from S1.15's done gate (TDD §18.4), and item 18 of `docs/playtest/phase1-defects.md` moves to Phase 5.
2. **Phases 2, 3 and 4 exit on automated gates only**: `make sim-check` for the phase's presets, Playwright on every seed the phase adds, `make e2e-agents` in the phase's presets, the lexicon and Chronicle copy tests. No human playtest sits on any of their gates. The "felt difference" week is struck from the Phase 2 milestone.
3. **A new Phase 5, the playtest.** One to three humans play consecutive short epochs in each of the five canonical presets on one build, keep a diary a day, and are interviewed once per preset from a guide that generalises `docs/playtest/phase1.md` (the Freeport section as written; four more in the same shape). The felt-difference comparison runs across all five societies, not Freeport-then-Commune. S1.14 (deployment) is taken up in Phase 5 if a second human is remote, or earlier when one is real. Phase 5 exits when the guide is filled in for every player and every preset, the defects are filed with severities, and a "what v2 must answer" list exists.
4. **`docs/playtest/phase1.md` stays as written** and is not filled in now. Each phase from 2 on records what only a person can judge as `[H]` marks in `docs/playtest/phase<N>-visual.md`; Phase 5 reads them first.
5. **Phase 2 is cut into cards now**, from the GDD (§6.2, §8, §12), the engine scaffolding that already exists, and the S1.16 journals, since the interviews TDD §18.5 waited for will not happen before it. Phases 3 and 4 are cut at their own kickoff.

## Consequences

- TDD §18.4 S1.15 is done; §18.5 carries the S2.1–S2.11 cards, the Phase 2 exit gate and a Phase 5 paragraph; §18.6 chains the cards and adds the Phase 5 node. GDD §18's table gains the Phase 5 row and drops "retention" and "same cohort" from Phases 1 and 2. `CLAUDE.md` names the new order.
- Retention is dropped as a criterion everywhere; three people cannot measure it, and Phase 5's interviews ask for moments, not verdicts (GDD §19).
- The first real stranger's session, whenever it comes, is still more informative than anything before it (ADR-0009). Phase 5 is the earliest point the project asks a person to spend a week on it, and by then the ask is "live in five societies", which is the offer the concept makes.
- `docs/plans/phase2.md` is the kickoff plan; Q116–Q122 in `docs/QUESTIONS.md` hold the provisional answers the cards start from.
