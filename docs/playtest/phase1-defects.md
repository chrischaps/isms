# Phase 1 defects from the synthetic cohort (S1.16)

What the cohort's journals and the fuzzer turned up, one row per distinct defect, with a severity S1.15's bug bash triages by: **blocking** (a person cannot get through the core loop, or the server is wrong about money or goods), **major** (a rule is not enforced, or a refusal misleads), **minor** (copy, consistency, a rough edge). Each row names the run that first showed it (`docs/playtest/runs/`), the persona that hit it, and where the fix belongs. Engine and preset text go through a PR; server and web copy go straight to main.

| # | severity | what | seen by | where it bites | fix belongs in | status |
|---|---|---|---|---|---|---|
| D1 | major | `PUT /s/{id}/plan` accepts a standing order for shares in an org that does not exist (`{"share": 999999}`); nothing checks the instrument, so the plan executor will carry a dead order every cycle. | rule-prober (scripted), run e2e-1789697220 | standing plan | `isms-core` `SetStandingPlan` validation (PR) | fixed 2026-09-18: `plan.rs` calls `market::check_instrument` for every standing order (Q108 decided) |
| D2 | minor | Refusals name citizens, workplaces, dwellings and orders by raw id: `c47 already works at w2`, `no workplace w999999`, `Citizen(c47) has 965.94`, `no dwelling d999999`, `Citizen(c47) does not own d1`, `no order r1`, `c47 does not control Legacy Farm No. 1`, `the contract allows at most 8 h at w2`. The web translates them on the way in (S1.13f); an agent, the CLI and any log read the ids. Twelve distinct texts in one scripted day. | rule-prober (scripted), run e2e-1789697220 | every rejection | `isms-core` reject text and/or a server-side naming pass mirroring `web/src/lib/names.ts` (PR for engine text) | fixed 2026-09-18 (main): the server names every refusal for its reader (`crates/isms-server/src/names.rs`, a port of the web's pass, applied in `send`); the engine's texts and `code` are unchanged, and the web's own pass leaves a named sentence alone |
| D3 | minor | An unknown offer id on `POST /s/{id}/offers/{oid}/accept` answers HTTP 404 with no `code`, while an unknown workplace, dwelling or order on the other commands answers 422 with an engine code (`unknown_workplace`, `unknown_dwelling`, `unknown_order`). A client cannot handle "does not exist" one way. | rule-prober and speculator (scripted), run e2e-1789697220 | offers | `isms-server` `accept_offer` (main) | fixed 2026-09-18 (main): 422 `unknown_offer` from `accept` and `withdraw` (Q107 decided) |
| D4 | minor | `PUT /s/{id}/labor` past the day's budget is refused as `over_contract_hours` ("the contract allows at most 8 h at w2") before the budget is checked; a person who wants to know the budget rule never sees it from this path. Not wrong, but the probe's expectation (budget) and the answer (contract) differ. | rule-prober (scripted), run e2e-1789697220 | labor | engine copy or the web's labor editor (already shows the budget) | note; 2026-09-18 the web editor caps each row at its contract's hours and will not save past the day's budget, so neither refusal is met from the screen. The engine's order (contract before budget) is by design |
| D5 | major | Money transferred into an org (`POST /transfers` to `{"org": id}`, committed as `Transferred`) is not there for a `PlaceOrder` on the org's behalf in the same or a later turn: `insufficient_funds: Org(o19) has 0.00`, and once the pre-transfer treasury `8.88`. Either the transfer lands somewhere other than the treasury the escrow reads, or the org view and the escrow disagree. Twice, in separate hours. | landlord (llm), run live-20260917-2136, day 3 h 14 and day 5 h 13 | orgs, market | none in the engine | closed 2026-09-18, not a defect: the event log shows both refused orders came *before* the transfer in the same turn, and the 8.88 matches the ledger to the cent (200.00 in, then the manager's own Materials bids and payroll). What was missing was the view: `OrgView.escrow` now shows money resting in the org's bids, and the org page says so beside the treasury |
| D6 | major | A Builder workplace's `cycle_output` climbed for three days and no dwelling ever appeared anywhere the manager could see: not in `inventory`, not in the ledger, not on the board. Either dwellings complete on a threshold the API never states, or they are created somewhere unlisted. The landlord spent its run on this. | landlord (llm), run live-20260917-2136, days 4–6 | orgs, housing | `OrgView`/`OrgLedgerView` should show dwellings owned and `DwellingBuilt` events; the recipe view should say the unit size (main), and the engine's rule needs writing down | fixed 2026-09-18 (main): the log shows the landlord's firm built six dwellings the API never listed. `OrgView.dwellings`, `RecipeView.produces_asset`, `DwellingBuilt` on the ledger, a Dwellings section on the org page with let/sell; rule in GDD §4.1 (Q110 decided) |
| D7 | minor | `OrgView.payment_missed` is a bare boolean: nothing says what was missed, to whom, or when. | landlord (llm), day 3 h 21 | orgs | `OrgView` (main): the `PaymentMissed` event's seq and the party | fixed 2026-09-18 (main): `OrgView.last_payment_missed` (seq, day, citizen, owed, paid) on the single-org view for its manager, owners and members; the header names it |
| D8 | minor | A payslip's `hours` (2.625, then 2.208, then 2.0) is not the hours the player set (2 or 3 a day at low effort), and the Explain does not say how one became the other; the slacker watched it shrink for four days without an answer. | slacker (llm), days 2–4 | labor, payslips | `Paid.explain` copy (engine, PR): name the rule (tick-hours over the cycle, absent hours) | fixed 2026-09-18 (PR): the formula is `tick_hours / ticks_per_cycle x rate` with all three as inputs, `x treasury / owed` when payday was short; the rule text says an hour not worked is not paid. The slacker's 2.625 was 63 tick-hours over 24: it changed its allocation from 2 to 3 mid-day |
| D9 | minor | An employment offer cannot be withdrawn: `only sale offers and wanted ads can be withdrawn` (HTTP 400). A manager who posted a job at the wrong wage has no way back but to let it be taken. Design gap; Q109. | founder (llm), day 1 h 14 | offers | engine `WithdrawOffer` (PR) once Q109 is decided | fixed 2026-09-18 (PR): `Command::WithdrawOffer` and `Event::OfferWithdrawn`; the server's `DELETE /offers/{oid}` sends it for every kind and infers `on_behalf_of` from the poster |
| D10 | minor | An offer on the board vanishes between reading and accepting (`no offer 81`, twice in one hour, by two players): a householder took it in between. Expected in a live market, but the refusal should say "taken", not "does not exist"; see D3. | borrower and slacker (llm), day 1 h 24 | offers | with D3 | fixed 2026-09-18 (main): "That offer was taken or withdrawn" when the id was once issued |
| D11 | minor | Comfort sat at 0 for days with no visible consequence; nothing the player can read says what Comfort governs. | slacker (llm), day 4 h 20 | needs | Welcome Brief / need tooltips copy (main) | fixed 2026-09-18: the Comfort tooltip says it is a third of wellbeing and changes neither output nor pay (main); the Freeport Welcome Brief says Food and Shelter keep you working and Comfort is what the Chronicle counts as living well (PR #61) |
| D12 | minor | `DeclareDividend` refused `already_exists` ("a dividend is already declared this cycle") and nothing on the org view showed the one declared earlier that day, so the refusal read as a bug. | founder (llm), run live-20260917-2136, day 4 h 21 | orgs | `OrgView` (main) | fixed 2026-09-18 (main): `OrgView.declared_dividend`; the org page shows it and hides the form until the day ends; the cost preview now uses citizen-held shares, as the engine does |
| D13 | blocking | A society's clock stops for good after one failed tick: `scheduler.rs` returns from the tick loop on the first error (`tick failed: database: pool timed out while waiting for an open connection`, seen when the laptop woke from sleep with the pool's connections gone), and nothing restarts it; players kept taking turns against a world that never moved again. | harness run live-20260918-1148 (log kept), 2026-09-18 | scheduler | `isms-server` scheduler (main): retry a failed tick with backoff and report it, instead of returning; the S1.15 hardening card | open |

## Not defects, but worth knowing

- `AcceptOffer` of a job the citizen already holds is refused `already_exists`, which is right; the text names ids (D2).
- Transfer to oneself is refused `self_deal` with plain words: the one refusal in the scripted day a person could read as written.
- The society's per-citizen command bucket (5/s, burst 20) was never hit by eight scripted players taking up to four actions an hour.

## From the first model-driven run (live-20260917-2136)

Eight players, five on Sonnet 5 with Opus 5 reflections, six days and seventeen hours, 855 turns, $12.74, stopped by the token cap (`docs/playtest/runs/live-20260917-2136.md`). D5–D11 are its rows. Not defects, but from the same run:

- The borrower reported "set_labor refused right after accept_offer succeeded"; its own journal shows the calls in the other order. A model narrating its turn is not a witness to it; the journal is.
- The wage-maximiser was refused 120 times with `not_assigned: c45 has no position at w20`: its script kept working a contract in its notice period. A harness bug, fixed (contracts must be `active`); the game was right, and the text names ids (D2).
- The slacker reported the situation header listing `defaulted, destitute, in_hardship, options_narrowed` while `home` showed them false. A harness bug (the header printed every flag key), fixed. It found the harness's bug the same way it would find the game's.
- The Anthropic account ran out of credit at the end of day 2 and every model turn failed for a day until it was topped up; the harness now stops a player on a billing or key error instead of retrying every hour.

## Follow-ups from the playtests, in order

Everything the playtests so far have left to do, gathered from this file, the run report (`docs/playtest/runs/live-20260917-2136.md`), `docs/QUESTIONS.md` Q105–Q110, the S1.16 entry in `docs/SESSIONS.md`, and the day Chris played by hand (2026-09-17, ADR-0009). The order is the order S1.15 should take them. Each line says where it came from and where the fix lives; engine work goes through a PR.

**Engine, one PR each**
1. ~~D5~~ closed 2026-09-18: verified against the log, not a defect; the escrow line on `OrgView` is the fix (see the row).
2. ~~D6~~ done 2026-09-18 on main (see the row); no engine change was needed.
3. D1 — `SetStandingPlan` accepts a standing order for shares in an org that does not exist; refuse it like `PlaceOrder` refuses an unknown instrument (Q108). Remove the line from `agents/known-defects.txt` when fixed. Source: fuzzer.
4. ~~D8~~ done 2026-09-18 (PR).
5. ~~D2~~ done 2026-09-18 on main: a server-side naming pass (see the row).
6. ~~D9~~ done 2026-09-18 (PR; server and web follow-through on main).

**Server and web, straight to main**
7. ~~D3 and D10~~ done 2026-09-18 on main (Q107 decided).
8. ~~D7~~ done 2026-09-18 on main.
9. ~~D11~~ done 2026-09-18: the need tooltip (main) and the Freeport Welcome Brief (PR #61).
10. ~~D12~~ done 2026-09-18 on main: the declared dividend shows on the org view (row D12).
11. ~~D4~~ note closed 2026-09-18: the labor editor caps rows and blocks an over-budget save.

**Harness (`agents/`)**
12. ~~A second `make agents` run~~ three partial runs on 2026-09-18 (`docs/playtest/runs/2026-09-18-partial-runs.md`), none past day 3, about $17; the cohort test is complete at Chris's budget (ADR-0011). They confirmed D2, D3, D6, D8 and D12 on the build and found D13.
13. ~~Record a model-driven player for the replay fixture~~ dropped for Phase 1 (ADR-0011): it needs a paid run.
14. ~~A Makefile guard~~ done 2026-09-18.
15. An OpenAI-compatible provider behind the `LlmProvider` seam for local models; designed, not built (ADR-0010). Only if a free run becomes worth its noise.

**S1.15 proper (the card in `docs/tdd.md` §18.4)**
16. The epoch-end sequence, the archive that Phase 3 consumes, the load test with the TDD §17 budget assertions, and the `events` size measurement (T5).
17. The interview guide, `docs/playtest/phase1.md`, written for a cohort of one to three, from GDD §18/§19 ("what did you feel", not "was it fun").
18. Chris plays a Freeport epoch and writes down what it was like in the guide's terms. This is the exit criterion nothing else can stand in for (ADR-0009).

**Already fixed, for the record**
- From Chris's day of play (S1.13f, web): rejections printed with raw ids on screen (now named on the way in), a price history drawn from the previous epoch, a job taken with no confirmation, an ask side that looked empty because it sold out each hour.
- From the live run (S1.16, harness): the situation header listed every flag as set; the wage-maximiser worked a contract in its notice period; a billing refusal from the model API now stops the player; the token budget's default counts cache reads.
