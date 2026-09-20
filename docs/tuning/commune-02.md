# The Commune — tuning report 02 (the scripted assembly on the rolls, S2.4)

**Sessions:** S2.4 · **Date:** 2026-09-20 · **Command:** `make sim-check PRESET=commune` (= `cargo run -p isms-sim --release -- run --preset commune --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/commune.toml` as committed with this report. Values that differ from `commune-01.md`:

| Tunable | Previous | Now | Kind |
|---|---|---|---|
| `sim.assembly_size` | none | 6 (0 in `_base.toml`) | new tunable (S2.4, Q134), sim only: scripted humans joined before epoch 0 |
| `sim.split_nudge` | none | 0.05 | new tunable (S2.4), sim only: how far each cycle's split proposal moves |
| `sim.honor_every_cycles` | none | 10 | new tunable (S2.4), sim only |

The society's own policy is unchanged; what changed is who runs it. Six scripted humans now sit on the rolls beside 34 householders (the floor stays 40), work under the norm, draw from the Store, hold the three coordinator seats in two alternating slates, and vote the Materials split every cycle.

## Result: stable

```
commune seed 1: 5 epochs, 131138 events, 0 rejected commands, 0.3s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42  100.0          -          -  0.001   0.067         0      0         0         0
    1      42  100.0          -          -  0.001   0.066         0      0         0         0
    2      42  100.0          -          -  0.001   0.066         0      0         0         0
    3      42  100.0          -          -  0.001   0.066         0      0         0         0
    4      42  100.0          -          -  0.001   0.066         0      0         0         0
seed 2: 131162 events · seed 3: 131038 · seed 4: 131099 · seed 5: 131142; every table identical to seed 1's to three decimals.
```

| Target (GDD §17, as derived for this preset) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 100.0 % every epoch | yes |
| Price index within ±30 % of the basket | not applicable: no prices (A2 = none) | n/a |
| No persistent Food stock-out (≤ 3 consecutive cycles with unserved store requests) | 0 | yes |
| Materials sinks balanced (investment share 0.05–0.6) | 0.066–0.067 every epoch | yes, near the floor |
| Rejected householder commands = 0 | 0 (the assembly's commands count too) | yes |

The governance invariants (`crates/isms-sim/tests/invariants.rs`) hold across seeds 1–5 and five epochs: a `PolicyChanged` from a proposal in every epoch, three coordinators from cycle 1 of every epoch, no proposal open past its `closes_cycle`.

## What changed and why

1. **Nothing numeric was tuned.** The first run with the assembly passed every target.
2. **Why the seeds now differ.** The scripted humans are the first source of variation `commune-01` predicted: the tick RNG's phase-2 shuffle now orders 46 plans, and the humans' draws and hours differ across seeds by a few events. The epoch tables do not move at the printed precision.
3. **Why the investment share does not move.** The assembly's split hovers near even thirds (after two epochs on seed 1: wares 0.333, machines 0.341, dwellings 0.326). The split is a per-tick cap on Materials consumption per sink (Q59) and the cap never binds here: the Machine Shop's single worker consumes about 24 of some 360 Materials a cycle whatever its share, so labor, not the vote, sets the share. A tighter Commune (fewer Materials, more Machine Shop weight) would show what the vote is for.
4. **The Gini rose from 0.000 to 0.001.** Six humans return from the rollover unhoused for the first two cycles of epochs 1–4 (the roster reset puts every human to sleep, Q41; 8l trims the surplus householders at the end of cycle 0 and 8b houses the humans at the end of cycle 1). Shelter dips for six of forty-six for two cycles; the need-fulfilment rate does not register it.
5. **A first cut of the split script pinned the split at all-Machines.** It read "scarcest" as the good with the lowest Store stock, and the Store's Machines stock is always zero because managers install every Machine the tick it lands. Need fulfilment stayed at 100 % throughout, so the stability gate did not catch it. Scarcity is now read against needs the engine already states (unhoused citizens, Wares below tomorrow's entitlements, machines below one per worker) and drifts back toward even thirds when nothing is short (Q134).

## Neutrality (GDD Appendix B item 6)

The Commune's headline (100 % need fulfilment, zero hardship) is unchanged by the assembly; the constants added are the simulator's script, not the society's rules, and they are set to zero in the four presets that have no assembly, whose five-seed sweeps produce event counts identical to main's. The Marxist advocate of `commune-01` now has the two things they missed: the coordinators' Plan is publishable (S2.3) and the Materials split is voted, not set by the sim. They might now object that the vote changes nothing because Materials are not scarce, which is a true statement about this economy's tuning, not about the mechanism.

## Open items carried forward

- The split vote is inert while Materials are abundant (item 3). Whether to tighten the Commune's Materials so the accumulation decision bites is a tuning question for a later report, not a Phase 2 gate.
- The two unhoused cycles after each rollover (item 4) are the roster reset's design (Q41); a live human returning at epoch start sees the same. Visible on the Chronicle once S2.9 writes it.
- Q59 (whether the split should reserve stock rather than cap consumption) is still open; the assembly's script would work either way.
