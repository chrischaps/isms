# Playtest run sj1-2day

Synthetic cohort (S1.16, ADR-0009/0010). 8 players, 368 turns, stopped by epoch_end, 0 turns skipped over 46 hours and 1 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | jev | ~typesafe/jev-latest | 0 |
| wage-maximiser-1 | wage-maximiser | jev | ~typesafe/jev-latest | 0 |
| speculator-1 | speculator | jev | ~typesafe/jev-latest | 0 |
| saver-1 | saver | scripted | - | 0 |
| slacker-1 | slacker | scripted | - | 0 |
| borrower-1 | borrower | scripted | - | 0 |
| landlord-1 | landlord | jev | ~typesafe/jev-latest | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.01 of $1 (224,253 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 212272 | 0 | 0 | 11981 | 0.01 |

## The economy at the end

Epoch 1, day 3, hour 1. Population 48, 8 people, 0 unemployed. Firms 19; credit outstanding 0.00; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 8,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.08044522741832161,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 19,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 32,
 "investment_share": 0.0502092050209205,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 717,
 "materials_to_machines": 36,
 "mean_cycle_wage": 63.2,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.8,
 "need_fulfillment_rate": 0.8,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 1352,
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

Epoch 1 ended (scheduled) after day 2 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-24T20:49:59.281Z.

## Rejections

### unknown_offer (7)

| count | text | tools | players |
|---|---|---|---|
| 5 | That offer was taken or withdrawn | accept_offer | founder-1, speculator-1, wage-maximiser-1 |
| 2 | There is no such offer | accept_offer | rule-prober-1 |

### insufficient_funds (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

### insufficient_goods (4)

| count | text | tools | players |
|---|---|---|---|
| 2 | You have 0 Machines | place_order | rule-prober-1 |
| 2 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

### not_party (4)

| count | text | tools | players |
|---|---|---|---|
| 2 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 2 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

### not_controlling_owner (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | You do not control Legacy Farm No. 1 | declare_dividend, issue_shares | rule-prober-1 |

### unknown_workplace (3)

| count | text | tools | players |
|---|---|---|---|
| 3 | There is no such workplace | set_labor | rule-prober-1 |

### HTTP_400 (3)

| count | text | tools | players |
|---|---|---|---|
| 3 | unknown instrument gold | place_order | rule-prober-1 |

### self_deal (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | Cannot transfer to yourself | transfer | rule-prober-1 |

### unknown_dwelling (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | There is no such dwelling | move_in | rule-prober-1 |

### unknown_order (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | There is no such order | cancel_order | rule-prober-1 |

### not_manager (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | You do not manage Legacy Farm No. 1 | post_employment_offer | rule-prober-1 |

### HTTP_403 (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | you are not in this channel | post_message | rule-prober-1 |

### unknown_org (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | There is no such organization | set_plan | rule-prober-1 |

### not_owner (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | You do not own dwelling no. 1 | post_lease_offer | rule-prober-1 |

### over_contract_hours (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | The contract allows at most 8 h at the farm at Legacy Farm No. 2 | set_labor | rule-prober-1 |

### already_exists (1)

| count | text | tools | players |
|---|---|---|---|
| 1 | You already work at the farm at Legacy Farm No. 2 | accept_offer | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| founder-1 | 46 | 42 (91%) | 0.45 | 0 of 92 | 0 | 2 | 0 | 301 | 0.00006 |
| landlord-1 | 46 | 44 (96%) | 0.63 | 0 of 46 | 0 | 0 | 0 | 290 | 0.00004 |
| speculator-1 | 46 | 42 (91%) | 0.66 | 0 of 51 | 0 | 1 | 0 | 302 | 0.00005 |
| wage-maximiser-1 | 46 | 0 (0%) | 0.59 | 0 of 46 | 0 | 2 | 0 | 295 | 0.00004 |

| slot | options chosen |
|---|---|
| job | take_best 9 |
| market | hold 4, buy_grain 1 |
| venture | keep_saving 46 |
| work | none 129, full_normal 46 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, scripted) took 46 turns, making 189 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 672.72 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 46.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 46 turns, making 143 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 969.44 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 46 turns, making 95 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 973.47 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**rule-prober-1** (rule-prober, scripted) took 46 turns, making 58 tool calls of which 44 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 969.54 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46.

**saver-1** (saver, scripted) took 46 turns, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 968.23 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46.

**slacker-1** (slacker, scripted) took 46 turns, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 952.02 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 46 turns, making 97 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 728.45 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 46 turns, making 51 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 966.82 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

## Harness notes

Every turn ended with end_turn (or a script).
