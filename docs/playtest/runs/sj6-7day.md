# Playtest run sj6-7day

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

Spend: $0.15 of $1 (3,768,777 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 3609576 | 0 | 0 | 159201 | 0.15 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 40, 16 people, 12 unemployed. Firms 22; credit outstanding 0.00; food last 1.31; price index 1.01.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.21306471306471297,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 22,
 "floor_paid": 0,
 "hardship_count": 13,
 "householders": 8,
 "investment_share": 0,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 286,
 "materials_to_machines": 0,
 "mean_cycle_wage": 32.90863636363637,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 34.81458333333333,
 "need_fulfillment_rate": 0,
 "plan_fulfillment": null,
 "population": 24,
 "price_index": 1.0055793991416309,
 "rations_issued": 0,
 "real_output": 376,
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
| 1 | 54.54 | 1.00 | 1.31 | 2 | 22 | 203.28 | 766 | 240 | 0 | 0% | 0.031 | 100% | 0 | 95.4 |
| 2 | 44.96 | 1.01 | 1.31 | 15 | 22 | 152.46 | 0 | 92 | 0 | 0% | 0.040 | 96% | 0 | 81.6 |
| 3 | 55.01 | 1.01 | 1.31 | 1 | 22 | 101.64 | 64 | 285 | 0 | 0% | 0.054 | 0% | 0 | 49.9 |
| 4 | 60.44 | 1.01 | 1.31 | 6 | 22 | 50.82 | 56 | 280 | 0 | 0% | 0.159 | 0% | 21 | 38.1 |
| 5 | 44.22 | 1.01 | 1.31 | 6 | 22 | 0.00 | 79 | 336 | 0 | 0% | 0.167 | 0% | 21 | 33.4 |
| 6 | 35.63 | 1.01 | 1.31 | 12 | 22 | 0.00 | 122 | 460 | 0 | 0% | 0.212 | 0% | 19 | 33.5 |
| 7 | 32.91 | 1.01 | 1.31 | 11 | 22 | 0.00 | 376 | 286 | 0 | 0% | 0.213 | 0% | 13 | 34.8 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

### The days' headlines

**Day 1** (37)

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
- founder-2 founds founder-2's Works, a new firm.
- H-23 has left Freeport.
- H-22 has left Freeport.
- H-21 has left Freeport.
- H-20 has left Freeport.
- H-19 has left Freeport.
- H-18 has left Freeport.
- H-17 has left Freeport.
- H-16 has left Freeport.
- H-15 has left Freeport.
- H-14 has left Freeport.
- H-13 has left Freeport.
- H-12 has left Freeport.
- H-11 has left Freeport.
- H-10 has left Freeport.
- H-9 has left Freeport.
- H-8 has left Freeport.

**Day 4** (15)

- founder-1 can no longer keep food on the table.
- lender-1 can no longer keep food on the table.
- builder-1 can no longer keep food on the table.
- landlord-1 can no longer keep food on the table.
- speculator-1 can no longer keep food on the table.
- wage-maximiser-1 can no longer keep food on the table.
- saver-1 can no longer keep food on the table.
- borrower-1 can no longer keep food on the table.
- founder-2 can no longer keep food on the table.
- lender-2 can no longer keep food on the table.
- speculator-2 can no longer keep food on the table.
- wage-maximiser-2 can no longer keep food on the table.
- slacker-2 can no longer keep food on the table.
- rule-prober-1 can no longer keep food on the table.
- 21 went hungry this cycle.

**Day 5** (4)

- speculator-2 is eating again.
- saver-2 can no longer keep food on the table.
- 21 went hungry this cycle.
- Two days remain in the epoch. Settle what you can.

**Day 6** (15)

- founder-1 is destitute. The longer contracts are closed to them now.
- lender-1 is destitute. The longer contracts are closed to them now.
- builder-1 is eating again.
- landlord-1 is destitute. The longer contracts are closed to them now.
- speculator-1 is destitute. The longer contracts are closed to them now.
- wage-maximiser-1 is eating again.
- saver-1 is destitute. The longer contracts are closed to them now.
- slacker-1 can no longer keep food on the table.
- borrower-1 is destitute. The longer contracts are closed to them now.
- founder-2 is destitute. The longer contracts are closed to them now.
- lender-2 is destitute. The longer contracts are closed to them now.
- wage-maximiser-2 is destitute. The longer contracts are closed to them now.
- slacker-2 is eating again.
- rule-prober-1 is destitute. The longer contracts are closed to them now.
- 19 went hungry this cycle.

**Day 7** (9)

- speculator-1 is eating again.
- speculator-1 is back on their feet.
- slacker-1 is eating again.
- lender-2 is eating again.
- lender-2 is back on their feet.
- speculator-2 can no longer keep food on the table.
- saver-2 is eating again.
- 13 went hungry this cycle.
- The epoch ends after cycle 6. The ledgers close.

## Prices, day by day

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 | 0.61 | 0.92 | 3.04 | 4.00 | 9.00 |
| 2 | 1.31 | 0.61 | 0.92 | 3.04 | **4.12** | 9.00 |
| 3 | 1.31 | **0.59** | **0.88** | 3.04 | 4.12 | 9.00 |
| 4 | 1.31 | 0.59 | 0.88 | **2.28** | 4.12 | 9.00 |
| 5 | 1.31 | **0.56** | 0.88 | 2.28 | 4.12 | 9.00 |
| 6 | 1.31 | 0.56 | **0.84** | 2.28 | 4.12 | 9.00 |
| 7 | 1.31 | **0.53** | **0.80** | **1.70** | 4.12 | 9.00 |

The last price of each good at the day's end, in bold when it differs from the day before. A line is a market that did not move.

### The books at each day's end

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 (337) / no ask | no bid / 0.61 (685) | 1.06 (495) / no ask | 3.04 (640) / no ask | no bid / 4.12 (1) | 10.52 (18) / no ask |
| 2 | 1.31 (257) / no ask | no bid / **no ask** | 1.06 (4) / no ask | 3.04 (358) / no ask | 4.12 (70) / **no ask** | no bid / no ask |
| 3 | 1.31 (379) / no ask | no bid / **0.59 (3039)** | no bid / **0.88 (469)** | 2.28 (464) / no ask | 4.12 (8) / no ask | 10.52 (6) / no ask |
| 4 | 1.31 (241) / no ask | no bid / **no ask** | no bid / 0.88 (171) | no bid / no ask | 4.12 (134) / no ask | no bid / no ask |
| 5 | 1.31 (402) / no ask | no bid / **0.56 (2906)** | no bid / 0.88 (274) | no bid / **1.81 (11)** | 4.12 (35) / no ask | 10.52 (4) / no ask |
| 6 | 1.31 (260) / no ask | no bid / **no ask** | no bid / **0.84 (6)** | no bid / **1.72 (83)** | 4.12 (151) / no ask | 10.52 (1) / no ask |
| 7 | 1.31 (506) / no ask | no bid / **0.53 (1995)** | no bid / **0.80 (346)** | no bid / **1.70 (197)** | 4.12 (50) / no ask | 10.52 (5) / no ask |

Best bid / best ask, each with the units resting at it; the ask in bold when it differs from the day before. The ask, not the print, is the seller's signal: a cut ask fills at the resting bid's price, so the tape shows the old price the hour the ask drops.

## SJ.5's week beside it: `sj5b-7day` (forty in the town), prices day by day

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

### `sj5b-7day`: the books at each day's end

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
| 1 | H-4 | 1244.03 | 244.03 | 0 |
| 2 | H-5 | 1237.02 | 237.02 | 0 |
| 3 | H-7 | 1227.44 | 227.44 | 0 |
| 4 | H-0 | 1215.44 | 215.44 | 0 |
| 5 | **saver-2** | 1206.93 | 206.93 | 0 |
| 6 | H-2 | 1202.15 | 202.15 | 0 |
| 7 | **wage-maximiser-2** | 1198.51 | 198.51 | 0 |
| 8 | **saver-1** | 1194.03 | 194.03 | 0 |
| 9 | H-1 | 1189.74 | 189.74 | 0 |
| 10 | **wage-maximiser-1** | 1170.72 | 170.72 | 0 |
| 11 | **landlord-1** | 1168.90 | 168.90 | 0 |
| 12 | **slacker-1** | 1121.43 | 121.43 | 0 |
| 13 | **slacker-2** | 1114.20 | 114.20 | 0 |
| … | | | | |
| 16 | **speculator-2** | 1019.01 | 19.01 | 0 |
| 17 | **rule-prober-1** | 1013.50 | 13.50 | 0 |
| 18 | **speculator-1** | 917.77 | -82.23 | 0 |
| 19 | **lender-1** | 895.32 | -104.68 | 0 |
| 20 | **founder-1** | 832.07 | -167.93 | 1 |
| 21 | **builder-1** | 825.27 | -174.73 | 1 |
| 22 | **lender-2** | 772.23 | -227.77 | 0 |
| 23 | **founder-2** | 689.73 | -310.27 | 1 |
| 24 | **borrower-1** | 447.72 | -552.28 | 1 |

24 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T16:18:15.389Z.

## Rejections

### unknown_offer (39)

| count | text | tools | players |
|---|---|---|---|
| 29 | That offer was taken or withdrawn | accept_offer | borrower-1, builder-1, founder-1, founder-2, landlord-1, saver-1, saver-2, slacker-1, slacker-2, speculator-1, speculator-2, wage-maximiser-1, wage-maximiser-2 |
| 10 | There is no such offer | accept_offer | landlord-1, rule-prober-1, saver-1 |

### insufficient_funds (23)

| count | text | tools | players |
|---|---|---|---|
| 23 | You have 964.63 | place_order, transfer, post_credit_offer | rule-prober-1 |

### not_controlling_owner (16)

| count | text | tools | players |
|---|---|---|---|
| 16 | You do not control Legacy Farm No. 1 | declare_dividend, issue_shares | rule-prober-1 |

### insufficient_goods (15)

| count | text | tools | players |
|---|---|---|---|
| 8 | You have 0 Machines | place_order | rule-prober-1 |
| 7 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

### not_party (10)

| count | text | tools | players |
|---|---|---|---|
| 8 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 2 | Only the worker or the employer terminates | terminate_contract | rule-prober-1 |

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

### unknown_contract (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | That contract is not an open employment | terminate_contract | rule-prober-1 |

### over_contract_hours (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | The contract allows at most 8 h at the farm at Legacy Farm No. 2 | set_labor | rule-prober-1 |

### options_narrowed (1)

| count | text | tools | players |
|---|---|---|---|
| 1 | A destitute citizen cannot found an org | found_org | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| borrower-1 | 166 | 33 (20%) | 0.75 | 41 of 414 | 18 | 4 | 0 | 222 | 0.00009 |
| builder-1 | 166 | 148 (89%) | 0.51 | 5 of 267 | 3 | 3 | 0 | 317 | 0.00007 |
| founder-1 | 166 | 147 (89%) | 0.58 | 3 of 411 | 2 | 3 | 0 | 288 | 0.00009 |
| founder-2 | 166 | 144 (87%) | 0.50 | 3 of 383 | 2 | 1 | 0 | 223 | 0.00009 |
| landlord-1 | 166 | 158 (95%) | 0.77 | 3 of 243 | 2 | 2 | 0 | 254 | 0.00005 |
| lender-1 | 166 | 156 (94%) | 0.52 | 49 of 263 | 14 | 0 | 0 | 261 | 0.00006 |
| lender-2 | 166 | 157 (95%) | 0.66 | 37 of 240 | 1 | 0 | 0 | 187 | 0.00006 |
| saver-1 | 166 | 156 (94%) | 0.54 | 5 of 244 | 6 | 5 | 0 | 216 | 0.00006 |
| saver-2 | 166 | 161 (97%) | 0.50 | 7 of 219 | 1 | 3 | 0 | 203 | 0.00005 |
| slacker-1 | 166 | 155 (93%) | 0.64 | 6 of 170 | 5 | 3 | 0 | 172 | 0.00004 |
| slacker-2 | 166 | 158 (95%) | 0.61 | 10 of 196 | 2 | 1 | 0 | 163 | 0.00004 |
| speculator-1 | 166 | 143 (86%) | 0.61 | 7 of 239 | 3 | 1 | 0 | 217 | 0.00005 |
| speculator-2 | 166 | 143 (86%) | 0.52 | 24 of 213 | 2 | 2 | 0 | 210 | 0.00005 |
| wage-maximiser-1 | 166 | 159 (96%) | 0.79 | 3 of 221 | 2 | 2 | 0 | 210 | 0.00005 |
| wage-maximiser-2 | 166 | 159 (96%) | 0.79 | 2 of 242 | 3 | 1 | 0 | 196 | 0.00006 |

| slot | options chosen |
|---|---|
| comfort | take_ask 61, go_without 29 |
| credit | take_credit 2 |
| housing | rent_cheapest 33 |
| job | take_best 59, wait 49 |
| lend | lend_cheap 3 |
| market | buy_materials 21, buy_ore 2, sell_ore 2 |
| plan | keep_plan 1757, stock_more_food 31, put_aside 30, loosen_floor 2 |
| venture | run_quietly 345, take_ask_ore 114, fund_firm 31, buy_materials 26, post_job 15, found_now 4, undercut 4, take_ask_materials 4 |
| which_workplace | open_foundry 3 |
| work | keep_hours 1220, full_normal 80, work_more 25, few_low 9, work_job 3, work_own 1 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 166 turns, making 1268 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 154.46 credits, 3 job(s), housed, food 0. Accepted 4 offer(s). Founded 1 org(s). Was below 20 food in 112 turn(s). Turns ended by: script 166. Cost $0.01.

**builder-1** (builder, jev on ~typesafe/jev-latest) took 166 turns, making 1148 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 607.06 credits, 3 job(s), housed, food 20. Accepted 3 offer(s). Founded 1 org(s). Was below 20 food in 92 turn(s). Turns ended by: script 166. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 1131 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 644.85 credits, 3 job(s), housed, food 0. Accepted 4 offer(s). Founded 1 org(s). Was below 20 food in 114 turn(s). Turns ended by: script 166. Cost $0.02.

**founder-2** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 1136 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 448.43 credits, 3 job(s), housed, food 0. Accepted 4 offer(s). Founded 1 org(s). Was below 20 food in 112 turn(s). Turns ended by: script 166. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 166 turns, making 640 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1166.78 credits, 3 job(s), housed, food 0. Accepted 4 offer(s). Was below 20 food in 112 turn(s). Turns ended by: script 166. Cost $0.01.

**lender-1** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 641 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 856.96 credits, 3 job(s), housed, food 4. Accepted 4 offer(s). Was below 20 food in 109 turn(s). Turns ended by: script 166. Cost $0.01.

**lender-2** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 634 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 778.92 credits, 3 job(s), housed, food 44. Accepted 4 offer(s). Was below 20 food in 83 turn(s). Turns ended by: script 166. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 166 turns, making 196 tool calls of which 152 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1013.50 credits, 1 job(s), unhoused, food 0. Accepted 1 offer(s). Was below 20 food in 113 turn(s). Turns ended by: script 166.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 477 tool calls of which 5 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1160.03 credits, 3 job(s), housed, food 0. Accepted 4 offer(s). Was below 20 food in 113 turn(s). Turns ended by: script 166. Cost $0.01.

**saver-2** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 472 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1190.40 credits, 2 job(s), housed, food 20. Accepted 3 offer(s). Was below 20 food in 106 turn(s). Turns ended by: script 166. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 473 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1085.46 credits, 3 job(s), housed, food 32. Accepted 4 offer(s). Was below 20 food in 93 turn(s). Turns ended by: script 166. Cost $0.01.

**slacker-2** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 469 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1091.68 credits, 2 job(s), housed, food 18. Accepted 3 offer(s). Was below 20 food in 97 turn(s). Turns ended by: script 166. Cost $0.01.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 653 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 512.13 credits, 3 job(s), housed, food 0. Accepted 3 offer(s). Was below 20 food in 111 turn(s). Turns ended by: script 166. Cost $0.01.

**speculator-2** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 656 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 639.34 credits, 3 job(s), housed, food 16. Accepted 4 offer(s). Was below 20 food in 104 turn(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 472 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1131.64 credits, 3 job(s), housed, food 10. Accepted 4 offer(s). Was below 20 food in 102 turn(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-2** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 472 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1160.51 credits, 3 job(s), housed, food 0. Accepted 4 offer(s). Was below 20 food in 112 turn(s). Turns ended by: script 166. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).
