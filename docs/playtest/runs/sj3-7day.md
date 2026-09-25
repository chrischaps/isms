# Playtest run sj3-7day

Synthetic cohort (S1.16, ADR-0009/0010). 16 players, 2652 turns, stopped by epoch_end, 4 turns skipped over 166 hours and 6 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | jev | ~typesafe/jev-latest | 0 |
| lender-1 | lender | jev | ~typesafe/jev-latest | 0 |
| builder-1 | builder | jev | ~typesafe/jev-latest | 0 |
| landlord-1 | landlord | jev | ~typesafe/jev-latest | 0 |
| speculator-1 | speculator | jev | ~typesafe/jev-latest | 0 |
| wage-maximiser-1 | wage-maximiser | jev | ~typesafe/jev-latest | 0 |
| saver-1 | saver | jev | ~typesafe/jev-latest | 0 |
| slacker-1 | slacker | jev | ~typesafe/jev-latest | 0 |
| borrower-1 | borrower | jev | ~typesafe/jev-latest | 0 |
| founder-2 | founder | jev | ~typesafe/jev-latest | 0 |
| lender-2 | lender | jev | ~typesafe/jev-latest | 0 |
| speculator-2 | speculator | jev | ~typesafe/jev-latest | 0 |
| wage-maximiser-2 | wage-maximiser | jev | ~typesafe/jev-latest | 4 |
| saver-2 | saver | jev | ~typesafe/jev-latest | 0 |
| slacker-2 | slacker | jev | ~typesafe/jev-latest | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.11 of $1 (2,778,175 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 2686450 | 0 | 0 | 91725 | 0.11 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 56, 16 people, 17 unemployed. Firms 22; credit outstanding 0.00; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.0211538461538463,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 22,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 24,
 "investment_share": 0.14334470989761092,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 293,
 "materials_to_machines": 42,
 "mean_cycle_wage": 42.041351351351345,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.71666666666667,
 "need_fulfillment_rate": 0.975,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 2027,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 15
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 57.51 | 1.13 | 1.31 | 2 | 22 | 203.28 | 1465 | 483 | 16 | 3% | 0.090 | 100% | 0 | 95.4 |
| 2 | 62.34 | 1.14 | 1.31 | 2 | 22 | 152.46 | 792 | 383 | 38 | 10% | 0.055 | 98% | 0 | 89.4 |
| 3 | 60.98 | 1.14 | 1.31 | 4 | 22 | 101.64 | 1046 | 470 | 38 | 8% | 0.049 | 98% | 0 | 92.5 |
| 4 | 53.73 | 1.14 | 1.31 | 13 | 22 | 50.82 | 1206 | 554 | 40 | 7% | 0.056 | 98% | 0 | 91.1 |
| 5 | 47.11 | 1.14 | 1.31 | 14 | 22 | 0.00 | 1200 | 754 | 42 | 6% | 0.023 | 98% | 0 | 89.9 |
| 6 | 52.44 | 1.14 | 1.31 | 14 | 22 | 0.00 | 1489 | 899 | 44 | 5% | 0.049 | 98% | 0 | 93.1 |
| 7 | 42.04 | 1.14 | 1.31 | 15 | 22 | 0.00 | 2027 | 293 | 42 | 14% | 0.021 | 98% | 0 | 89.7 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

### The days' headlines

**Day 1** (35)

- Epoch 0 opens. Everything is for sale again.
- founder-1 arrives with 1000.00 credits and nothing else.
- lender-1 arrives with 1000.00 credits and nothing else.
- builder-1 arrives with 1000.00 credits and nothing else.
- landlord-1 arrives with 1000.00 credits and nothing else.
- speculator-1 arrives with 1000.00 credits and nothing else.
- wage-maximiser-1 arrives with 1000.00 credits and nothing else.
- saver-1 arrives with 1000.00 credits and nothing else.
- slacker-1 arrives with 1000.00 credits and nothing else.
- borrower-1 arrives with 1000.00 credits and nothing else.
- founder-2 arrives with 1000.00 credits and nothing else.
- lender-2 arrives with 1000.00 credits and nothing else.
- speculator-2 arrives with 1000.00 credits and nothing else.
- wage-maximiser-2 arrives with 1000.00 credits and nothing else.
- saver-2 arrives with 1000.00 credits and nothing else.
- slacker-2 arrives with 1000.00 credits and nothing else.
- rule-prober-1 arrives with 1000.00 credits and nothing else.
- builder-1 founds builder-1's Roofs, a new firm.
- borrower-1 founds borrower-1's Works, a new firm.
- founder-1 founds founder-1's Works, a new firm.
- founder-2 founds founder-2's Works, a new firm.
- H-39 has left Freeport.
- H-38 has left Freeport.
- H-37 has left Freeport.
- H-36 has left Freeport.
- H-35 has left Freeport.
- H-34 has left Freeport.
- H-33 has left Freeport.
- H-32 has left Freeport.
- H-31 has left Freeport.
- H-30 has left Freeport.
- H-29 has left Freeport.
- H-28 has left Freeport.
- H-27 has left Freeport.
- H-26 has left Freeport.

**Day 5** (1)

- Two days remain in the epoch. Settle what you can.

**Day 7** (1)

- The epoch ends after cycle 6. The ledgers close.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **saver-1** | 1197.93 | 197.93 | 0 |
| 2 | **wage-maximiser-1** | 1190.07 | 190.07 | 0 |
| 3 | **saver-2** | 1110.07 | 110.07 | 0 |
| 4 | H-17 | 1100.88 | 100.88 | 0 |
| 5 | H-18 | 1100.88 | 100.88 | 0 |
| 6 | H-22 | 1099.57 | 99.57 | 0 |
| 7 | **speculator-1** | 1098.21 | 98.21 | 0 |
| 8 | H-11 | 1095.45 | 95.45 | 0 |
| 9 | H-12 | 1095.45 | 95.45 | 0 |
| 10 | H-10 | 1094.14 | 94.14 | 0 |
| … | | | | |
| 15 | **rule-prober-1** | 1083.40 | 83.40 | 0 |
| 16 | **wage-maximiser-2** | 1081.89 | 81.89 | 0 |
| … | | | | |
| 20 | **speculator-2** | 1070.10 | 70.10 | 0 |
| … | | | | |
| 32 | **builder-1** | 944.93 | -55.07 | 1 |
| 33 | **slacker-1** | 934.73 | -65.27 | 0 |
| 34 | **slacker-2** | 924.59 | -75.41 | 0 |
| 35 | **founder-1** | 764.85 | -235.15 | 1 |
| 36 | **borrower-1** | 753.91 | -246.09 | 1 |
| 37 | **founder-2** | 731.49 | -268.51 | 1 |
| 38 | **landlord-1** | 628.47 | -371.53 | 0 |
| 39 | **lender-1** | 583.39 | -416.61 | 0 |
| 40 | **lender-2** | 505.50 | -494.50 | 0 |

40 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T00:49:00.227Z.

## Rejections

### unknown_offer (34)

| count | text | tools | players |
|---|---|---|---|
| 20 | That offer was taken or withdrawn | accept_offer | borrower-1, builder-1, founder-1, lender-2, saver-1, saver-2, slacker-1, speculator-1, speculator-2, wage-maximiser-1, wage-maximiser-2 |
| 14 | There is no such offer | accept_offer | founder-1, lender-1, rule-prober-1, saver-2, wage-maximiser-1, wage-maximiser-2 |

### insufficient_funds (23)

| count | text | tools | players |
|---|---|---|---|
| 23 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

### insufficient_goods (16)

| count | text | tools | players |
|---|---|---|---|
| 8 | You have 0 Machines | place_order | rule-prober-1 |
| 8 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

### not_controlling_owner (16)

| count | text | tools | players |
|---|---|---|---|
| 16 | You do not control Legacy Farm No. 1 | declare_dividend, issue_shares | rule-prober-1 |

### not_party (12)

| count | text | tools | players |
|---|---|---|---|
| 8 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 4 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

### unknown_workplace (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | There is no such workplace | set_labor | rule-prober-1 |

### HTTP_400 (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | unknown instrument gold | place_order | rule-prober-1 |

### self_deal (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | Cannot transfer to yourself | transfer | rule-prober-1 |

### unknown_dwelling (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | There is no such dwelling | move_in | rule-prober-1 |

### unknown_order (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | There is no such order | cancel_order | rule-prober-1 |

### not_manager (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | You do not manage Legacy Farm No. 1 | post_employment_offer | rule-prober-1 |

### HTTP_403 (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | you are not in this channel | post_message | rule-prober-1 |

### unknown_org (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | There is no such organization | set_plan | rule-prober-1 |

### not_owner (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | You do not own dwelling no. 1 | post_lease_offer | rule-prober-1 |

### over_contract_hours (5)

| count | text | tools | players |
|---|---|---|---|
| 5 | The contract allows at most 8 h at the farm at Legacy Farm No. 3 | set_labor | rule-prober-1 |

### already_exists (5)

| count | text | tools | players |
|---|---|---|---|
| 5 | You already work at the farm at Legacy Farm No. 3 | accept_offer | rule-prober-1 |

### unknown_contract (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | That contract is not an open employment | terminate_contract | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| borrower-1 | 166 | 20 (12%) | 0.78 | 26 of 295 | 3 | 2 | 0 | 421 | 0.00007 |
| builder-1 | 166 | 143 (86%) | 0.84 | 7 of 206 | 2 | 2 | 0 | 697 | 0.00006 |
| founder-1 | 166 | 90 (54%) | 0.77 | 6 of 242 | 3 | 3 | 0 | 681 | 0.00006 |
| founder-2 | 166 | 89 (54%) | 0.77 | 3 of 264 | 2 | 0 | 0 | 441 | 0.00006 |
| landlord-1 | 166 | 138 (83%) | 0.90 | 2 of 199 | 23 | 0 | 0 | 695 | 0.00005 |
| lender-1 | 166 | 162 (98%) | 0.53 | 3 of 176 | 0 | 1 | 0 | 695 | 0.00005 |
| lender-2 | 166 | 161 (97%) | 0.52 | 0 of 173 | 0 | 3 | 0 | 408 | 0.00005 |
| saver-1 | 166 | 163 (98%) | 0.92 | 3 of 158 | 4 | 2 | 0 | 362 | 0.00004 |
| saver-2 | 166 | 158 (95%) | 0.90 | 4 of 154 | 4 | 4 | 0 | 382 | 0.00004 |
| slacker-1 | 166 | 159 (96%) | 0.66 | 1 of 17 | 0 | 2 | 0 | 92 | 0.00000 |
| slacker-2 | 166 | 162 (98%) | 0.63 | 0 of 13 | 1 | 0 | 0 | 85 | 0.00000 |
| speculator-1 | 166 | 160 (96%) | 0.92 | 5 of 154 | 1 | 1 | 0 | 373 | 0.00004 |
| speculator-2 | 166 | 154 (93%) | 0.93 | 0 of 173 | 5 | 1 | 0 | 425 | 0.00005 |
| wage-maximiser-1 | 166 | 163 (98%) | 0.98 | 0 of 176 | 0 | 3 | 0 | 418 | 0.00005 |
| wage-maximiser-2 | 162 | 151 (93%) | 0.98 | 0 of 162 | 5 | 2 | 0 | 628 | 0.00005 |

| slot | options chosen |
|---|---|
| credit | take_credit 2 |
| dwelling | sell_dwelling 15 |
| housing | rent_cheapest 36 |
| job | wait 332, take_best 36, stay 2, switch 2 |
| lend | lend_cheap 3 |
| market | buy_materials 4, buy_grain 2, buy_ore 2, hold 1 |
| plan | keep_plan 67, put_aside 18 |
| property | buy_dwelling 13, offer_lease 12 |
| venture | sell_output 277, buy_materials 24, fund_firm 24, keep_saving 20, post_job 7, found_now 5, run_quietly 2, post_job_generous 1 |
| work | keep_hours 1613, full_normal 24, work_own 8, work_job 5, few_low 5 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 166 turns, making 1000 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 232.36 credits, 2 job(s), housed, food 100. Accepted 4 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**builder-1** (builder, jev on ~typesafe/jev-latest) took 166 turns, making 1075 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 638.19 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 916 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 579.97 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-2** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 904 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 535.61 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 166 turns, making 535 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 586.85 credits, 1 job(s), housed, food 100. Accepted 15 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-1** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 515 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 562.57 credits, 0 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-2** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 517 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 484.68 credits, 0 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 166 turns, making 204 tool calls of which 160 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1054.58 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 166.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 347 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1113.11 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.01.

**saver-2** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 355 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1085.90 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 166. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 351 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 908.95 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.00.

**slacker-2** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 346 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 898.60 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.00.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 515 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 825.48 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**speculator-2** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 524 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 699.48 credits, 5 job(s), housed, food 100. Accepted 6 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 349 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1105.25 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-2** (wage-maximiser, jev on ~typesafe/jev-latest) took 162 turns and skipped 4, making 349 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1057.72 credits, 5 job(s), housed, food 100. Accepted 6 offer(s). Turns ended by: script 162. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).
