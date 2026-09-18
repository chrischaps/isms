# Phase 1 defects from the synthetic cohort (S1.16)

What the cohort's journals and the fuzzer turned up, one row per distinct defect, with a severity S1.15's bug bash triages by: **blocking** (a person cannot get through the core loop, or the server is wrong about money or goods), **major** (a rule is not enforced, or a refusal misleads), **minor** (copy, consistency, a rough edge). Each row names the run that first showed it (`docs/playtest/runs/`), the persona that hit it, and where the fix belongs. Engine and preset text go through a PR; server and web copy go straight to main.

| # | severity | what | seen by | where it bites | fix belongs in | status |
|---|---|---|---|---|---|---|
| D1 | major | `PUT /s/{id}/plan` accepts a standing order for shares in an org that does not exist (`{"share": 999999}`); nothing checks the instrument, so the plan executor will carry a dead order every cycle. | rule-prober (scripted), run e2e-1789697220 | standing plan | `isms-core` `SetStandingPlan` validation (PR) | open, listed in `agents/known-defects.txt` |
| D2 | minor | Refusals name citizens, workplaces, dwellings and orders by raw id: `c47 already works at w2`, `no workplace w999999`, `Citizen(c47) has 965.94`, `no dwelling d999999`, `Citizen(c47) does not own d1`, `no order r1`, `c47 does not control Legacy Farm No. 1`, `the contract allows at most 8 h at w2`. The web translates them on the way in (S1.13f); an agent, the CLI and any log read the ids. Twelve distinct texts in one scripted day. | rule-prober (scripted), run e2e-1789697220 | every rejection | `isms-core` reject text and/or a server-side naming pass mirroring `web/src/lib/names.ts` (PR for engine text) | open |
| D3 | minor | An unknown offer id on `POST /s/{id}/offers/{oid}/accept` answers HTTP 404 with no `code`, while an unknown workplace, dwelling or order on the other commands answers 422 with an engine code (`unknown_workplace`, `unknown_dwelling`, `unknown_order`). A client cannot handle "does not exist" one way. | rule-prober and speculator (scripted), run e2e-1789697220 | offers | `isms-server` `accept_offer` (main) | open |
| D4 | minor | `PUT /s/{id}/labor` past the day's budget is refused as `over_contract_hours` ("the contract allows at most 8 h at w2") before the budget is checked; a person who wants to know the budget rule never sees it from this path. Not wrong, but the probe's expectation (budget) and the answer (contract) differ. | rule-prober (scripted), run e2e-1789697220 | labor | engine copy or the web's labor editor (already shows the budget) | note |

## Not defects, but worth knowing

- `AcceptOffer` of a job the citizen already holds is refused `already_exists`, which is right; the text names ids (D2).
- Transfer to oneself is refused `self_deal` with plain words: the one refusal in the scripted day a person could read as written.
- The society's per-citizen command bucket (5/s, burst 20) was never hit by eight scripted players taking up to four actions an hour.

## Waiting on the model-driven run

The rows above come from scripted players and the fuzzer. The `did_not_understand` data, and the founder, borrower and landlord flows read by a model, arrive with the first `make agents` run (needs `ANTHROPIC_API_KEY`); add its rows here with the run's name.
