# 0011 — The synthetic cohort test ends at the budget

**Status:** accepted · **Date:** 2026-09-18 · **Session:** S1.15a

## Context

ADR-0009 made "an S1.16 cohort has played a full epoch on this build with no blocking defect open" one of Phase 1's exit criteria, and S1.15's done gate repeats it. The cohort has now played four times: the first live run (2026-09-17, six days and seventeen hours, $12.74, stopped by the token cap) and three partial runs on 2026-09-18 (two, two and three days; about $17 together) that were cut off by an invalid key, a harness error in the new subscription-side reflection, and twice by the laptop sleeping, which also exposed D13 (a society's clock stops for good after one failed tick). No run reached `epoch_end`. Each attempt spends real money on the hourly turns, and Chris has reached the budget he set for these tests: "I don't want any more runs, I've reached my budget for these tests. Please use any data you've been able to gather so far and consider this test complete."

What the runs did establish: eleven defects from the first run, all closed on this build (D5 was not one); the twelve raw-id refusal texts gone (zero in 580 turns on 2026-09-18); `unknown_offer` where `HTTP_404` was; dwellings, escrow and the declared dividend visible to the players who asked for them; the reflection path on the subscription working; and D13, which is blocking and unfixed.

## Decision

1. The model-driven cohort test is complete. No further `make agents` run with LLM brains is planned for Phase 1, and none is started without Chris's explicit go-ahead for that attempt.
2. The Phase 1 exit criterion "an S1.16 cohort has played a full epoch on this build with no blocking defect open" is replaced by: "the S1.16 cohort's findings are closed or triaged with no blocking defect open, and `make e2e-agents` (the scripted cohort and the fuzzer, no model calls) plays its two days clean on the build that exits." The scripted cohort is the regression that runs in CI; the model-driven journals are the evidence already gathered.
3. D13 is blocking and is fixed in S1.15 before the load test: the scheduler retries a failed tick with backoff and reports it, instead of returning from the tick loop.
4. The rest of S1.15 stands: the epoch-end sequence and archive, the load test with the §17 budget assertions, the `events` size measurement (T5), the interview guide, and Chris's own Freeport epoch, which was always the evidence nothing synthetic could replace.

## Consequences

- S1.15's done gate in `docs/tdd.md` §18.4 reads as in decision 2. The two partial-run logs of 2026-09-18 are kept under `docs/playtest/runs/` with a short note on what they showed.
- The harness keeps the subscription-side reflection provider (`models.cycle_provider = "claude_code"`, one CLI at a time) for whoever runs the cohort next, and `agents/README.md` says what a run costs.
- Item 13 of the follow-up list (record a model-driven player for the replay fixture) is dropped for Phase 1: it needs a paid run. The scripted fixture stays.
- Tests that assert this: `make e2e-agents` in CI; `scheduler.rs` gets a test that a failed tick is retried (S1.15).
