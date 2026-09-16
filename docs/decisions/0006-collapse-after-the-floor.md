# 0006 - Collapse only after the floor has been reached

**Status:** accepted . **Date:** 2026-09-16 . **Session:** S1.13b

## Context
GDD 5 puts the population floor at 40 citizens counting humans and AI fill, and GDD Q7 and Appendix A end an epoch by "collapse" when active humans are below the floor for 5 consecutive cycles; TDD 5.5 phase 9 implements the second reading. Householders fill the population to the floor by construction, so the first reading never fires, and the second ends every society with fewer than 40 people after five cycles: on 2026-09-16 two dev societies died with one human present, and the S1.15 playtest cohort of 10 to 20 would have died the same way. Chris decided that a fully AI economy may keep running, and that "abandoned" means a society that first reached the threshold and then fell below it.

## Decision
`World.meta.reached_floor` records whether active humans have been at or above `params.population.floor` at any cycle end of the epoch (reset by `start_epoch`). Phase 8m runs the `low_population_cycles` counter only when `reached_floor` is set; phase 9 is unchanged (collapse when the counter reaches `collapse_cycles`, only with `collapse_enabled`). `CycleClosed` carries `reached_floor` (serde default `false`) so replay and the fold agree. No new tunable: the threshold is the floor itself.

## Consequences
A society that never draws 40 humans runs to its scheduled end as an AI economy; one that had 40 and lost them for five cycles ends with the Chronicle as before. `collapse_enabled = false` stays the simulator switch and is no longer needed for dev seeds. `tests/collapse.rs` asserts both halves. Snapshots and goldens change shape (a new `Meta` field, a new `CycleClosed` field): goldens are regenerated in this session, and databases holding pre-ADR snapshots must be reseeded, as Phase 1 has no snapshot versioning yet.
