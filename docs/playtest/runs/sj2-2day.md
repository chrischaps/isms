# Playtest run sj2-2day

Synthetic cohort (S1.16, ADR-0009/0010). 8 players, 748 turns, stopped by epoch_end, 4 turns skipped over 48 hours and 2 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | jev | ~typesafe/jev-latest | 1 |
| wage-maximiser-1 | wage-maximiser | jev | ~typesafe/jev-latest | 1 |
| speculator-1 | speculator | jev | ~typesafe/jev-latest | 1 |
| saver-1 | saver | jev | ~typesafe/jev-latest | 1 |
| slacker-1 | slacker | jev | ~typesafe/jev-latest | 0 |
| borrower-1 | borrower | jev | ~typesafe/jev-latest | 0 |
| landlord-1 | landlord | jev | ~typesafe/jev-latest | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.02 of $1 (525,669 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 504516 | 0 | 0 | 21153 | 0.02 |

## The economy at the end

Epoch 2, day 3, hour 1. Population 48, 8 people, 0 unemployed. Firms 20; credit outstanding 0.00; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 8,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.039380865879503624,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 20,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 32,
 "investment_share": 0.05049088359046283,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 713,
 "materials_to_machines": 36,
 "mean_cycle_wage": 63.67999999999999,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 90.13333333333334,
 "need_fulfillment_rate": 0.975,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 1372,
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

Epoch 2 ended (scheduled) after day 2 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 3 started on its own at 2026-09-24T21:56:10.477Z.

## Rejections

### unknown_offer (28)

| count | text | tools | players |
|---|---|---|---|
| 18 | That offer was taken or withdrawn | accept_offer | borrower-1, founder-1, landlord-1, saver-1, speculator-1, wage-maximiser-1 |
| 10 | There is no such offer | accept_offer | borrower-1, rule-prober-1, saver-1, slacker-1, wage-maximiser-1 |

### insufficient_funds (12)

| count | text | tools | players |
|---|---|---|---|
| 12 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

### insufficient_goods (9)

| count | text | tools | players |
|---|---|---|---|
| 5 | You have 0 Machines | place_order | rule-prober-1 |
| 4 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

### unknown_dwelling (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | There is no such dwelling | move_in, move_out, post_lease_offer | rule-prober-1 |

### not_controlling_owner (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | You do not control Legacy Farm No. 1 | declare_dividend, issue_shares | rule-prober-1 |

### unknown_workplace (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | There is no such workplace | set_labor | rule-prober-1 |

### HTTP_400 (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | unknown instrument gold | place_order | rule-prober-1 |

### self_deal (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | Cannot transfer to yourself | transfer | rule-prober-1 |

### not_party (4)

| count | text | tools | players |
|---|---|---|---|
| 2 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 2 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

### unknown_order (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | There is no such order | cancel_order | rule-prober-1 |

### not_manager (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | You do not manage Legacy Farm No. 1 | post_employment_offer | rule-prober-1 |

### HTTP_403 (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | you are not in this channel | post_message | rule-prober-1 |

### unknown_org (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | There is no such organization | set_plan | rule-prober-1 |

### over_contract_hours (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | The contract allows at most 8 h at the farm at Legacy Farm No. 2 | set_labor | rule-prober-1 |

### not_owner (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | You do not own dwelling no. 1 | post_lease_offer | rule-prober-1 |

### unknown_citizen (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | There is no such citizen | transfer | rule-prober-1 |

### HTTP_404 (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | no contract 1 | terminate_contract | rule-prober-1 |

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
| borrower-1 | 94 | 3 (3%) | 0.82 | 3 of 261 | 2 | 5 | 0 | 229 | 0.00008 |
| founder-1 | 93 | 79 (85%) | 0.57 | 17 of 242 | 3 | 2 | 0 | 325 | 0.00008 |
| landlord-1 | 94 | 86 (91%) | 0.57 | 6 of 190 | 8 | 3 | 0 | 226 | 0.00007 |
| saver-1 | 93 | 85 (91%) | 0.69 | 2 of 144 | 2 | 3 | 0 | 325 | 0.00006 |
| slacker-1 | 94 | 89 (95%) | 0.75 | 0 of 103 | 2 | 1 | 0 | 202 | 0.00005 |
| speculator-1 | 93 | 80 (86%) | 0.61 | 12 of 198 | 10 | 2 | 0 | 327 | 0.00007 |
| wage-maximiser-1 | 93 | 77 (83%) | 0.87 | 0 of 196 | 5 | 7 | 0 | 326 | 0.00007 |

| slot | options chosen |
|---|---|
| housing | rent_cheapest 32 |
| job | take_best 18, switch 6, stay 5 |
| market | buy_materials 6, buy_grain 2, hold 2 |
| plan | keep_plan 561, put_aside 24 |
| venture | sell_output 77, run_quietly 35, buy_materials 12, found_now 6, keep_saving 6, fund_firm 5, post_job_generous 2, post_job 2 |
| work | keep_hours 512, full_normal 19, few_low 2 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 94 turns, making 804 tool calls of which 5 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 322.30 credits, 1 job(s), housed, food 100. Accepted 4 offer(s). Founded 2 org(s). Turns ended by: script 94. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 93 turns and skipped 1, making 585 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 533.10 credits, 1 job(s), housed, food 100. Accepted 3 offer(s). Founded 2 org(s). Turns ended by: script 93. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 94 turns, making 414 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 968.18 credits, 1 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 94. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 94 turns, making 117 tool calls of which 89 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 974.87 credits, 1 job(s), unhoused, food 100. Accepted 2 offer(s). Turns ended by: script 94.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 93 turns and skipped 1, making 338 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 965.47 credits, 1 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 93. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 94 turns, making 317 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 937.51 credits, 1 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 94. Cost $0.00.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 93 turns and skipped 1, making 437 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 723.84 credits, 2 job(s), housed, food 100. Accepted 6 offer(s). Turns ended by: script 93. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 93 turns and skipped 1, making 352 tool calls of which 7 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 965.22 credits, 2 job(s), housed, food 100. Accepted 6 offer(s). Turns ended by: script 93. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).
