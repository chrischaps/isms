# 0004 - The Seeded ledger term and legacy treasuries

**Status:** accepted . **Date:** 2026-09-12 . **Session:** S0.3a

## Context
Legacy firms need 18 workplaces at 20 Materials each (TDD 5.3, App. A) in a society that has produced nothing, and TDD 5.8's goods conservation `holdings + escrow == produced - consumed - depreciated` has no term for anything that did not come from production. `Minted`/`Burned` are defined only for money.

## Decision
`LedgerMeta` tracks per-good `seeded`, `produced`, `consumed`, `depreciated`, and `burned` counters and per-money `minted` and `burned`. Goods conservation is `sum(holdings) + sum(escrow) == produced + seeded - consumed - depreciated - burned`. A `Seeded { holder, asset }` event is the only way to add stock without production, and only `start_epoch` emits it. Seeding bypasses founding costs. Legacy orgs receive `legacy_treasury_credits` (384 = about one cycle of payroll at full staffing, `[new tunable]`), counted in `minted`.

## Consequences
`conservation_check` stays exact from the first event. Householder emigration burns to `burned` (T19). Tuning owns the seed sizes.
