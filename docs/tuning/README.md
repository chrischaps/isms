# Tuning reports

Per-preset stability reports from the headless simulator (Phase 0). `freeport-00.md` is the first raw run (S0.13); `freeport-01.md` the tuned result and Phase 0a exit (S0.14b, S0.14); `commune-01.md` the first stable Commune (S0.15).

## Running a sweep

```
make sim PRESET=freeport EPOCHS=5 SEED=1          # one run, CSV in docs/tuning/runs/
cargo run -p isms-sim --release -- run --preset freeport --epochs 5 --seeds 1..5 --out docs/tuning/runs
cargo run -p isms-sim --release -- run --preset freeport --param params.money.legacy_wage_credits=7.0 --check
make sim-check PRESET=freeport                     # the GDD 17 targets over seeds 1..5 (ignored test)
make sim-all                                       # all five presets over seeds 1..5, one table each, --check
```

Every preset has its own `stability_<preset>` test (`make sim-check PRESET=commune` and so on); the
targets are derived from the preset's capabilities by `StabilityTargets::for_capabilities`, so a
moneyless society is never judged on a price index and "stock-out" means what it means in that
system: no Food asks resting at cycle end (market), or Food requested from the store and not served
during the cycle (Common Store and state stock; the `food_unfilled` and `stocked_out` columns).

`--param` takes `params.<section>.<key>=<toml value>` and applies before validation, so any tunable in `_base.toml` or the preset can be swept without editing files. `--check` exits 1 when any epoch misses a target.

## Reading the table

One line per epoch: mean need-fulfillment (target >= 95%), min/max price index over the epoch (target 0.7..1.3), mean consumption Gini, mean investment share (Materials to Machine Shops over Materials produced; band 0.05..0.6), the worst hardship count, the worst unemployment, the longest run of stocked-out cycles (at most 3), and rejected householder commands (must be 0: a rejection is a script bug).

The CSV has one row per cycle with every `CycleAggregates` field plus the Food and Wares ask depth and the Food last price, so a sweep can be plotted or diffed.

## Writing a report

Start from `TEMPLATE.md`. A report names the seeds and parameter values, pastes the per-epoch tables, states which targets pass, and answers GDD Appendix B item 6 in prose: does the change move this preset's headline metrics because of the system's logic or because of our constants?
