# Playtest run sj3-2day

Synthetic cohort (S1.16, ADR-0009/0010). 16 players, 736 turns, stopped by epoch_end, 0 turns skipped over 46 hours and 1 day ends.

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
| wage-maximiser-2 | wage-maximiser | jev | ~typesafe/jev-latest | 0 |
| saver-2 | saver | jev | ~typesafe/jev-latest | 0 |
| slacker-2 | slacker | jev | ~typesafe/jev-latest | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.03 of $1 (751,010 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 724562 | 0 | 0 | 26448 | 0.03 |

## The economy at the end

Epoch 1, day 3, hour 1. Population 56, 16 people, 5 unemployed. Firms 22; credit outstanding 152.46; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.05553511705685632,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 15246,
 "firm_count": 22,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 24,
 "investment_share": 0.09921671018276762,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 383,
 "materials_to_machines": 38,
 "mean_cycle_wage": 62.23756756756757,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.42500000000001,
 "need_fulfillment_rate": 0.975,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 792,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 5
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 56.77 | 1.13 | 1.31 | 3 | 22 | 203.28 | 1461 | 483 | 16 | 3% | 0.090 | 100% | 0 | 95.4 |
| 2 | 62.24 | 1.14 | 1.31 | 5 | 22 | 152.46 | 792 | 383 | 38 | 10% | 0.056 | 98% | 0 | 89.4 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

### The days' headlines

**Day 1** (34)

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

**Day 2** (1)

- The epoch ends after cycle 1. The ledgers close.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **rule-prober-1** | 1041.40 | 41.40 | 0 |
| 2 | **saver-2** | 1030.59 | 30.59 | 0 |
| 3 | **saver-1** | 1027.97 | 27.97 | 0 |
| 4 | **wage-maximiser-2** | 1026.66 | 26.66 | 0 |
| 5 | **speculator-1** | 1020.92 | 20.92 | 0 |
| 6 | **speculator-2** | 1020.16 | 20.16 | 0 |
| 7 | **wage-maximiser-1** | 1009.13 | 9.13 | 0 |
| 8 | H-22 | 1007.89 | 7.89 | 0 |
| 9 | H-8 | 994.03 | -5.97 | 0 |
| 10 | H-2 | 992.72 | -7.28 | 0 |
| … | | | | |
| 32 | **slacker-2** | 962.75 | -37.25 | 0 |
| 33 | **slacker-1** | 962.73 | -37.27 | 0 |
| 34 | **landlord-1** | 904.09 | -95.91 | 0 |
| 35 | **founder-1** | 782.47 | -217.53 | 1 |
| 36 | **builder-1** | 774.92 | -225.08 | 1 |
| 37 | **borrower-1** | 771.23 | -228.77 | 1 |
| 38 | **founder-2** | 767.34 | -232.66 | 1 |
| 39 | **lender-2** | 658.16 | -341.84 | 0 |
| 40 | **lender-1** | 582.28 | -417.72 | 0 |

40 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 2 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T00:19:19.606Z.

## Rejections

### unknown_offer (31)

| count | text | tools | players |
|---|---|---|---|
| 22 | That offer was taken or withdrawn | accept_offer | borrower-1, builder-1, lender-1, saver-1, saver-2, slacker-1, slacker-2, speculator-1, speculator-2, wage-maximiser-1, wage-maximiser-2 |
| 9 | There is no such offer | accept_offer | landlord-1, lender-1, lender-2, rule-prober-1, saver-1, slacker-2 |

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
| 2 | The contract allows at most 8 h at the farm at Legacy Farm No. 3 | set_labor | rule-prober-1 |

### already_exists (1)

| count | text | tools | players |
|---|---|---|---|
| 1 | You already work at the farm at Legacy Farm No. 3 | accept_offer | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| borrower-1 | 46 | 1 (2%) | 0.79 | 2 of 100 | 0 | 4 | 0 | 414 | 0.00008 |
| builder-1 | 46 | 41 (89%) | 0.61 | 2 of 90 | 0 | 2 | 0 | 669 | 0.00008 |
| founder-1 | 46 | 40 (87%) | 0.70 | 5 of 44 | 1 | 0 | 0 | 511 | 0.00004 |
| founder-2 | 46 | 40 (87%) | 0.76 | 2 of 67 | 0 | 0 | 0 | 412 | 0.00006 |
| landlord-1 | 46 | 44 (96%) | 0.49 | 1 of 50 | 0 | 1 | 0 | 681 | 0.00005 |
| lender-1 | 46 | 41 (89%) | 0.55 | 0 of 54 | 0 | 3 | 0 | 665 | 0.00006 |
| lender-2 | 46 | 44 (96%) | 0.56 | 1 of 51 | 0 | 1 | 0 | 392 | 0.00005 |
| saver-1 | 46 | 42 (91%) | 0.85 | 1 of 36 | 0 | 4 | 0 | 289 | 0.00003 |
| saver-2 | 46 | 43 (93%) | 0.85 | 2 of 26 | 0 | 1 | 0 | 209 | 0.00003 |
| slacker-1 | 46 | 42 (91%) | 0.69 | 1 of 10 | 0 | 4 | 0 | 84 | 0.00001 |
| slacker-2 | 46 | 43 (93%) | 0.75 | 0 of 8 | 0 | 3 | 0 | 63 | 0.00001 |
| speculator-1 | 46 | 42 (91%) | 0.77 | 3 of 38 | 1 | 1 | 0 | 262 | 0.00003 |
| speculator-2 | 46 | 42 (91%) | 0.83 | 7 of 58 | 0 | 2 | 0 | 412 | 0.00005 |
| wage-maximiser-1 | 46 | 41 (89%) | 0.98 | 0 of 50 | 1 | 1 | 0 | 419 | 0.00005 |
| wage-maximiser-2 | 46 | 43 (93%) | 0.97 | 0 of 39 | 0 | 2 | 0 | 294 | 0.00004 |

| slot | options chosen |
|---|---|
| credit | take_credit 2 |
| housing | rent_cheapest 37 |
| job | wait 138, take_best 19, switch 4, stay 3 |
| lend | lend_cheap 3 |
| market | buy_ore 9, hold 3, buy_grain 2 |
| plan | keep_plan 21, put_aside 8 |
| venture | run_quietly 39, sell_output 38, keep_saving 17, buy_materials 11, found_now 6, fund_firm 4, post_job 4, post_job_generous 1 |
| work | keep_hours 333, full_normal 12, work_own 3, work_job 2, few_low 2 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 46 turns, making 298 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 589.40 credits, 2 job(s), housed, food 96. Accepted 4 offer(s). Founded 1 org(s). Turns ended by: script 46. Cost $0.00.

**builder-1** (builder, jev on ~typesafe/jev-latest) took 46 turns, making 289 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 507.92 credits, 1 job(s), housed, food 98. Accepted 2 offer(s). Founded 1 org(s). Turns ended by: script 46. Cost $0.00.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 46 turns, making 231 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 544.85 credits, 1 job(s), housed, food 96. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 46. Cost $0.00.

**founder-2** (founder, jev on ~typesafe/jev-latest) took 46 turns, making 226 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 535.03 credits, 1 job(s), housed, food 96. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 46. Cost $0.00.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 46 turns, making 145 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 910.78 credits, 0 job(s), housed, food 96. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**lender-1** (lender, jev on ~typesafe/jev-latest) took 46 turns, making 153 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 538.15 credits, 0 job(s), housed, food 96. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**lender-2** (lender, jev on ~typesafe/jev-latest) took 46 turns, making 147 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 666.16 credits, 0 job(s), housed, food 94. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**rule-prober-1** (rule-prober, scripted) took 46 turns, making 58 tool calls of which 44 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 977.40 credits, 1 job(s), unhoused, food 98. Accepted 1 offer(s). Turns ended by: script 46.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 46 turns, making 106 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 970.66 credits, 1 job(s), housed, food 96. Accepted 2 offer(s). Turns ended by: script 46. Cost $0.00.

**saver-2** (saver, jev on ~typesafe/jev-latest) took 46 turns, making 100 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 973.28 credits, 1 job(s), housed, food 94. Accepted 2 offer(s). Turns ended by: script 46. Cost $0.00.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 46 turns, making 105 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 938.73 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

**slacker-2** (slacker, jev on ~typesafe/jev-latest) took 46 turns, making 104 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 938.75 credits, 1 job(s), housed, food 99. Accepted 2 offer(s). Turns ended by: script 46. Cost $0.00.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 46 turns, making 152 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 721.32 credits, 2 job(s), housed, food 98. Accepted 3 offer(s). Turns ended by: script 46. Cost $0.00.

**speculator-2** (speculator, jev on ~typesafe/jev-latest) took 46 turns, making 150 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 723.12 credits, 1 job(s), housed, food 96. Accepted 2 offer(s). Turns ended by: script 46. Cost $0.00.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 46 turns, making 103 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 951.09 credits, 2 job(s), housed, food 96. Accepted 3 offer(s). Turns ended by: script 46. Cost $0.00.

**wage-maximiser-2** (wage-maximiser, jev on ~typesafe/jev-latest) took 46 turns, making 102 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 969.35 credits, 1 job(s), housed, food 96. Accepted 1 offer(s). Turns ended by: script 46. Cost $0.00.

## Harness notes

Every turn ended with end_turn (or a script).
