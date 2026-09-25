# Playtest run sj5b-7day

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

Spend: $0.11 of $1 (2,825,276 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 2723104 | 0 | 0 | 102172 | 0.11 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 56, 16 people, 14 unemployed. Firms 21; credit outstanding 0.00; food last 1.14; price index 1.11.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.027516556291390826,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 21,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 24,
 "investment_share": 0.051094890510948905,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 822,
 "materials_to_machines": 42,
 "mean_cycle_wage": 54.835135135135154,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.71666666666667,
 "need_fulfillment_rate": 0.975,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.105579399141631,
 "rations_issued": 0,
 "real_output": 2226,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 13
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 57.47 | 1.13 | 1.31 | 3 | 21 | 203.28 | 1453 | 483 | 16 | 3% | 0.090 | 100% | 0 | 95.4 |
| 2 | 58.77 | 1.14 | 1.31 | 0 | 21 | 152.46 | 799 | 467 | 38 | 8% | 0.037 | 98% | 0 | 90.0 |
| 3 | 59.20 | 1.14 | 1.31 | 0 | 21 | 101.64 | 1057 | 567 | 38 | 7% | 0.035 | 98% | 0 | 91.7 |
| 4 | 51.41 | 1.13 | 1.31 | 11 | 21 | 50.82 | 1222 | 648 | 40 | 6% | 0.047 | 98% | 0 | 91.1 |
| 5 | 50.62 | 1.12 | 1.31 | 11 | 21 | 0.00 | 1353 | 833 | 42 | 5% | 0.044 | 98% | 0 | 90.2 |
| 6 | 56.30 | 1.11 | 1.31 | 12 | 21 | 0.00 | 1410 | 929 | 44 | 5% | 0.045 | 98% | 0 | 93.1 |
| 7 | 54.84 | 1.11 | 1.14 | 13 | 21 | 0.00 | 2226 | 822 | 42 | 5% | 0.028 | 98% | 0 | 89.7 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

### The days' headlines

**Day 1** (30)

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
- founder-1 founds founder-1's Works, a new firm.
- borrower-1 founds borrower-1's Works, a new firm.
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

**Day 5** (1)

- Two days remain in the epoch. Settle what you can.

**Day 7** (1)

- The epoch ends after cycle 6. The ledgers close.

## Prices, day by day

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 | 0.61 | 1.06 | 2.28 | 4.00 | 10.52 |
| 2 | 1.31 | 0.61 | **0.92** | 2.28 | **4.12** | 10.52 |
| 3 | 1.31 | **0.59** | **0.88** | **1.98** | 4.12 | 10.52 |
| 4 | 1.31 | **0.56** | **0.84** | **1.89** | **3.94** | 10.52 |
| 5 | 1.31 | **0.53** | **0.80** | **1.81** | **3.76** | 10.52 |
| 6 | 1.31 | 0.53 | 0.80 | 1.81 | **3.58** | 10.52 |
| 7 | **1.14** | 0.53 | 0.80 | **1.72** | 3.58 | 10.52 |

The last price of each good at the day's end, in bold when it differs from the day before. A line is a market that did not move.

### The books at each day's end

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 (599) / no ask | no bid / 0.61 (37) | 1.06 (135) / no ask | 2.28 (256) / no ask | no bid / 4.12 (9) | 10.52 (11) / no ask |
| 2 | 1.31 (879) / no ask | no bid / 0.61 (891) | no bid / **0.92 (308)** | 2.28 (116) / no ask | no bid / 4.12 (25) | 10.52 (17) / no ask |
| 3 | 1.31 (728) / no ask | no bid / **0.59 (1458)** | no bid / **0.88 (591)** | no bid / **1.98 (49)** | no bid / 4.12 (132) | 10.52 (12) / no ask |
| 4 | 1.31 (888) / no ask | no bid / **0.56 (2412)** | no bid / **0.84 (523)** | no bid / **1.89 (170)** | no bid / **3.94 (55)** | 10.52 (10) / no ask |
| 5 | 1.31 (847) / no ask | no bid / **0.53 (351)** | no bid / **0.80 (475)** | no bid / **1.81 (8)** | no bid / **3.76 (64)** | 10.52 (9) / no ask |
| 6 | 1.31 (797) / no ask | no bid / 0.53 (5114) | no bid / 0.80 (1923) | no bid / 1.81 (593) | no bid / **3.58 (149)** | 10.52 (7) / no ask |
| 7 | no bid / **1.14 (44)** | no bid / **no ask** | no bid / **no ask** | no bid / **1.72 (54)** | no bid / 3.58 (291) | 10.52 (7) / no ask |

Best bid / best ask, each with the units resting at it; the ask in bold when it differs from the day before. The ask, not the print, is the seller's signal: a cut ask fills at the resting bid's price, so the tape shows the old price the hour the ask drops.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **saver-1** | 1192.24 | 192.24 | 0 |
| 2 | H-20 | 1104.20 | 104.20 | 0 |
| 3 | H-17 | 1102.89 | 102.89 | 0 |
| 4 | H-18 | 1102.89 | 102.89 | 0 |
| 5 | H-14 | 1099.31 | 99.31 | 0 |
| 6 | H-22 | 1098.15 | 98.15 | 0 |
| 7 | H-11 | 1098.00 | 98.00 | 0 |
| 8 | H-2 | 1096.69 | 96.69 | 0 |
| 9 | H-10 | 1096.69 | 96.69 | 0 |
| 10 | H-13 | 1096.69 | 96.69 | 0 |
| … | | | | |
| 15 | **saver-2** | 1092.58 | 92.58 | 0 |
| … | | | | |
| 18 | **founder-2** | 1090.22 | 90.22 | 0 |
| … | | | | |
| 23 | **speculator-1** | 1079.67 | 79.67 | 0 |
| … | | | | |
| 27 | **borrower-1** | 1038.95 | 38.95 | 1 |
| 28 | **wage-maximiser-1** | 1022.37 | 22.37 | 0 |
| 29 | **wage-maximiser-2** | 1005.68 | 5.68 | 0 |
| 30 | **rule-prober-1** | 1003.83 | 3.83 | 0 |
| 31 | **speculator-2** | 995.52 | -4.48 | 0 |
| … | | | | |
| 34 | **slacker-1** | 880.70 | -119.30 | 0 |
| 35 | **slacker-2** | 874.37 | -125.63 | 0 |
| 36 | **builder-1** | 868.11 | -131.89 | 1 |
| 37 | **founder-1** | 605.68 | -394.32 | 1 |
| 38 | **lender-2** | 604.77 | -395.23 | 0 |
| 39 | **lender-1** | 531.74 | -468.26 | 0 |
| 40 | **landlord-1** | 415.61 | -584.39 | 0 |

40 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T15:25:57.977Z.

## Rejections

### unknown_offer (34)

| count | text | tools | players |
|---|---|---|---|
| 24 | That offer was taken or withdrawn | accept_offer | builder-1, founder-1, founder-2, landlord-1, lender-1, saver-1, slacker-1, slacker-2, speculator-1, speculator-2, wage-maximiser-2 |
| 10 | There is no such offer | accept_offer | lender-2, rule-prober-1, saver-1 |

### insufficient_funds (23)

| count | text | tools | players |
|---|---|---|---|
| 23 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

### insufficient_goods (16)

| count | text | tools | players |
|---|---|---|---|
| 8 | You have 0 Machines | place_order | rule-prober-1 |
| 8 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

### not_party (16)

| count | text | tools | players |
|---|---|---|---|
| 8 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 8 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

### not_controlling_owner (16)

| count | text | tools | players |
|---|---|---|---|
| 16 | You do not control Legacy Farm No. 1 | declare_dividend, issue_shares | rule-prober-1 |

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
| borrower-1 | 166 | 11 (7%) | 0.68 | 6 of 238 | 3 | 0 | 0 | 227 | 0.00007 |
| builder-1 | 166 | 137 (83%) | 0.82 | 6 of 214 | 2 | 2 | 0 | 295 | 0.00006 |
| founder-1 | 166 | 148 (89%) | 0.73 | 3 of 330 | 2 | 2 | 0 | 300 | 0.00008 |
| founder-2 | 166 | 157 (95%) | 0.61 | 0 of 348 | 0 | 2 | 0 | 211 | 0.00007 |
| landlord-1 | 166 | 129 (78%) | 0.76 | 5 of 206 | 25 | 1 | 0 | 266 | 0.00005 |
| lender-1 | 166 | 152 (92%) | 0.47 | 10 of 96 | 0 | 3 | 0 | 157 | 0.00003 |
| lender-2 | 166 | 157 (95%) | 0.47 | 9 of 118 | 1 | 1 | 0 | 146 | 0.00003 |
| saver-1 | 166 | 163 (98%) | 0.92 | 5 of 165 | 2 | 3 | 0 | 185 | 0.00004 |
| saver-2 | 166 | 160 (96%) | 0.91 | 6 of 156 | 3 | 0 | 0 | 171 | 0.00004 |
| slacker-1 | 166 | 156 (94%) | 0.68 | 0 of 19 | 0 | 3 | 0 | 39 | 0.00000 |
| slacker-2 | 166 | 157 (95%) | 0.69 | 0 of 18 | 0 | 2 | 0 | 37 | 0.00000 |
| speculator-1 | 166 | 157 (95%) | 0.81 | 16 of 177 | 0 | 2 | 0 | 196 | 0.00005 |
| speculator-2 | 166 | 154 (93%) | 0.82 | 11 of 196 | 2 | 2 | 0 | 206 | 0.00005 |
| wage-maximiser-1 | 166 | 158 (95%) | 0.96 | 3 of 179 | 2 | 0 | 0 | 213 | 0.00005 |
| wage-maximiser-2 | 166 | 154 (93%) | 0.96 | 1 of 172 | 2 | 3 | 0 | 186 | 0.00005 |

| slot | options chosen |
|---|---|
| comfort | take_ask 75, go_without 13 |
| credit | take_credit 1 |
| dwelling | sell_dwelling 14 |
| housing | rent_cheapest 34 |
| job | wait 172, take_best 44 |
| lend | lend_cheap 3 |
| market | buy_materials 16, hold 8, buy_ore 6, buy_grain 2 |
| plan | keep_plan 72, put_aside 28 |
| property | buy_dwelling 13, offer_lease 12 |
| venture | keep_saving 166, run_quietly 162, take_ask_ore 130, take_ask_materials 15, undercut 14, buy_materials 9, fund_firm 6, post_job 5, found_now 3, lay_off 2 |
| which_workplace | open_foundry 2 |
| work | keep_hours 1521, none 41, full_normal 26, few_low 8, work_job 6, work_own 2, push_harder 1 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 166 turns, making 1177 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 0.09 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**builder-1** (builder, jev on ~typesafe/jev-latest) took 166 turns, making 1083 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 498.85 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 870 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 385.27 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-2** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 692 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1009.14 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 166 turns, making 551 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 374.37 credits, 2 job(s), housed, food 100. Accepted 16 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-1** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 532 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 514.66 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.00.

**lender-2** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 526 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 587.69 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 166 turns, making 202 tool calls of which 158 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 978.75 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 166.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 355 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1113.28 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**saver-2** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 354 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1057.36 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 362 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 831.62 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.00.

**slacker-2** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 359 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 826.35 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.00.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 525 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 790.83 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.01.

**speculator-2** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 528 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 752.01 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 357 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 987.15 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-2** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 365 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 970.46 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 166. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).
