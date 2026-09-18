# Load run load-20260918-epoch

S1.15 load test (TDD 17): 1 lab Freeport(s), 100 householders and 20 scripted players (the harness's brains, ADR-0010) at `tick_seconds=5`, 42-day epoch; 1008 ticks resolved; 259342 events in the database.

| budget line | measured | budget | |
|---|---|---|---|
| tick, p99 (max) with persistence | 253 ms (522 ms) over 1008 ticks; 2 interrupted by sleep, listed below | <= 500 ms | pass |
| command p99, actor side (validate, persist, apply) | not logged (needs isms_server::actor=debug) | <= 50 ms | FAIL |
| World, canonical bytes | 0.03 MB (snapshot 0.01 MB zstd) | <= 16 MB | pass |
| events per 42-day epoch, one society | 0.128 GB from 127.7 MB of table after 1008 of 1008 ticks (126.7 kB per tick, 257973 events) | <= 3 GB | pass |
| RSS, max over the run (1 societies) | 0.22 GB over 58 samples | <= 2 GB for ten societies | pass |

## Events by kind (the society under load)

| kind | count | payload bytes | share |
|---|---|---|---|
| OrderPlaced | 111163 | 40,003,550 | 45.7% |
| Trade | 108278 | 28,316,440 | 32.3% |
| TickResolved | 1008 | 7,445,532 | 8.5% |
| Produced | 7553 | 5,963,631 | 6.8% |
| Paid | 2549 | 1,324,344 | 1.5% |
| PlanChanged | 2990 | 1,029,952 | 1.2% |
| RentPaid | 3794 | 1,001,616 | 1.1% |
| CitizenSeen | 7880 | 788,000 | 0.9% |
| MachinesInstalled | 3204 | 257,584 | 0.3% |
| EmploymentAccepted | 860 | 227,040 | 0.3% |
| OrderExpired | 2177 | 223,733 | 0.3% |
| LeaseOffered | 757 | 139,288 | 0.2% |

`TickResolved` is 8% of the payload bytes, 7.4 kB each (T5: the JSONB question). The table with its index and TOAST is 1.46x the payload bytes.

## Tick time

p50 43 ms, p90 100 ms, p99 253 ms, max 522 ms over 1008.

Interrupted by the machine sleeping (the clock kept counting; the scheduler caught up afterwards):

- tick 108: 47 min, logged at 2026-09-18T21:48:11.471806Z
- tick 894: 28 min, logged at 2026-09-18T22:34:49.408899Z

## Command time

Actor side: not logged.

Client side, HTTP included: commands p50 4 ms, p90 11 ms, p99 1020 ms, max 1025 ms over 162; reads p50 6 ms, p90 13 ms, p99 83 ms, max 1281 ms over 34008. 29 calls took a second or more: the harness's one retry after a 429 from the per-citizen rate limit, which the scripted players hit when several act in the same tick.

Measured on the developer's machine with the dev Postgres in Docker Desktop, not on the 2 vCPU / 4 GB VPS the budget names; `synchronous_commit` is at its default.
