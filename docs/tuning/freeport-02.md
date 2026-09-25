# Freeport — tuning report 02 (E-1: the runner's markup answers the shelf)

**Sessions:** E-1 (`s3.1-runner-markup`) · **Date:** 2026-09-24 · **Command:** `make sim-check PRESET=freeport` (= `cargo test -p isms-sim --release -- --ignored stability_freeport`, seeds 1–5, 5 epochs)
**Params:** `presets/_base.toml` and `presets/freeport.toml` as committed with this report. Values that differ from report 01:

| Tunable | Report 01 | Now | Kind |
|---|---|---|---|
| `householder.legacy_markup_step` | — (the markup was fixed) | 0.05 | new tunable (E-1, Q162) |
| `householder.legacy_markup_min` | — (floor: the start price) | 0.0 | new tunable (E-1, Q162) |
| `householder.legacy_markup_max` | — | 0.45 | new tunable (E-1, Q162) |

The rule (Q162, TDD 9.3 amended): at every cycle close a market org reads its shelf of each good it makes; the markup step falls by one when the closing stock is above the close before and rises by one when the shelf closed empty after producing; the ask is cost-plus at `legacy_markup + step × legacy_markup_step`, clamped to the band. The start-price floor is gone: the floor is cost at `legacy_markup_min`.

## Result: stable (all five presets)

```
freeport seed 1
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42  100.0       0.89       1.14  0.006   0.228         0     27         2         0
    1      42  100.0       0.89       1.14  0.007   0.226         0     22         2         0
    2      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
    3      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    4      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
freeport seed 2
    0      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
    1      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    2      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
    3      42  100.0       0.89       1.14  0.006   0.228         0     27         2         0
    4      42  100.0       0.89       1.14  0.008   0.270         0     30         2         0
freeport seed 3
    0      42  100.0       0.89       1.14  0.007   0.226         0     22         2         0
    1      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    2      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
    3      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    4      42  100.0       0.89       1.14  0.006   0.228         0     27         2         0
freeport seed 4
    0      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    1      42  100.0       0.89       1.14  0.007   0.226         0     22         2         0
    2      42  100.0       0.89       1.14  0.006   0.228         0     27         2         0
    3      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    4      42  100.0       0.89       1.14  0.007   0.226         0     22         2         0
freeport seed 5
    0      42  100.0       0.89       1.14  0.007   0.226         0     22         2         0
    1      42  100.0       0.89       1.14  0.007   0.226         0     22         2         0
    2      42  100.0       0.89       1.14  0.007   0.246         0     25         2         0
    3      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
    4      42  100.0       0.89       1.14  0.007   0.240         0     27         2         0
```

Main before this change (same command, `6d178e5`): need 99.8–100.0, index 1.02–1.14, hardship 0–1, invest 0.249–0.257, unemp 15–18, stock-out 2, rejected 0. The Republic tracks Freeport line for line as before; the Commonwealth's band is the same 0.89–1.14 with its usual hardship counts (0–10); the Commune and the Directorate have no prices and are unchanged.

| Target (GDD §17) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 100.0 % every epoch (99.8–100.0 before) | yes |
| Price index within ±30 % of the basket | 0.89–1.14 every epoch (1.02–1.14 before) | yes |
| No persistent stock-out | 2, the first two cycles of every epoch, as before | yes |
| Materials sinks balanced (0.05–0.6) | 0.226–0.270 (0.249–0.257 before) | yes |
| Rejected householder commands | 0 | yes |

## What the shelf did (seed 1, epoch 0, per-cycle CSV)

| cycle | Food last price | index | Food asks resting | worst unemployed |
|---|---|---|---|---|
| 0–1 | 1.31 | 1.13 | 0 | 0 |
| 2 | 1.26 | 1.13 | 259 | 0 |
| 3 | 1.20 | 1.12 | 653 | 4 |
| 4 | **1.14** (cost) | 1.12 | 1 328 | 10 |
| 11 | 1.14 | 0.92 | 2 298 | 14 |
| 12–41 | 1.14 | 0.886 | 4 400 → 7 700 | 4–27 |

Every legacy shelf grows from the first close (grain 19 → 51 → 89 at one Farm; Food 13 → 18 → 22 at one Mill in the three-cycle golden), so every step falls: Food takes three cuts and sits on the cost floor from day 4; Wares and Machines take longer and the index reaches its floor of 0.886 on day 12. **The lever moves prices, and it moves them in one direction to one place**: forty householders at full staffing make more of everything than the plan's 24 Food a day buys, before and after this change (the resting Food asks grew to 10 086 by cycle 40 on main and to 8 929 here). A shelf that never sells out never steps up.

The one cost: worst-in-epoch unemployment rose from 15–18 to 22–30. At cost the firms earn no margin, so a payday finds thinner treasuries and `affordable` places fall. A quick sweep on seeds 1–2 (three epochs each, `--param`):

| variant | index_min | worst unemp |
|---|---|---|
| `legacy_markup_min = 0.0` (committed) | 0.89 | 22–27 |
| `legacy_markup_min = 0.05` | 0.93 | 20–27 |
| `legacy_markup_min = 0.10` | 0.98 | 22–24 |
| `legacy_markup_min = −0.10` (below cost) | 0.80 | 24–28 |
| `legacy_markup_step = 0.02` | 0.89 | 21–33 |

The rise is not the floor's level; any fall in the asks brings it, so the defaults stay. Need-fulfillment is 100 % and hardship 0 throughout: the unemployed are fed from the glut at a lower price.

## Neutrality (GDD Appendix B item 6)

The change is one rule for every market org in every market preset, with no per-preset value; Freeport and the Republic move identically and the Commonwealth's coops step the same way. The metrics moved because of the system's logic (a seller with a growing shelf lowers its price), not because a constant was chosen for a preset. The Freeport advocate would say a market where a glut finally cheapens Food is fairer than the one before; the Commonwealth's would note the coops' surplus per member falls with the margin, which is what a glut does to a coop.

## Open items carried forward

- Q162: whether `legacy_markup_min` should go below zero (a firm clearing below cost, then shrinking, is how a glut ends in a market; the sweep above says it costs 0.09 of index and nothing in need) is a tuning decision for Chris, not a default to change here.
- The structural glut itself: the recipes' start rates against the plan's fixed 24 Food; the next lever after SJ.5/SJ.6 if the basket is still a line (`docs/plans/market-liveliness.md`, "What done looks like").
- Worst-epoch unemployment 22–30 out of 40 householders: not a GDD 17 target, but a number SJ.6 (players as the labour force) should read against.
