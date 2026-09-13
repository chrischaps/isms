# Freeport — tuning report 01 (Phase 0a exit)

**Sessions:** S0.14b, S0.14 · **Date:** 2026-09-13 · **Command:** `make sim-check PRESET=freeport` (= `cargo run -p isms-sim --release -- run --preset freeport --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/freeport.toml` as committed with this report. Two values differ from report 00:

| Tunable | Report 00 | Now | Kind |
|---|---|---|---|
| `seeding.legacy_inventory` | none | Mill: 320 Food, 120 Grain; Foundry: 60 Ore | new tunable (engine session S0.14b, ADR-0004, Q45) |
| `householder.legacy_machine_buy_payroll_mult` | 1.0 | 1.5 | config (Q43) |

`needs.food_meter_per_unit` stays at the GDD value of 4. Report 00 proposed 6; it is not needed once day one has stock (below). (S1.0 later set it to 6 anyway, for humans: Q20. The householder tables below are unchanged by that.)

## Result: stable

```
freeport seed 1: 5 epochs, 594009 events, 0 rejected commands, 1.0s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   99.5       1.02       1.14  0.009   0.274         1     16         2         0
    1      42   99.4       1.02       1.14  0.009   0.281         2     22         2         0
    2      42   99.5       1.02       1.14  0.009   0.274         1     16         2         0
    3      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    4      42  100.0       1.02       1.14  0.007   0.288         0     16         2         0
freeport seed 2: 5 epochs, 597360 events, 0 rejected commands, 0.9s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   99.6       1.02       1.14  0.009   0.269         1     17         2         0
    1      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    2      42   99.6       1.02       1.14  0.009   0.269         1     17         2         0
    3      42   99.5       1.02       1.14  0.009   0.274         1     16         2         0
    4      42   99.6       1.02       1.14  0.011   0.269         1     15         2         0
freeport seed 3: 5 epochs, 594460 events, 0 rejected commands, 0.9s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   99.4       1.02       1.14  0.009   0.281         2     22         2         0
    1      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    2      42   99.6       1.02       1.14  0.009   0.269         1     17         2         0
    3      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    4      42   99.5       1.02       1.14  0.009   0.274         1     16         2         0
freeport seed 4: 5 epochs, 594003 events, 0 rejected commands, 0.9s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    1      42   99.4       1.02       1.14  0.009   0.281         2     22         2         0
    2      42   99.5       1.02       1.14  0.009   0.274         1     16         2         0
    3      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    4      42   99.4       1.02       1.14  0.009   0.281         2     22         2         0
freeport seed 5: 5 epochs, 594953 events, 0 rejected commands, 1.1s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   99.4       1.02       1.14  0.009   0.281         2     22         2         0
    1      42   99.4       1.02       1.14  0.009   0.281         2     22         2         0
    2      42  100.0       1.02       1.14  0.007   0.289         0     16         2         0
    3      42   99.6       1.02       1.14  0.009   0.269         1     17         2         0
    4      42   99.6       1.02       1.14  0.009   0.269         1     17         2         0
```

| Target (GDD §17) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 99.4–100.0 % every epoch | yes |
| Price index within ±30 % of the basket | 1.02–1.14 every epoch | yes |
| No persistent stock-out | longest run of cycles without a Food ask: 2 (the first two cycles of every epoch, while the Mills work through the seeded Grain) | yes |
| Materials sinks balanced (band 0.05–0.6 of Materials into Machines) | 0.269–0.289 | yes |
| Rejected householder commands | 0 in about 595 000 events per seed | yes |

Also observed (not gate metrics): hardship peaks at 0–2 citizens per epoch, consumption Gini 0.007–0.011, unemployment 15–22 of 40. The unemployment number is the legacy hiring rule (Q2): the six legacy firms hire only while inventory is below two cycles of sales, and 40 householders consuming 24 Food and 4 Wares per cycle need about half the workforce to be fed and clothed at the base rates. It is a fact about the recipe constants, not a fault, and it becomes a design question once humans arrive (a human with no job and a full pantry has nothing to do). Recorded as Q46.

## What changed and why

1. **Day one had no stock (S0.14b).** Report 00 traced the collapse to cycle 0: no Food exists until the Mills have bought Grain and milled it, so most meters fall below 20 in the first cycle and the Q20 trap makes the hardship permanent. No tunable could create stock, so the pre-authorised engine session added `seeding.legacy_inventory` (ADR-0004 had reserved the term). Mills are seeded with a cycle of Food and Grain, Foundries with Ore. That alone takes need-fulfillment from 2–4 % to 98–99 % at the GDD's own needs values.

   Seeding Materials into the Machine Shop was tried and dropped: seeded Materials are consumed but never produced, so they pushed the investment-share ratio to 0.66. The Machine Shop instead waits for the Foundry and Workshop chain, which costs it nothing because machine demand only appears once treasuries have grown.

2. **Machine buying was lumpy (S0.14).** With `legacy_machine_buy_payroll_mult = 1.0`, one epoch per seed showed an investment share of exactly 0.65 while the others sat at 0.33–0.36. The CSV shows why: at 1.0, several legacy firms cross the buy threshold in the same cycle, and the Machine Shop's Materials draw for that burst is measured against a denominator that is smaller in exactly those epochs (unemployment 20 instead of 16, so fewer Materials produced). The band was met on average and missed in the burst epochs. Raising the multiplier to 1.5 staggers the buys and settles the share at 0.27–0.29 in every epoch; 2.0 lowers it further (0.21–0.32) with more spread. 1.5 is the smallest change that passes, so it is the one committed.

   The 0.05–0.6 band itself is ours (the GDD asks only for "a documented band"); it was set in S0.13b before any run and has not been moved to fit the result.

## Appendix B item 6

Neither change favors Freeport's logic over the others. Legacy inventory is a seeding constant every preset will use (the Commune's Common Store and the Directorate's State Stock need day-one stock for the same reason); the payroll multiplier is a householder-script constant, not a market rule. The headline numbers Freeport now posts (near-total need-fulfillment, a flat price index, low consumption Gini) are what forty identical householders on a script produce under any rule set that feeds them; they say nothing yet about markets versus planning. The comparison the GDD wants starts when the Commune and Directorate presets run the same seeds (Phase 4) and when humans, not scripts, make the accumulation decision.

## Open items carried into Phase 0b

- Q20 (sticky hardship) is untouched: a citizen whose meter reaches 0 still cannot climb out by eating at +4 per tick. Not triggered in householder runs now that day one has stock; a human who runs out of money will hit it. Decide with Chris before Phase 1 (candidates: `food_meter_per_unit` 5–6, or a second unit eaten below the hardship line).
- Q46 (structural unemployment of 15–22 with 40 citizens) as above.
- Stock-out of 2 cycles at every epoch start is the seeded Grain being milled; a Mill seed of Food for two cycles would remove it, at the cost of a less honest day one.
