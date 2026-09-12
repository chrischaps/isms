# 0003 - Epoch seeding is events, not a World

**Status:** accepted . **Date:** 2026-09-12 . **Session:** S0.3a

## Context
TDD 5.5 step 9 says `start_epoch(world, epoch + 1) -> World`, "a pure function producing fresh material state". TDD 0 says state changes only by applying events, and S1.1's loader rebuilds a `World` from a snapshot plus events. A `World` that is not a fold of events cannot be replayed, and the seeding of epoch 0 (S0.12) has the same problem.

## Decision
`start_epoch(world, rules, epoch) -> Vec<Event>`. Seeding is ordinary events with `actor = System`: `EpochStarted { epoch }`, then `OrgFounded`, `WorkplaceAdded`, `DwellingBuilt`, `Seeded { holder, asset }`, `HouseholderJoined`, and so on. Epoch 0 is `SocietyCreated` -> `EpochStarted { 0 }` -> seeding events. `apply(EpochStarted)` resets material state (pantries, inventories, books, contracts, offers) and keeps the constitution, params, and the citizen roster with every human dormant.

## Consequences
Replay is a pure fold from `SocietyCreated` for the life of a society. Seeding is visible in the research export. The S0.4 tick skeleton owns `start_epoch`'s signature; S0.12 fills its body.
