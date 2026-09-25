# Playtest run sj2-rec

Synthetic cohort (S1.16, ADR-0009/0010). 1 players, 22 turns, stopped by epoch_end, 0 turns skipped over 22 hours and 0 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | jev | ~typesafe/jev-latest | 0 |

Spend: $0.00 of $1 (39,191 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 37575 | 0 | 0 | 1616 | 0.00 |

## The economy at the end

Epoch 1, day 2, hour 1. Population 41, 1 people, 0 unemployed. Firms 18; credit outstanding 0.00; food last 1.31; price index 1.13.

Last day's aggregates:

```json
{
 "active_humans": 1,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.005171508575428918,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 18,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 39,
 "investment_share": 0.06,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 500,
 "materials_to_machines": 30,
 "mean_cycle_wage": 61.129999999999974,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 95.8,
 "need_fulfillment_rate": 1,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1309012875536482,
 "rations_issued": 0,
 "real_output": 1469,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 0
}
```

## The epoch's end

Epoch 1 ended (scheduled) after day 1 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-24T22:03:37.031Z.

## Rejections

None.
## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| founder-1 | 22 | 20 (91%) | 0.62 | 1 of 46 | 0 | 0 | 0 | 248 | 0.00007 |

| slot | options chosen |
|---|---|
| housing | rent_cheapest 1 |
| job | take_best 1 |
| plan | put_aside 1 |
| venture | keep_saving 22 |
| work | keep_hours 20, full_normal 1 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**founder-1** (founder, jev on ~typesafe/jev-latest) took 22 turns, making 94 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 942.36 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 22. Cost $0.00.

## Harness notes

Every turn ended with end_turn (or a script).
