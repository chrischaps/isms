# Playtest run sj2-7day

Synthetic cohort (S1.16, ADR-0009/0010). 8 players, 1324 turns, stopped by epoch_end, 4 turns skipped over 166 hours and 6 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | jev | ~typesafe/jev-latest | 0 |
| wage-maximiser-1 | wage-maximiser | jev | ~typesafe/jev-latest | 0 |
| speculator-1 | speculator | jev | ~typesafe/jev-latest | 4 |
| saver-1 | saver | jev | ~typesafe/jev-latest | 0 |
| slacker-1 | slacker | jev | ~typesafe/jev-latest | 0 |
| borrower-1 | borrower | jev | ~typesafe/jev-latest | 0 |
| landlord-1 | landlord | jev | ~typesafe/jev-latest | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.05 of $1 (1,270,838 of 60,000,000 tokens).

| model | input | cache read | cache write | output | usd |
|---|---|---|---|---|---|
| ~typesafe/jev-latest | 1230471 | 0 | 0 | 40367 | 0.05 |

## The economy at the end

Epoch 1, day 8, hour 1. Population 48, 8 people, 3 unemployed. Firms 20; credit outstanding 0.00; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 8,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.019275067750677666,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 0,
 "firm_count": 20,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 32,
 "investment_share": 0.07815275310834814,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 563,
 "materials_to_machines": 44,
 "mean_cycle_wage": 61.683947368421045,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.71666666666667,
 "need_fulfillment_rate": 0.975,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 2113,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 3
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 58.98 | 1.13 | 1.31 | 0 | 20 | 0.00 | 1425 | 492 | 26 | 5% | 0.042 | 100% | 0 | 95.4 |
| 2 | 61.75 | 1.14 | 1.31 | 2 | 20 | 0.00 | 1335 | 710 | 36 | 5% | 0.038 | 98% | 0 | 89.8 |
| 3 | 63.00 | 1.14 | 1.31 | 0 | 20 | 0.00 | 1646 | 838 | 40 | 5% | 0.044 | 98% | 0 | 93.8 |
| 4 | 61.70 | 1.14 | 1.31 | 2 | 20 | 0.00 | 1839 | 868 | 40 | 5% | 0.046 | 98% | 0 | 91.6 |
| 5 | 54.35 | 1.14 | 1.31 | 9 | 20 | 0.00 | 1812 | 320 | 42 | 13% | 0.008 | 98% | 0 | 91.5 |
| 6 | 58.15 | 1.14 | 1.31 | 6 | 20 | 0.00 | 2025 | 304 | 42 | 14% | 0.034 | 98% | 0 | 93.1 |
| 7 | 61.68 | 1.14 | 1.31 | 3 | 20 | 0.00 | 2113 | 563 | 44 | 8% | 0.019 | 98% | 0 | 89.7 |

Day wage and credit in credits; food is the day's last Food price; output and materials in units; invest is the share of Materials that became Machines or dwellings; Gini is of consumption; needs met is the fulfilment rate. A vibrant economy moves on this table: wages and prices that differ from day to day, credit above zero, an investment share that is not the legacy runner's constant, a Gini that opens as strategies diverge.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **rule-prober-1** | 1225.16 | 225.16 | 0 |
| 2 | **landlord-1** | 1170.42 | 170.42 | 0 |
| 3 | **saver-1** | 1167.85 | 167.85 | 0 |
| 4 | **speculator-1** | 1131.76 | 131.76 | 0 |
| 5 | **wage-maximiser-1** | 1130.40 | 130.40 | 0 |
| 6 | H-18 | 1078.61 | 78.61 | 0 |
| 7 | H-19 | 1078.61 | 78.61 | 0 |
| 8 | H-20 | 1078.61 | 78.61 | 0 |
| 9 | H-21 | 1078.61 | 78.61 | 0 |
| 10 | H-22 | 1078.61 | 78.61 | 0 |
| … | | | | |
| 36 | **slacker-1** | 953.06 | -46.94 | 0 |
| … | | | | |
| 38 | **borrower-1** | 906.54 | -93.46 | 1 |
| … | | | | |
| 40 | **founder-1** | 872.92 | -127.08 | 1 |

40 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 7 and was archived with 1 closing statement (one by founder-1); the window was 61 s. Epoch 2 started on its own at 2026-09-24T22:42:41.057Z.

## Rejections

### insufficient_funds (23)

| count | text | tools | players |
|---|---|---|---|
| 23 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

### unknown_offer (20)

| count | text | tools | players |
|---|---|---|---|
| 11 | That offer was taken or withdrawn | accept_offer | borrower-1, landlord-1, saver-1, slacker-1, speculator-1, wage-maximiser-1 |
| 9 | There is no such offer | accept_offer | rule-prober-1, wage-maximiser-1 |

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

### already_exists (8)

| count | text | tools | players |
|---|---|---|---|
| 8 | You already work at the farm at Legacy Farm No. 2 | accept_offer | rule-prober-1 |

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

### over_contract_hours (7)

| count | text | tools | players |
|---|---|---|---|
| 7 | The contract allows at most 8 h at the farm at Legacy Farm No. 2 | set_labor | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Jev decisions

| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |
|---|---|---|---|---|---|---|---|---|---|
| borrower-1 | 166 | 115 (69%) | 0.77 | 2 of 227 | 1 | 2 | 0 | 216 | 0.00005 |
| founder-1 | 166 | 115 (69%) | 0.82 | 7 of 230 | 1 | 0 | 0 | 324 | 0.00006 |
| landlord-1 | 166 | 163 (98%) | 0.90 | 2 of 174 | 0 | 1 | 0 | 210 | 0.00005 |
| saver-1 | 166 | 163 (98%) | 0.93 | 2 of 177 | 2 | 2 | 0 | 310 | 0.00005 |
| slacker-1 | 166 | 163 (98%) | 0.63 | 1 of 12 | 0 | 1 | 0 | 34 | 0.00000 |
| speculator-1 | 162 | 153 (95%) | 0.88 | 2 of 182 | 3 | 2 | 1 | 564 | 0.00005 |
| wage-maximiser-1 | 166 | 157 (95%) | 0.98 | 0 of 178 | 3 | 4 | 0 | 322 | 0.00005 |

| slot | options chosen |
|---|---|
| housing | rent_cheapest 16 |
| job | take_best 13, switch 5, stay 2 |
| market | hold 9, buy_grain 1, buy_ore 1 |
| plan | keep_plan 44, put_aside 5 |
| venture | sell_output 90, keep_saving 6, buy_materials 5, found_now 3, fund_firm 3, post_job 2, post_job_generous 1 |
| work | keep_hours 962, full_normal 11, few_low 1 |

Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.

## Each player's arc

**borrower-1** (borrower, jev on ~typesafe/jev-latest) took 166 turns, making 1059 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 479.72 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**founder-1** (founder, jev on ~typesafe/jev-latest) took 166 turns, making 880 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 661.10 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 166. Cost $0.01.

**landlord-1** (landlord, jev on ~typesafe/jev-latest) took 166 turns, making 512 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1085.60 credits, 1 job(s), housed, food 100. Accepted 1 offer(s). Turns ended by: script 166. Cost $0.01.

**rule-prober-1** (rule-prober, scripted) took 166 turns, making 209 tool calls of which 165 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1132.34 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 166.

**saver-1** (saver, jev on ~typesafe/jev-latest) took 166 turns, making 348 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1083.03 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.01.

**slacker-1** (slacker, jev on ~typesafe/jev-latest) took 166 turns, making 345 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 900.24 credits, 1 job(s), housed, food 100. Accepted 2 offer(s). Turns ended by: script 166. Cost $0.00.

**speculator-1** (speculator, jev on ~typesafe/jev-latest) took 162 turns and skipped 4, making 507 tool calls of which 2 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 806.60 credits, 3 job(s), housed, food 100. Accepted 3 offer(s). Turns ended by: script 161, error 1. Cost $0.01.

**wage-maximiser-1** (wage-maximiser, jev on ~typesafe/jev-latest) took 166 turns, making 356 tool calls of which 4 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 1045.58 credits, 3 job(s), housed, food 100. Accepted 4 offer(s). Turns ended by: script 166. Cost $0.01.

## Harness notes

1 turn(s) ended without end_turn: error 1.
- speculator-1, day 6 hour 6: JevError: 520 from the decisions API: {"error":{"message":"HTTP 520: error code: 520\n","code":520}}
