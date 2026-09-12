# 0005 - Share holders include the org itself

**Status:** accepted . **Date:** 2026-09-12 . **Session:** S0.3a

## Context
TDD 6 has `Ownership::Shares(BTreeMap<CitizenId, u64>)`, but TDD 5.3 says newly issued shares "land in the org's own holdings" and TDD 9.3 keeps every legacy firm listed via a standing sale of 100% of its shares. If a householder held those shares, T19's burn on emigration would destroy the firm.

## Decision
`ShareHolder = Citizen(CitizenId) | OrgSelf`. Legacy firms are seeded with 100% `OrgSelf`. The manager sells `OrgSelf` shares on behalf of the org through the normal `OfferSale`/`PlaceOrder(Share)` path; proceeds go to the treasury. `OrgSelf` shares carry no dividend and no vote. Per org, `sum(holdings) + share escrow == issued` is part of `conservation_check`. "Controlling owner" means a citizen holding more than 50% of *issued* shares.

## Consequences
S0.8's share escrow and S0.10's dividends and control tests share one model. A firm with 100% `OrgSelf` has no controlling owner and no dividends until someone buys in.
