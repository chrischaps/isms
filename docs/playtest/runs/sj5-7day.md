# Playtest run sj5-7day

Synthetic cohort (S1.16, ADR-0009/0010). 16 players, 2656 turns, stopped by epoch_end, 0 turns skipped over 166 hours and 6 day ends.

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

Spend: $0.11 of $1 (2,835,915 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 2731093 | 0 | 0 | 104822 | 0.11 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 56, 16 people, 12 unemployed. Firms 22; credit outstanding 0.00; food last 1.14; price index 1.11.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.028190789473684363,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 22,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 24,
 "investment_share": 0.15671641791044777,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 268,
 "materials_to_machines": 42,
 "mean_cycle_wage": 47.2757894736842,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.71666666666667,
 "need_fulfillment_rate": 0.975,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.105579399141631,
 "rations_issued": 0,
 "real_output": 1960,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 11
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 57.26 | 1.13 | 1.31 | 2 | 22 | 203.28 | 1465 | 483 | 16 | 3% | 0.090 | 100% | 0 | 95.4 |
| 2 | 60.10 | 1.14 | 1.31 | 0 | 22 | 152.46 | 792 | 383 | 38 | 10% | 0.040 | 98% | 0 | 90.0 |
| 3 | 59.93 | 1.14 | 1.31 | 6 | 22 | 101.64 | 1064 | 478 | 38 | 8% | 0.041 | 98% | 0 | 91.7 |
| 4 | 50.95 | 1.13 | 1.31 | 11 | 22 | 50.82 | 1266 | 587 | 40 | 7% | 0.049 | 98% | 0 | 91.1 |
| 5 | 47.11 | 1.12 | 1.31 | 16 | 22 | 0.00 | 1378 | 881 | 42 | 5% | 0.044 | 98% | 0 | 90.6 |
| 6 | 62.64 | 1.11 | 1.31 | 3 | 22 | 0.00 | 1820 | 1560 | 44 | 3% | 0.045 | 98% | 0 | 93.1 |
| 7 | 47.28 | 1.11 | 1.14 | 11 | 22 | 0.00 | 1960 | 268 | 42 | 16% | 0.028 | 98% | 0 | 89.7 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

### The days' headlines

**Day 1** (36)

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
- H-25 has left Freeport.

**Day 5** (1)

- Two days remain in the epoch. Settle what you can.

**Day 7** (1)

- The epoch ends after cycle 6. The ledgers close.

## Prices, day by day

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 | 0.61 | 0.92 | 2.28 | 4.00 | 10.52 |
| 2 | 1.31 | 0.61 | 0.92 | 2.28 | **4.12** | 10.52 |
| 3 | 1.31 | **0.59** | **0.88** | **1.98** | 4.12 | 10.52 |
| 4 | 1.31 | **0.56** | **0.84** | **1.89** | **3.94** | 10.52 |
| 5 | 1.31 | **0.53** | **0.80** | **1.81** | **3.76** | 10.52 |
| 6 | 1.31 | 0.53 | 0.80 | **1.72** | **3.58** | 10.52 |
| 7 | **1.14** | 0.53 | 0.80 | 1.72 | 3.58 | 10.52 |

The last price of each good at the day's end, in bold when it differs from the day before. A line is a market that did not move.

### The books at each day's end

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 (599) / no ask | no bid / 0.61 (32) | 1.06 (33) / no ask | 2.28 (261) / no ask | no bid / 4.12 (9) | 10.52 (11) / no ask |
| 2 | 1.31 (879) / no ask | no bid / 0.61 (1007) | no bid / **0.92 (387)** | 2.28 (151) / no ask | no bid / 4.12 (33) | 10.52 (15) / no ask |
| 3 | 1.31 (735) / no ask | no bid / **0.59 (1898)** | no bid / **0.88 (1085)** | no bid / **1.98 (39)** | no bid / 4.12 (131) | 10.52 (10) / no ask |
| 4 | 1.31 (885) / no ask | no bid / **0.56 (2174)** | no bid / **0.84 (2047)** | no bid / **1.89 (124)** | no bid / **3.94 (47)** | 10.52 (9) / no ask |
| 5 | 1.31 (843) / no ask | no bid / **0.53 (81)** | no bid / **0.80 (2008)** | no bid / **1.81 (269)** | no bid / **3.76 (123)** | 10.52 (9) / no ask |
| 6 | 1.31 (454) / no ask | no bid / 0.53 (3404) | no bid / 0.80 (580) | no bid / **1.72 (1080)** | no bid / **3.58 (74)** | 10.52 (8) / no ask |
| 7 | no bid / **1.14 (78)** | no bid / 0.53 (487) | no bid / 0.80 (1428) | no bid / 1.72 (499) | no bid / 3.58 (323) | 10.52 (8) / no ask |

Best bid / best ask, each with the units resting at it; the ask in bold when it differs from the day before. The ask, not the print, is the seller's signal: a cut ask fills at the resting bid's price, so the tape shows the old price the hour the ask drops.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **saver-2** | 1200.45 | 200.45 | 0 |
| 2 | H-3 | 1156.10 | 156.10 | 0 |
| 3 | **saver-1** | 1154.20 | 154.20 | 0 |
| 4 | **wage-maximiser-2** | 1121.43 | 121.43 | 0 |
| 5 | H-18 | 1103.74 | 103.74 | 0 |
| 6 | H-17 | 1102.43 | 102.43 | 0 |
| 7 | H-7 | 1100.16 | 100.16 | 0 |
| 8 | H-22 | 1099.81 | 99.81 | 0 |
| 9 | H-14 | 1098.85 | 98.85 | 0 |
| 10 | H-11 | 1097.54 | 97.54 | 0 |
| … | | | | |
| 26 | **rule-prober-1** | 1004.99 | 4.99 | 0 |
| … | | | | |
| 29 | **wage-maximiser-1** | 975.99 | -24.01 | 0 |
| 30 | **speculator-1** | 964.10 | -35.90 | 0 |
| 31 | **speculator-2** | 951.94 | -48.06 | 0 |
| 32 | **builder-1** | 889.75 | -110.25 | 1 |
| 33 | **slacker-2** | 874.97 | -125.03 | 0 |
| 34 | **slacker-1** | 853.02 | -146.98 | 0 |
| 35 | **founder-2** | 739.05 | -260.95 | 1 |
| 36 | **founder-1** | 617.21 | -382.79 | 1 |
| 37 | **lender-1** | 600.44 | -399.56 | 0 |
| 38 | **lender-2** | 560.96 | -439.04 | 0 |
| 39 | **borrower-1** | 538.24 | -461.76 | 1 |
| 40 | **landlord-1** | 209.11 | -790.89 | 0 |

40 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T14:50:15.460Z.

## Rejections

### unknown_offer (42)

| count | text | tools | players |
|---|---|---|---|
| 27 | That offer was taken or withdrawn | accept_offer | borrower-1, builder-1, founder-1, lender-2, saver-1, saver-2, slacker-1, slacker-2, speculator-1, speculator-2, wage-maximiser-1, wage-maximiser-2 |
| 15 | There is no such offer | accept_offer | builder-1, founder-1, lender-1, rule-prober-1, saver-1, wage-maximiser-1, wage-maximiser-2 |

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

### not_party (11)

| count | text | tools | players |
|---|---|---|---|
| 8 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 3 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

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

### unknown_contract (5)

| count | text | tools | players |
|---|---|---|---|
| 5 | That contract is not an open employment | terminate_contract | rule-prober-1 |

### over_contract_hours (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | The contract allows at most 8 h at the farm at Legacy Farm No. 3 | set_labor | rule-prober-1 |

### already_exists (4)

| count | text | tools | players |
|---|---|---|---|
| 4 | You already work at the farm at Legacy Farm No. 3 | accept_offer | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| borrower-1 | 166 | 61 (37%) | 0.79 | 3 of 282 | 3 | 4 | 0 | 236 | 0.00007 |
| builder-1 | 166 | 134 (81%) | 0.84 | 2 of 220 | 1 | 4 | 0 | 286 | 0.00006 |
| founder-1 | 166 | 148 (89%) | 0.69 | 2 of 330 | 1 | 2 | 0 | 297 | 0.00008 |
| founder-2 | 166 | 150 (90%) | 0.71 | 2 of 346 | 2 | 0 | 0 | 224 | 0.00008 |
| landlord-1 | 166 | 124 (75%) | 0.82 | 7 of 201 | 34 | 0 | 0 | 271 | 0.00005 |
| lender-1 | 166 | 158 (95%) | 0.42 | 19 of 128 | 0 | 1 | 0 | 199 | 0.00004 |
| lender-2 | 166 | 156 (94%) | 0.45 | 13 of 79 | 3 | 1 | 0 | 103 | 0.00002 |
| saver-1 | 166 | 159 (96%) | 0.91 | 5 of 165 | 2 | 5 | 0 | 179 | 0.00004 |
| saver-2 | 166 | 163 (98%) | 0.91 | 5 of 155 | 2 | 1 | 0 | 174 | 0.00004 |
| slacker-1 | 166 | 154 (93%) | 0.64 | 0 of 21 | 0 | 3 | 0 | 41 | 0.00001 |
| slacker-2 | 166 | 157 (95%) | 0.66 | 0 of 18 | 0 | 2 | 0 | 41 | 0.00000 |
| speculator-1 | 166 | 154 (93%) | 0.85 | 13 of 169 | 2 | 1 | 0 | 208 | 0.00005 |
| speculator-2 | 166 | 151 (91%) | 0.88 | 3 of 184 | 2 | 2 | 0 | 217 | 0.00005 |
| wage-maximiser-1 | 166 | 154 (93%) | 0.96 | 1 of 183 | 2 | 5 | 0 | 218 | 0.00005 |
| wage-maximiser-2 | 166 | 159 (96%) | 0.96 | 2 of 171 | 0 | 3 | 0 | 186 | 0.00005 |

| slot | options chosen |
|---|---|
| comfort | take_ask 76, go_without 14 |
| credit | take_credit 2 |
| dwelling | sell_dwelling 18 |
| housing | rent_cheapest 37 |
| job | wait 150, take_best 48 |
| lend | lend_cheap 3 |
| market | buy_materials 10, buy_ore 4, buy_grain 2, hold 1 |
| plan | keep_plan 71, put_aside 27 |
| property | buy_dwelling 18, offer_lease 17 |
| venture | run_quietly 295, take_ask_ore 94, keep_saving 23, take_ask_materials 19, buy_materials 9, fund_firm 6, post_job 6, found_now 4, lay_off 3 |
| which_workplace | open_foundry 3 |
| work | keep_hours 1621, full_normal 39, none 23, few_low 9 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 166 turns, making 1133 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 320.90 credits, 3 job(s), housed, food 100. Accepted 5 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**builder-1** (builder, jev on ~typesafe/jev-latest) took 166 turns, making 1104 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 536.62 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 863 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 394.13 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-2** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 844 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 546.39 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 166 turns, making 558 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 213.82 credits, 2 job(s), housed, food 100. Accepted 21 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-1** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 525 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 583.36 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-2** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 526 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 511.88 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.00.

**rule-prober-1** (rule-prober, scripted) took 166 turns, making 202 tool calls of which 158 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 979.91 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 166.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 359 tool calls of which 5 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1073.12 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**saver-2** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 351 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1119.37 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 362 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 803.94 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.00.

**slacker-2** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 359 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 825.89 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.00.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 527 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 719.09 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**speculator-2** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 532 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 658.39 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 367 tool calls of which 5 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 956.33 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-2** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 359 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1040.35 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).
