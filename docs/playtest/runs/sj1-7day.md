# Playtest run sj1-7day

Synthetic cohort (S1.16, ADR-0009/0010). 8 players, 1144 turns, stopped by epoch_end, 184 turns skipped over 166 hours and 6 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | jev | ~typesafe/jev-latest | 27 |
| wage-maximiser-1 | wage-maximiser | jev | ~typesafe/jev-latest | 24 |
| speculator-1 | speculator | jev | ~typesafe/jev-latest | 11 |
| saver-1 | saver | scripted | - | 16 |
| slacker-1 | slacker | scripted | - | 10 |
| borrower-1 | borrower | scripted | - | 29 |
| landlord-1 | landlord | jev | ~typesafe/jev-latest | 42 |
| rule-prober-1 | rule-prober | scripted | - | 25 |

Spend: $0.03 of $1 (663,877 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 627570 | 0 | 0 | 36307 | 0.03 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 48, 8 people, 5 unemployed. Firms 20; credit outstanding 0.00; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 8,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.06720545977011505,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 20,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 32,
 "investment_share": 0.08527131782945736,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 516,
 "materials_to_machines": 44,
 "mean_cycle_wage": 60.3781081081081,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.71666666666667,
 "need_fulfillment_rate": 0.8,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 2179,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 5
}
```

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 61 s. Epoch 2 started on its own at 2026-09-24T21:19:21.962Z.

## Rejections

### insufficient_funds (20)

| count | text | tools | players |
|---|---|---|---|
| 20 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

### insufficient_goods (14)

| count | text | tools | players |
|---|---|---|---|
| 7 | You have 0 Machines | place_order | rule-prober-1 |
| 7 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

### not_party (14)

| count | text | tools | players |
|---|---|---|---|
| 7 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 7 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

### not_controlling_owner (13)

| count | text | tools | players |
|---|---|---|---|
| 13 | You do not control Legacy Farm No. 1 | declare_dividend, issue_shares | rule-prober-1 |

### unknown_offer (12)

| count | text | tools | players |
|---|---|---|---|
| 7 | There is no such offer | accept_offer | rule-prober-1 |
| 5 | That offer was taken or withdrawn | accept_offer | founder-1, speculator-1, wage-maximiser-1 |

### unknown_workplace (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | There is no such workplace | set_labor | rule-prober-1 |

### HTTP_400 (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | unknown instrument gold | place_order | rule-prober-1 |

### self_deal (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | Cannot transfer to yourself | transfer | rule-prober-1 |

### unknown_dwelling (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | There is no such dwelling | move_in | rule-prober-1 |

### unknown_order (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | There is no such order | cancel_order | rule-prober-1 |

### not_manager (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | You do not manage Legacy Farm No. 1 | post_employment_offer | rule-prober-1 |

### HTTP_403 (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | you are not in this channel | post_message | rule-prober-1 |

### unknown_org (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | There is no such organization | set_plan | rule-prober-1 |

### not_owner (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | You do not own dwelling no. 1 | post_lease_offer | rule-prober-1 |

### over_contract_hours (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | The contract allows at most 8 h at the farm at Legacy Farm No. 2 | set_labor | rule-prober-1 |

### already_exists (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | You already work at the farm at Legacy Farm No. 2 | accept_offer | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| founder-1 | 139 | 133 (96%) | 0.74 | 8 of 278 | 0 | 2 | 0 | 322 | 0.00006 |
| landlord-1 | 124 | 122 (98%) | 0.61 | 0 of 124 | 0 | 0 | 0 | 298 | 0.00004 |
| speculator-1 | 155 | 150 (97%) | 0.68 | 0 of 157 | 0 | 2 | 0 | 282 | 0.00004 |
| wage-maximiser-1 | 142 | 0 (0%) | 0.55 | 0 of 142 | 0 | 1 | 0 | 288 | 0.00004 |

| slot | options chosen |
|---|---|
| job | take_best 9 |
| market | buy_grain 1, hold 1 |
| venture | run_quietly 129, buy_materials 8, found_now 2 |
| work | none 408, full_normal 143 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, scripted) took 137 turns and skipped 29, making 598 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 835.52 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 137.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 139 turns and skipped 27, making 553 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 882.04 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 139. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 124 turns and skipped 42, making 251 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1136.27 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 124. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 141 turns and skipped 25, making 175 tool calls of which 139 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1132.34 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 141.

**saver-1** (saver, scripted) took 150 turns and skipped 16, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1131.03 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 150.

**slacker-1** (slacker, scripted) took 156 turns and skipped 10, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 954.82 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 156.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 155 turns and skipped 11, making 316 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 887.97 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 155. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 142 turns and skipped 24, making 146 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1132.29 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 142. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).
