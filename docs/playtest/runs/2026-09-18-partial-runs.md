# The partial runs of 2026-09-18

Three model-driven runs were started on the bug-bash build (main at `fa82c3c` and after) and none reached the epoch's end; the cohort test is complete at Chris's budget (ADR-0011). Two of the three left their logs (`live-20260918-1148.log`, `live-20260918-1235.log`, beside this note); the journals went with the run directories. This is what those logs can support.

## The runs

| run | brains | got to | stopped by | turns | API spend |
|---|---|---|---|---|---|
| live-20260918-1110 | 5 LLM (Sonnet 5) + 3 scripted | day 2 | every reflection failed: the CLI refused zod's `$schema` key (fixed, `4495518`) | ~140 | ~$4 |
| live-20260918-1148 | same | day 2, hour 24 | the laptop slept at 11:55 local; on wake the server's first tick timed out on the pool and the scheduler stopped (D13) | 283 | $6.47 |
| live-20260918-1235 | same | day 3, hour 1 | the laptop slept again at 12:43 local, the same way | 296 | $6.12 |

Reflections ran on Chris's Claude subscription through `claude -p` (one at a time after the first attempt showed five at once stall the machine); every day-end reflection in the last two runs came back with notes, except one issued during a sleep.

## What the build did differently from 2026-09-17

- **No refusal named an id.** 114 refusals across the two logs, zero with a raw `c`/`w`/`o`/`d` token; the fuzzer's twelve catalogued texts now read "There is no such workplace", "You do not control Legacy Farm No. 1", "You have 0 Machines", "Cannot transfer to yourself" (D2).
- **An unknown offer is `unknown_offer`**, not `HTTP_404` (D3): 2 per run, from the rule-prober's probe.
- **The landlord saw its firm's dwellings and let them.** By day 2 it had five lease offers posted on the firm's own dwellings (ids 36, 30, 38, 3, 1) at 8.00 a day and was housed by a lease itself (D6: `OrgView.dwellings`, let on the org's behalf).
- **The founder saw its declared dividend** on the org view ("dividend of 50/share already declared") and did not try to declare it twice (D12).
- **Payslips said tick-hours** and the slacker read the pro-rata ("payslip shows hourly rate 800c applied pro-rata") without asking what the hours were (D8).
- The remaining `HTTP_400` rows are the rule-prober's "unknown instrument gold", a request-shape refusal from the server, which is the right status for a string that is not an instrument.

## What the runs did not reach

- Days 3 to 7: the mid-epoch economy, the founder's second week, the borrower's default, epoch end. The first run of 2026-09-17 remains the only evidence past day 3.
- Item 13 (a model-driven replay fixture) was not recorded.

## Found on the way

- **D13 (blocking):** a society's clock stops for good after one failed tick; `scheduler.rs` returns from its loop on the first error. Both sleeping runs show the same shape: `tick failed: database: pool timed out while waiting for an open connection` on wake, then no tick ever again while players kept taking turns. Fix in S1.15 before the load test.
- The machine, not the harness: Windows logged sleep at 11:23, 11:55 and 12:43 local, fifteen minutes after the last input each time, matching every gap in the logs to the second. A run needs the sleep timeout off.
