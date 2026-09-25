# Playtest run sj6b-7day

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
| speculator-2 | speculator | jev | ~typesafe/jev-latest | 4 |
| wage-maximiser-2 | wage-maximiser | jev | ~typesafe/jev-latest | 0 |
| saver-2 | saver | jev | ~typesafe/jev-latest | 0 |
| slacker-2 | slacker | jev | ~typesafe/jev-latest | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.15 of $40 (3,749,077 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 3612595 | 0 | 0 | 136482 | 0.15 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 24, 16 people, 4 unemployed. Firms 22; credit outstanding 0.00; food last 1.14; price index 0.99.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.013497652582159514,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 22,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 8,
 "investment_share": 0,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 267,
 "materials_to_machines": 0,
 "mean_cycle_wage": 60.69954545454546,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 66.6,
 "need_fulfillment_rate": 0.9583333333333334,
 "plan_fulfillment": null,
 "population": 24,
 "price_index": 0.9931330472103004,
 "rations_issued": 0,
 "real_output": 724,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 4
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 52.26 | 1.00 | 1.31 | 6 | 20 | 203.28 | 637 | 60 | 0 | 0% | 0.034 | 100% | 0 | 95.3 |
| 2 | 58.31 | 1.00 | 1.31 | 1 | 20 | 152.46 | 864 | 0 | 0 | 0% | 0.013 | 96% | 0 | 87.5 |
| 3 | 50.19 | 1.00 | 1.26 | 4 | 20 | 101.64 | 481 | 0 | 0 | 0% | 0.013 | 96% | 0 | 79.5 |
| 4 | 59.64 | 1.00 | 1.26 | 1 | 20 | 50.82 | 647 | 0 | 0 | 0% | 0.013 | 96% | 0 | 71.5 |
| 5 | 57.21 | 1.00 | 1.20 | 4 | 20 | 0.00 | 682 | 0 | 0 | 0% | 0.013 | 96% | 0 | 66.6 |
| 6 | 61.39 | 0.99 | 1.14 | 1 | 22 | 0.00 | 705 | 252 | 0 | 0% | 0.013 | 96% | 0 | 66.6 |
| 7 | 60.70 | 0.99 | 1.14 | 4 | 22 | 0.00 | 724 | 267 | 0 | 0% | 0.013 | 96% | 0 | 66.6 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

### The days' headlines

**Day 1** (19)

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

**Day 5** (1)

- Two days remain in the epoch. Settle what you can.

**Day 6** (2)

- founder-2 founds founder-2's Works, a new firm.
- founder-1 founds founder-1's Works, a new firm.

**Day 7** (1)

- The epoch ends after cycle 6. The ledgers close.

## Prices, day by day

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 | 0.70 | 0.90 | 3.67 | 4.00 | 9.00 |
| 2 | 1.31 | 0.70 | 0.90 | 3.67 | 4.00 | 9.00 |
| 3 | **1.26** | **0.59** | **0.95** | 3.67 | 4.00 | 9.00 |
| 4 | 1.26 | **0.56** | 0.95 | 3.67 | 4.00 | 9.00 |
| 5 | **1.20** | 0.56 | 0.95 | 3.67 | 4.00 | 9.00 |
| 6 | **1.14** | **0.53** | **0.88** | **3.85** | 4.00 | 9.00 |
| 7 | 1.14 | 0.53 | **0.84** | **2.28** | 4.00 | 9.00 |

The last price of each good at the day's end, in bold when it differs from the day before. A line is a market that did not move.

### The books at each day's end

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | no bid / 1.31 (46) | 0.70 (502) / no ask | 1.06 (523) / no ask | 4.04 (758) / no ask | no bid / no ask | 10.52 (8) / no ask |
| 2 | no bid / 1.31 (28) | 0.70 (256) / no ask | 0.95 (240) / no ask | 4.04 (112) / no ask | 4.00 (64) / no ask | no bid / no ask |
| 3 | no bid / **1.26 (170)** | no bid / **0.59 (72)** | 0.95 (80) / no ask | 4.04 (637) / no ask | 4.00 (20) / no ask | 10.52 (8) / no ask |
| 4 | no bid / 1.26 (117) | no bid / **0.56 (370)** | no bid / **0.92 (16)** | 4.04 (112) / no ask | 4.00 (122) / no ask | 10.52 (2) / no ask |
| 5 | no bid / **1.20 (137)** | no bid / 0.56 (808) | no bid / **0.88 (384)** | 4.04 (617) / no ask | 4.00 (20) / no ask | 10.52 (7) / no ask |
| 6 | no bid / **1.14 (23)** | no bid / **0.53 (242)** | no bid / 0.88 (165) | 3.85 (30) / **3.93 (38)** | 4.00 (146) / no ask | 10.52 (4) / no ask |
| 7 | no bid / 1.14 (18) | no bid / 0.53 (2091) | no bid / **0.84 (103)** | 2.28 (443) / **3.74 (76)** | 4.00 (25) / no ask | 10.52 (6) / no ask |

Best bid / best ask, each with the units resting at it; the ask in bold when it differs from the day before. The ask, not the print, is the seller's signal: a cut ask fills at the resting bid's price, so the tape shows the old price the hour the ask drops.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **wage-maximiser-2** | 1198.24 | 198.24 | 0 |
| 2 | H-6 | 1191.74 | 191.74 | 0 |
| 3 | H-3 | 1181.34 | 181.34 | 0 |
| 4 | H-1 | 1179.38 | 179.38 | 0 |
| 5 | H-5 | 1179.26 | 179.26 | 0 |
| 6 | H-7 | 1179.20 | 179.20 | 0 |
| 7 | H-4 | 1179.08 | 179.08 | 0 |
| 8 | **wage-maximiser-1** | 1177.40 | 177.40 | 0 |
| 9 | H-0 | 1176.41 | 176.41 | 0 |
| 10 | **saver-1** | 1175.30 | 175.30 | 0 |
| 11 | **landlord-1** | 1157.54 | 157.54 | 0 |
| … | | | | |
| 13 | **saver-2** | 1142.55 | 142.55 | 0 |
| 14 | **speculator-2** | 1038.07 | 38.07 | 0 |
| 15 | **slacker-1** | 946.14 | -53.86 | 0 |
| 16 | **slacker-2** | 938.92 | -61.08 | 0 |
| 17 | **rule-prober-1** | 919.77 | -80.23 | 0 |
| 18 | **speculator-1** | 851.23 | -148.77 | 0 |
| 19 | **founder-1** | 840.14 | -159.86 | 1 |
| 20 | **borrower-1** | 833.37 | -166.63 | 1 |
| 21 | **lender-1** | 748.60 | -251.40 | 0 |
| 22 | **founder-2** | 736.29 | -263.71 | 1 |
| 23 | **builder-1** | 735.43 | -264.57 | 1 |
| 24 | **lender-2** | 663.37 | -336.63 | 0 |

24 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T21:14:56.027Z.

## Rejections

### unknown_offer (38)

| count | text | tools | players |
|---|---|---|---|
| 29 | That offer was taken or withdrawn | accept_offer | borrower-1, builder-1, founder-1, founder-2, lender-2, saver-1, slacker-1, slacker-2, speculator-1, speculator-2, wage-maximiser-1, wage-maximiser-2 |
| 9 | There is no such offer | accept_offer | landlord-1, rule-prober-1 |

### insufficient_funds (23)

| count | text | tools | players |
|---|---|---|---|
| 23 | You have 964.63 | place_order, transfer, post_credit_offer | rule-prober-1 |

### not_party (18)

| count | text | tools | players |
|---|---|---|---|
| 8 | Not the owner-occupant (tenants end their lease) | move_out | rule-prober-1 |
| 8 | Only the owner or the tenant ends a lease | terminate_contract | rule-prober-1 |
| 2 | Only the owner cancels | cancel_order | rule-prober-1 |

### insufficient_goods (16)

| count | text | tools | players |
|---|---|---|---|
| 8 | You have 0 Machines | place_order | rule-prober-1 |
| 8 | A workplace needs 20 Materials; pantry holds 0 | found_org | rule-prober-1 |

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

### unknown_order (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | There is no such order | cancel_order | rule-prober-1 |

### over_contract_hours (3)

| count | text | tools | players |
|---|---|---|---|
| 3 | The contract allows at most 8 h at the foundry at Legacy Foundry No. 3 | set_labor | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| borrower-1 | 166 | 86 (52%) | 0.63 | 50 of 191 | 10 | 3 | 0 | 163 | 0.00005 |
| builder-1 | 166 | 107 (64%) | 0.45 | 14 of 310 | 8 | 3 | 0 | 274 | 0.00008 |
| founder-1 | 166 | 156 (94%) | 0.51 | 49 of 491 | 0 | 2 | 0 | 284 | 0.00009 |
| founder-2 | 166 | 41 (25%) | 0.65 | 81 of 485 | 4 | 1 | 0 | 220 | 0.00009 |
| landlord-1 | 166 | 161 (97%) | 0.68 | 2 of 324 | 2 | 1 | 0 | 262 | 0.00007 |
| lender-1 | 166 | 162 (98%) | 0.47 | 9 of 169 | 1 | 0 | 0 | 236 | 0.00005 |
| lender-2 | 166 | 159 (96%) | 0.43 | 9 of 165 | 1 | 4 | 0 | 191 | 0.00005 |
| saver-1 | 166 | 164 (99%) | 0.69 | 14 of 324 | 4 | 1 | 0 | 216 | 0.00007 |
| saver-2 | 166 | 162 (98%) | 0.75 | 2 of 322 | 3 | 0 | 0 | 201 | 0.00007 |
| slacker-1 | 166 | 160 (96%) | 0.25 | 6 of 165 | 0 | 4 | 0 | 194 | 0.00004 |
| slacker-2 | 166 | 162 (98%) | 0.35 | 5 of 161 | 1 | 1 | 0 | 176 | 0.00004 |
| speculator-1 | 166 | 139 (84%) | 0.81 | 18 of 214 | 7 | 2 | 0 | 217 | 0.00006 |
| speculator-2 | 162 | 137 (85%) | 0.80 | 25 of 211 | 6 | 3 | 0 | 456 | 0.00006 |
| wage-maximiser-1 | 166 | 150 (90%) | 0.96 | 1 of 187 | 10 | 1 | 0 | 227 | 0.00005 |
| wage-maximiser-2 | 166 | 149 (90%) | 0.96 | 1 of 190 | 6 | 4 | 0 | 214 | 0.00005 |

| slot | options chosen | options offered |
|---|---|---|
| comfort | take_ask 49, go_without 41 | take_ask 90, bid_under 90, go_without 90 |
| credit | take_credit 2 | take_credit 2, no_credit 2 |
| housing | rent_cheapest 37 | rent_cheapest 37, stay_unhoused 37 |
| job | stay 1225, switch 170, wait 75, take_best 30 | switch 1395, stay 1395, take_best 105, take_second 105, wait 105 |
| lend | lend_cheap 3 | lend_cheap 3, lend_dear 3, hold_money 3 |
| market | buy_materials 40, buy_grain 7, hold 5 | hold 52, buy_materials 40, buy_grain 12 |
| plan | keep_plan 75, put_aside 20 | put_aside 95, keep_plan 95 |
| venture | run_quietly 146, buy_materials 133, keep_saving 121, take_ask_materials 49, undercut 43, fund_firm 33, take_ask_ore 24, post_job 17, found_now 4 | run_quietly 312, keep_saving 258, buy_materials 254, post_job 183, post_job_generous 183, take_ask_ore 137, bid_under_ore 137, bid_under_materials 53, fund_firm 51, take_ask_materials 51, undercut 47, hold_price 47, pay_dividend 36, found_now 4 |
| which_workplace | open_foundry 3 | open_foundry 3, open_mill 3, open_mine 3, open_workshop 3, decide_later 3 |
| work | keep_hours 1494, full_normal 46, few_low 8, full_high 4, work_own 3, none 1, work_job 1 | keep_hours 1497, ease_off 1487, full_normal 60, full_high 60, half_normal 60, few_low 60, none 60, work_own 17, work_job 2 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns. Options offered: how often each option was on the slot's list (— in journals from before E-6); `--journal-questions` keeps the whole request in the journal.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 166 turns, making 947 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 166.29 credits, 10 job(s), housed, food 100. Accepted 10 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**builder-1** (builder, jev on ~typesafe/jev-latest) took 166 turns, making 1077 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 155.35 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 777 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 550.26 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.02.

**founder-2** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 1014 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 360.21 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.02.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 166 turns, making 520 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1076.46 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-1** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 520 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 697.92 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**lender-2** (lender, jev on ~typesafe/jev-latest) took 166 turns, making 526 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 611.09 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 166 turns, making 197 tool calls of which 153 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 894.69 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 166.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 352 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1094.22 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**saver-2** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 352 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1055.07 credits, 2 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 359 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 897.06 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.01.

**slacker-2** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 355 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 886.64 credits, 2 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 166. Cost $0.01.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 166 turns, making 551 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 583.91 credits, 8 job(s), housed, food 100. Accepted 9 offer(s). Turns ended by: script 166. Cost $0.01.

**speculator-2** (speculator, jev on ~typesafe/jev-latest) took 162 turns and skipped 4, making 535 tool calls of which 3 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 715.77 credits, 7 job(s), housed, food 100. Accepted 8 offer(s). Turns ended by: script 162. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 371 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1084.06 credits, 8 job(s), housed, food 100. Accepted 8 offer(s). Turns ended by: script 166. Cost $0.01.

**wage-maximiser-2** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 374 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1108.36 credits, 7 job(s), housed, food 100. Accepted 8 offer(s). Turns ended by: script 166. Cost $0.01.

## Harness notes

Every turn ended with end_turn (or a script).

## The labour board, day by day (E-7's first read)

| day | offers posted | at 8.00 | 8.40 | 8.80 | 9.20 | 9.60 | 10.00 | 10.40 |
|---|---|---|---|---|---|---|---|---|
| 1 | 36 | 36 | | | | | | |
| 2 | 17 | 2 | 15 | | | | | |
| 3 | 19 | 4 | 3 | 12 | | | | |
| 4 | 13 | 1 | 3 | 1 | 8 | | | |
| 5 | 14 | 2 | 3 | 1 | 1 | 7 | | |
| 6 | 18 | 3 | 2 | 1 | 2 (+3 at 9.00–9.10, a player's) | 1 | 6 | |
| 7 | 17 | 3 | 4 | | | 4 | 2 | 4 |

`EmploymentOffered` events by the hourly wage they carry. The boards at the last close: Machine Shop at step 7 (10.80), Builders No. 1 at 6, Foundries No. 1 and 2 and Mine No. 3 at 5 (10.00), the mills at 0–1 (8.00–8.40). The wage-maximisers switched jobs eight times each along this ladder (`job switch` chosen 170 times in the week); the mines got hands on day 3 (ore produced 369 that day, none before) and the foundries on day 6 (Materials 252, none since day 1's 60).

## Materials: who traded, and at whose price

| day | units | day-VWAP | sellers | buyers |
|---|---|---|---|---|
| 1 | 60 | 3.06 | Legacy Foundry No. 3, its seeded shelf at 2.28 → 3.67 as it emptied | speculators and founders (57 of 60), Builders No. 1 (3) |
| 2–5 | 0 | — | nobody: the foundries had no hands, `materials_produced` 0 | a 4.04 bid (`buy_materials`, 10 % over the last print) resting all four days, 112–758 units deep |
| 6 | 203 | **4.05** | borrower-1's Works (a foundry since day 1) at 4.04–4.88, then Legacy Foundry No. 1 at 4.04 into the resting bids and at 3.85 | speculator-2 (108 units at 4.04), founder-1 (4.44–4.88 at the ask), builder-1's Roofs (95 at 3.85) |
| 7 | 174 | **2.46** | Legacy Foundry No. 2 at its cost-plus 2.28 (its shelf grew at the close: step −1) | Legacy Builders No. 1 and 2, Workshop No. 1 — the runner's own buyers |

The day-VWAP moved 3.06 → 4.05 → 2.46: the first move is a player's (a bid posted over the last print, filled by a player's firm and a legacy foundry), the second is the runner's supply coming back at the runner's price.

## SJ.6's week beside it: `sj6-7day` (the same town before E-4, E-7, E-2 and E-3), prices day by day

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

### `sj6-7day`: the books at each day's end

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
