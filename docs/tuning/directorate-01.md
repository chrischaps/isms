# The Directorate — tuning report 01 (first stable run, S0.16 exit)

**Sessions:** S0.16a, S0.16b, S0.16c · **Date:** 2026-09-13 · **Command:** `make sim-check PRESET=directorate` (= `cargo run -p isms-sim --release -- run --preset directorate --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/directorate.toml` as committed with this report. Values that differ from the preset as it stood before S0.16:

| Tunable | Before | Now | Kind |
|---|---|---|---|
| `policy.price_list_credits.food` | 1.20 | 2.50 | the Committee's own policy (GDD 6.3); moved by this pass |
| `policy.price_list_credits.wares` | 3.00 | 5.00 | the Committee's own policy; moved by this pass |
| `policy.minimum_food_ration` | 24 (a top-up to 24) | 8 (a fixed issue per cycle) | the Committee's own policy; semantics revised (Q75) |
| `policy.wage_grades_credits` | 6.00 … 9.00 by skill band of 20 | unchanged | the Committee's own policy |
| `labor.skill_band` | none | 20 | new tunable (S0.16b, Q73) |
| `governance.ratchet_mult` | none | 1.0 | new tunable (S0.16b, Q70) |
| `governance.sim_planner_target_growth` | 1.05 | unchanged | sim only (T10) |

## Result: stable

```
directorate seed 1: 5 epochs, 403036 events, 0 rejected commands, 0.5s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42  100.0          -          -  0.028   0.067         0      0         0         0
    1      42  100.0          -          -  0.028   0.067         0      0         0         0
    2      42  100.0          -          -  0.028   0.067         0      0         0         0
    3      42  100.0          -          -  0.028   0.067         0      0         0         0
    4      42  100.0          -          -  0.028   0.067         0      0         0         0
seeds 2-5: identical to seed 1, event for event (403036 events each).
```

Seed 1, epoch 0, from the CSV (the state's books; `plan_fulfillment` starts at cycle 1 when the planner has a cycle to plan from):

| cycle | till (credits) | mean wage | rations issued | median wellbeing | plan fulfilment |
|---|---|---|---|---|---|
| 0 | 9592 | 48.0 | 266 | 95.7 | – |
| 1 | 11249 | 59.7 | 320 | 89.5 | 0.997 |
| 10 | 11052 | 60.8 | 320 | 98.3 | 0.969 |
| 20 | 10752 | 61.1 | 320 | 98.8 | 0.944 |
| 30 | 8474 | 66.0 | 320 | 98.7 | 0.950 |
| 41 | 5824 | 66.0 | 320 | 98.8 | 0.940 |

| Target (GDD §17, as derived for this preset) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 100.0 % every epoch | yes |
| Price index within ±30 % of the basket | not applicable: administered prices, no index | n/a |
| No persistent Food stock-out (≤ 3 consecutive cycles with unserved state-store requests) | 0: every request was served in full in every cycle | yes |
| Materials sinks balanced (investment share 0.05–0.6) | 0.067 every epoch | yes, near the floor (as in the Commune, by the Machine Shop's one worker) |
| Rejected commands (householders and the System planner) = 0 | 0 | yes |

## What changed and why

1. **The ration is a fixed issue, not a top-up (Q75).** As first built (Q68) every pantry was topped up to 24 Food at zero price; whoever bought Food held 24 anyway, so nobody bought any and the state's only takings were Wares money. Now every citizen receives 8 Food a cycle free (a third of a day) and buys the rest at list. Need-fulfillment was 100 % under both rules; the change is about who pays for what, which is the Directorate's own question.
2. **The price list must cover the wage fund.** The state is the only employer and the only seller, so its takings can never exceed what citizens spend, and the till converges to the point where wages paid equal consumer spending. At 1.20 / 3.00 a citizen spent about 26 a cycle against a grade-0 wage of 48: the seeded budget (18 × 384 = 7200) was gone by cycle 20 and the state paid every position pro rata for the rest of the epoch, wages settling at 26.5 (need still 100 %, nobody in hardship: the ration and the endowment carried them). Sweeps (`--param policy.price_list_credits.food=…`, made possible by Q77):

   | price list (Food / Wares) | till at cycle 10 / 20 / 41 | mean wage at cycle 41 |
   |---|---|---|
   | 1.20 / 3.00 (before) | 0 / 0 / 0 | 26.5 |
   | 1.60 / 3.60 | 1098 / 0 / 0 | 34.4 |
   | 2.00 / 4.00 | 5127 / 45 / 0 | 44.2 |
   | 2.50 / 5.00 (chosen) | 11052 / 10752 / 5824 | 66.0 (full grade 3) |
   | 3.00 / 6.00 | 16977 / 21459 / 26612 | 66.0 (the state hoards) |
   | grades −25 % at 1.20 / 3.00 | 3353 / 0 / 0 | 26.5 |

   Lowering the grades does not help: takings are bounded by spending, which the grades do not move. 2.50 / 5.00 is the lowest list of the sweep at which every position is paid its full grade for the whole epoch; the till still declines late in the epoch as skill lifts everyone to grade 3 (66 a cycle against about 60 of spending), which a Committee that indexed prices to grades would correct (Phase 4).
3. **Why the seeds do not differ.** As in the Commune: no request ever went unserved, so no tie-break consumed the RNG; the monitoring noise (σ = 0.25) changes attributed output inside `Produced` but not the event count or any decision; and the planner's targets follow output deterministically.
4. **Why plan fulfilment sits at 0.94–1.00.** The sim planner asks for last cycle's output × 1.05 every cycle; output grows with skill but not by 5 % a cycle, so workplaces fall just short and the bonus fires only in the cycles where skill jumps a band. That is the planner's own greed (T10), not the engine's.

## Neutrality (GDD Appendix B item 6)

The Directorate's headline metric (100 %) equals the Commune's and beats Freeport's by the hardship Freeport's own wage-before-bread logic produces in its first cycle. The two price changes are the Committee's own instrument, moved for the Committee's own reason (the wage fund and the consumer-goods fund must balance), and they move no other preset's metrics: the Directorate's list is not the reference basket. A Soviet planner (item 1) would recognise the plan, the ratchet, the wage grades, the state store queue and the ration, and would point out that the till's late-epoch decline is the classic sign of a price list lagging the wage scale. The shortage the system is famous for does not appear because the plan is fulfilled at 94 % or better: the sim planner is modest and the Materials split does not starve the Workshops. The ration at 8 and the list at 2.50 / 5.00 are ours; the mechanisms are the system's.

## Open items carried forward

- Prices indexed to the wage scale, or a planner that sets prices from the wage fund, is a Phase 4 Committee behaviour; until then the till declines slowly late in each epoch.
- `plan_fulfillment` is measured against the sim planner's 5 % growth ask; the Committee's real targets will change what "fulfilment" means.
- The investment band 0.05–0.6 remains the simulator's own reading; the Directorate sits at 0.067 by its labor allocation (`balance_weights`), as the Commune does.
- Q75 supersedes Q68: `minimum_food_ration` is now a per-cycle issue everywhere it is read (the Commonwealth's basic provision in S0.17c will use the same rule from its bank pool).
