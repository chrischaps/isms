# The Commune — tuning report 01 (first stable run, S0.15 exit)

**Sessions:** S0.15a, S0.15b, S0.15c · **Date:** 2026-09-13 · **Command:** `make sim-check PRESET=commune` (= `cargo run -p isms-sim --release -- run --preset commune --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/commune.toml` as committed with this report. Values that differ from the Freeport report (`freeport-01.md`):

| Tunable | Freeport | Commune | Kind |
|---|---|---|---|
| `labor.balance_weights` | unused | `{ farm 3, mine 2, foundry 2, mill 3, workshop 2, machine_shop 1, builder 1 }` | new tunable (S0.15c, Q62); the householder's workplace choice under a norm |
| `policy.work_norm_hours` | none | 6 | the Commune's own policy (GDD 6.2) |
| `policy.rationing` | none | `need_first` | the Commune's own policy (GDD 6.2, T8) |
| `policy.materials_split` | none | wares 0.5 / machines 0.3 / dwellings 0.2 | the Commune's own policy (GDD 6.2); the value is ours (Q59) |

No numeric tunable was moved: the Commune is stable at the GDD's starting values on the first run.

## Result: stable

```
commune seed 1: 5 epochs, 121736 events, 0 rejected commands, 0.2s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42  100.0          -          -  0.000   0.067         0      0         0         0
    1      42  100.0          -          -  0.000   0.067         0      0         0         0
    2      42  100.0          -          -  0.000   0.067         0      0         0         0
    3      42  100.0          -          -  0.000   0.067         0      0         0         0
    4      42  100.0          -          -  0.000   0.067         0      0         0         0
seeds 2-5: identical to seed 1, event for event (121736 events each).
```

| Target (GDD §17, as derived for this preset) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 100.0 % every epoch | yes |
| Price index within ±30 % of the basket | not applicable: no prices (A2 = none) | n/a |
| No persistent Food stock-out (≤ 3 consecutive cycles with unserved store requests) | 0: every draw request was served in full in every cycle | yes |
| Materials sinks balanced (investment share 0.05–0.6) | 0.067 every epoch | yes, near the floor |
| Rejected householder commands = 0 | 0 | yes |

## What changed and why

1. Nothing was tuned. The three engine sessions gave the Commune its mechanics (the store as the society's stock, draws by need, the balancing rule for positions, the split cap, the manager installing Machines) and the first sweep passed.
2. **Why the seeds do not differ.** The tick RNG is consumed only by rationing tie-breaks and the lottery (TDD §5.9), the monitoring noise (σ = 0.6 at A7 = low; it changes attributed output, which affects only the ledger), and the phase-2 shuffle. With every request served in full there is never a tie to break, the shuffle changes the order of requests that are all granted anyway, and no plan bids exist, so the five seeds produce the same events. The householder Commune is deterministic in the strong sense; humans will be the only source of variation.
3. **Why the investment share sits near the floor.** The split gives Machines 30 % of the Materials on hand each tick, but the Machine Shop has one worker (weight 1 of 39) working the six-hour norm: about 12 Machines per cycle, consuming 24 of roughly 360 Materials produced. The cap never binds; labor does. Raising the Machine Shop's weight would lift the share; nothing in the GDD asks for that, so it stays.
4. **Why the Gini is zero.** Every householder draws exactly what brings its meters to full and holds the same pantry, so consumption is identical by construction. A human who works less, or pledges more, would be the first difference the scoreboard shows.

## Neutrality (GDD Appendix B item 6)

The Commune's headline metric (need-fulfillment 100 %) beats Freeport's (99.4–100 %) by the width of Freeport's hardship count, which comes from Freeport's own logic: a householder there must earn before it eats, and the first cycle of each epoch has stock but no wages. The Commune has no such gap because the store serves need directly; that is the system's own provision (item 3), not a gift from the engine. The constants that differ are the Commune's own policy fields and one staffing table that stands in for the free choice GDD 6.2 gives its citizens. A Marxist advocate (item 1) would recognise distribution by need, contribution recorded and published, and no wage; they might object that the coordinators' Plan is not yet published (Phase 2) and that the Materials split is set by the sim rather than voted.

## Open items carried forward

- The investment band 0.05–0.6 is still the simulator's own reading of "roughly balanced" (freeport-01); the Commune sits at 0.067 by its labor allocation, not by the split.
- The `contribution_gini` aggregate (the Commune's scoreboard) lands with the S0.16b `CycleAggregates` change so the goldens regenerate once.
- Q59: the split is a per-tick consumption cap; whether the assembly's vote should instead reserve stock is a Phase 2 design question.
