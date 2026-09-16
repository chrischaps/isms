# 0007 - Operator routes and a held clock

**Status:** accepted . **Date:** 2026-09-16 . **Session:** S1.13c

## Context
TDD 10.2 lists one operator action, `EndEpoch`, "never from clients", and TDD 9.2 has the scheduler run missed ticks back-to-back after downtime. Chris asked for an admin mode that can pause the simulation while he plays and tests. Neither document says who an operator is or how a pause differs from downtime.

## Decision
Operators are account emails named in `ISMS_OPERATORS`; the API grows `/admin/*` routes (list, pause, resume, step, tick-seconds, end-epoch) that refuse anyone else with 403, so the web client and the CLI use the same surface (TDD 10.2's one-API rule). A pause is a hold on the scheduler, persisted as `societies.status = 'paused'` so a restart keeps it; a paused society still loads and answers reads and commands. Resume re-anchors `tick_origin` so the next tick is due one tick length from now: a pause is not downtime and nothing is caught up. Step resolves one tick while held. `EndEpoch` is sent as `Actor::System` from the operator's session and logged at warn.

## Consequences
`SocietyEntry` carries an `Arc<Control>` instead of a copied `Schedule`; the summary's `tick_seconds` and `next_tick_at` read the live clock and `next_tick_at` is `null` while held. Commands during a pause still apply (a citizen can trade while the world stands still), which is what a test wants and what a playtest should be told. `tests/admin.spec.ts` and the scheduler's unit test assert the hold and the re-anchoring.
