# Playtest run sj5-town-scripted

Synthetic cohort (S1.16, ADR-0009/0010). 16 players, 688 turns, stopped by epoch_end, 0 turns skipped over 43 hours and 1 day ends.

## Run

| player | persona | brain | model | skipped |
|---|---|---|---|---|
| founder-1 | founder | scripted | - | 0 |
| lender-1 | lender | scripted | - | 0 |
| builder-1 | builder | scripted | - | 0 |
| landlord-1 | landlord | scripted | - | 0 |
| speculator-1 | speculator | scripted | - | 0 |
| wage-maximiser-1 | wage-maximiser | scripted | - | 0 |
| saver-1 | saver | scripted | - | 0 |
| slacker-1 | slacker | scripted | - | 0 |
| borrower-1 | borrower | scripted | - | 0 |
| founder-2 | founder | scripted | - | 0 |
| lender-2 | lender | scripted | - | 0 |
| speculator-2 | speculator | scripted | - | 0 |
| wage-maximiser-2 | wage-maximiser | scripted | - | 0 |
| saver-2 | saver | scripted | - | 0 |
| slacker-2 | slacker | scripted | - | 0 |
| rule-prober-1 | rule-prober | scripted | - | 0 |

Spend: $0.00 of $40 (0 of 60,000,000 tokens).

## The economy at the end

Epoch 1, day 3, hour 1. Population 56, 16 people, 0 unemployed. Firms 22; credit outstanding 126.00; food last 1.31; price index 1.14.

Last day's aggregates:

```json
{
 "active_humans": 16,
 "bank_loans_outstanding": 0,
 "consumption_gini": 0.14925172678434384,
 "contribution_gini": 0,
 "coop_surplus_per_member": 0,
 "credit_outstanding": 12600,
 "firm_count": 22,
 "floor_paid": 0,
 "hardship_count": 0,
 "householders": 24,
 "investment_share": 0.1111111111111111,
 "levy_pool": 0,
 "low_population_cycles": 0,
 "materials_produced": 342,
 "materials_to_machines": 38,
 "mean_cycle_wage": 62.4,
 "mean_tenure_cycles": 0,
 "median_wellbeing": 89.39791666666667,
 "need_fulfillment_rate": 0.6,
 "plan_fulfillment": null,
 "population": 40,
 "price_index": 1.1360515021459228,
 "rations_issued": 0,
 "real_output": 710,
 "state_stock": {},
 "store_stock": {},
 "store_unfilled": 0,
 "tax_collected": 0,
 "till": 0,
 "treasury": 0,
 "unemployed": 0
}
```

## The economy, day by day

| day | day wage | price index | food | unemployed | firms | credit | output | materials | to machines | invest | Gini | needs met | hardship | wellbeing |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 54.66 | 1.00 | 1.31 | 0 | 22 | 168.00 | 1422 | 483 | 4 | 1% | 0.219 | 100% | 0 | 95.4 |
| 2 | 62.40 | 1.14 | 1.31 | 0 | 22 | 126.00 | 710 | 342 | 38 | 11% | 0.149 | 60% | 0 | 89.4 |

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
- founder-1 founds founder-1's Works, a new firm.
- founder-2 founds founder-2's Works, a new firm.
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
- H-29 has left Freeport.
- H-28 has left Freeport.
- H-27 has left Freeport.

**Day 2** (2)

- Prices rose 14% over the cycle. The basket costs 1.14 of what it did on day one.
- The epoch ends after cycle 1. The ledgers close.

## Prices, day by day

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 | 0.61 | 0.92 | 2.28 | 4.00 | 10.52 |
| 2 | 1.31 | 0.61 | 0.92 | 2.28 | **4.12** | 10.52 |

The last price of each good at the day's end, in bold when it differs from the day before. A line is a market that did not move.

### The books at each day's end

| day | food | grain | ore | materials | wares | machines |
|---|---|---|---|---|---|---|
| 1 | 1.31 (551) / no ask | 0.70 (879) / no ask | no bid / 0.92 (14) | 2.28 (557) / no ask | no bid / 4.12 (7) | 10.52 (12) / no ask |
| 2 | 1.31 (872) / no ask | no bid / **0.61 (205)** | no bid / 0.92 (259) | 2.28 (270) / no ask | 4.12 (38) / **no ask** | 10.52 (15) / no ask |

Best bid / best ask, each with the units resting at it; the ask in bold when it differs from the day before. The ask, not the print, is the seller's signal: a cut ask fills at the resting bid's price, so the tape shows the old price the hour the ask drops.

## Standings at the end

| rank | citizen | net worth | self-made | firms |
|---|---|---|---|---|
| 1 | **rule-prober-1** | 1046.50 | 46.50 | 0 |
| 2 | **landlord-1** | 1045.19 | 45.19 | 0 |
| 3 | **saver-1** | 1042.57 | 42.57 | 0 |
| 4 | **speculator-2** | 1041.26 | 41.26 | 0 |
| 5 | **wage-maximiser-2** | 1038.64 | 38.64 | 0 |
| 6 | **speculator-1** | 1037.33 | 37.33 | 0 |
| 7 | **wage-maximiser-1** | 1033.40 | 33.40 | 0 |
| 8 | **saver-2** | 1033.40 | 33.40 | 0 |
| 9 | H-20 | 999.65 | -0.35 | 0 |
| 10 | **slacker-2** | 998.36 | -1.64 | 0 |
| … | | | | |
| 29 | **slacker-1** | 986.57 | -13.43 | 0 |
| … | | | | |
| 35 | **borrower-1** | 860.37 | -139.63 | 1 |
| 36 | **lender-2** | 841.26 | -158.74 | 0 |
| 37 | **founder-1** | 780.17 | -219.83 | 1 |
| 38 | **builder-1** | 768.36 | -231.64 | 1 |
| 39 | **founder-2** | 762.33 | -237.67 | 1 |
| 40 | **lender-1** | 714.78 | -285.22 | 0 |

40 citizens ranked by net worth; the cohort's players in bold. Self-made is net worth less the endowment.

## The epoch's end

Epoch 1 ended (scheduled) after day 2 and was archived with 1 closing statement (one by founder-1); the window was 60 s. Epoch 2 started on its own at 2026-09-25T14:17:42.467Z.

## Rejections

### insufficient_funds (6)

| count | text | tools | players |
|---|---|---|---|
| 6 | You have 968.56 | place_order, transfer, post_credit_offer | rule-prober-1 |

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

### unknown_offer (3)

| count | text | tools | players |
|---|---|---|---|
| 2 | There is no such offer | accept_offer | rule-prober-1 |
| 1 | That offer was taken or withdrawn | accept_offer | lender-2 |

### unknown_workplace (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | There is no such workplace | set_labor | rule-prober-1 |

### HTTP_400 (2)

| count | text | tools | players |
|---|---|---|---|
| 2 | unknown instrument gold | place_order | rule-prober-1 |

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
| 2 | The contract allows at most 8 h at the mine at Legacy Mine No. 2 | set_labor | rule-prober-1 |

## Fuzzer findings

None: every probe was refused, without a server error and without raw ids in the refusal.

## What players did not understand

Nothing reported.
## Governance

No governance tool was used: there is no assembly in this society, or nobody reached for it.

## Each player's arc

**borrower-1** (borrower, scripted) took 43 turns, making 175 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 747.17 credits, 1 job(s), unhoused, food 96. Accepted 2 offer(s). Founded 1 org(s). Turns ended by: script 43.

**builder-1** (builder, scripted) took 43 turns, making 134 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 658.76 credits, 1 job(s), unhoused, food 98. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 43.

**founder-1** (founder, scripted) took 43 turns, making 134 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 706.71 credits, 1 job(s), unhoused, food 94. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 43.

**founder-2** (founder, scripted) took 43 turns, making 134 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 605.82 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Founded 1 org(s). Turns ended by: script 43.

**landlord-1** (landlord, scripted) took 43 turns, making 90 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 979.88 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Turns ended by: script 43.

**lender-1** (lender, scripted) took 43 turns, making 49 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 607.47 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Turns ended by: script 43.

**lender-2** (lender, scripted) took 43 turns, making 49 tool calls of which 1 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 777.26 credits, 1 job(s), unhoused, food 94. Accepted 1 offer(s). Turns ended by: script 43.

**rule-prober-1** (rule-prober, scripted) took 43 turns, making 54 tool calls of which 40 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 982.50 credits, 1 job(s), unhoused, food 90. Accepted 1 offer(s). Turns ended by: script 43.

**saver-1** (saver, scripted) took 43 turns, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 977.26 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Turns ended by: script 43.

**saver-2** (saver, scripted) took 43 turns, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 968.09 credits, 1 job(s), unhoused, food 94. Accepted 1 offer(s). Turns ended by: script 43.

**slacker-1** (slacker, scripted) took 43 turns, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 954.57 credits, 1 job(s), unhoused, food 100. Accepted 1 offer(s). Turns ended by: script 43.

**slacker-2** (slacker, scripted) took 43 turns, making 4 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 966.36 credits, 1 job(s), unhoused, food 84. Accepted 1 offer(s). Turns ended by: script 43.

**speculator-1** (speculator, scripted) took 43 turns, making 52 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 313.39 credits, 1 job(s), unhoused, food 98. Accepted 1 offer(s). Turns ended by: script 43.

**speculator-2** (speculator, scripted) took 43 turns, making 55 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 119.55 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Turns ended by: script 43.

**wage-maximiser-1** (wage-maximiser, scripted) took 43 turns, making 46 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 968.09 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Turns ended by: script 43.

**wage-maximiser-2** (wage-maximiser, scripted) took 43 turns, making 46 tool calls of which 0 were refused. Started with 968.56 credits, 0 job(s), unhoused; ended with 972.02 credits, 1 job(s), unhoused, food 96. Accepted 1 offer(s). Turns ended by: script 43.

## Harness notes

Every turn ended with end_turn (or a script).
