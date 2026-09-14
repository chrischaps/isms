# Tuning reports

Per-preset stability reports from the headless simulator (Phase 0). `freeport-00.md` is the first raw run (S0.13); `freeport-01.md` the tuned result and Phase 0a exit (S0.14b, S0.14); `commune-01.md` the first stable Commune (S0.15); `directorate-01.md` the first stable Directorate (S0.16); `republic-01.md` the first stable Republic (S0.17a); `commonwealth-01.md` the first stable Commonwealth (S0.17c); `neutrality-00.md` the five presets side by side and the Phase 0 exit checklist (S0.18).

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

## Nightly and the invariants

`.github/workflows/sim-nightly.yml` runs `sim-all --check` and the five-epoch invariants test every night and on demand (`gh workflow run sim-nightly`); it is not a PR gate. `crates/isms-sim/tests/invariants.rs` checks, at every cycle close of every preset, that money and every good are conserved, no meter is above full, no balance, treasury or till is negative, and that two runs from one seed give one world; the one-epoch form runs in `make check`, the five-epoch, five-seed form is `cargo test -p isms-sim --release -- --ignored invariants`.

## Reading the table

One line per epoch: mean need-fulfillment (target >= 95%), min/max price index over the epoch (target 0.7..1.3), mean consumption Gini, the epoch's investment share (Materials to Machine Shops over Materials produced, as a ratio of sums over the epoch's cycles; band 0.05..0.6), the worst hardship count, the worst unemployment, the longest run of stocked-out cycles (at most 3), and rejected householder commands (must be 0: a rejection is a script bug).

The CSV has one row per cycle with every `CycleAggregates` field plus the Food and Wares ask depth and the Food last price, so a sweep can be plotted or diffed.

## Writing a report

Start from `TEMPLATE.md`. A report names the seeds and parameter values, pastes the per-epoch tables, states which targets pass, and answers GDD Appendix B item 6 in prose: does the change move this preset's headline metrics because of the system's logic or because of our constants?

## Detail output

`--detail` (S0.14e) makes the same run also write six CSVs under `target/sim/<preset>-<seed>/` (change the parent with `--detail-dir`; `target/` is gitignored, and a five-epoch Freeport seed is about 20 MB, most of it the trade tape). The aggregates still go to `--out` as before, and a run with `--detail` is byte-identical to one without: the writer only reads.

```
make sim-detail PRESET=freeport SEED=1
cargo run -p isms-sim --release -- run --preset freeport --seeds 1..5 --detail --detail-ticks
```

Every file starts with `preset,seed,epoch,cycle`; ids are the engine's (`c12`, `o3`, `w5`, `k7` for a contract, `r9` for an order); money is in credits; hours are tick-hours (24 per real hour, Q5); meters are points (0–100). Enum labels are the engine's serde names (`machine_shop`, `state_enterprise`).

| File | One row per | Columns after the key |
|---|---|---|
| `citizens.csv` | citizen × cycle, at cycle close (dormant citizens included, `dormant=true`) | `citizen,handle,dormant,food_meter,shelter_meter,comfort_meter,balance_credits,pantry_food,pantry_wares,pantry_other,cycle_wages_credits,wages_total_credits,workplace,workplace_kind,org,contract,alloc_hours,effort,tick_hours_worked,attributed_output,skill_family,skill_level,skill_max_level,budget,fatigue_debt,output_mult,food_eaten,wares_consumed,housed,dwelling,in_hardship,destitute,defaulted`. `workplace` is the allocation with the most hours (else any position); `tick_hours_worked` is the change in cumulative skill hours over the cycle; `attributed_output`, `food_eaten`, `wares_consumed` are summed from the cycle's `Produced` and `TickResolved` events. |
| `citizens_ticks.csv` | citizen × tick (only with `--detail-ticks`; about 200 000 rows per five-epoch seed) | `tick,citizen,food_meter,shelter_meter,comfort_meter,balance_credits,pantry_food,pantry_wares,food_eaten,wares_consumed,output_mult,in_hardship` |
| `orgs.csv` | org × cycle | `org,name,kind,ownership,manager,treasury_credits,inv_grain,inv_ore,inv_materials,inv_food,inv_wares,inv_machines,workplaces,machines,employees,positions,members,wages_paid_credits,dividends_paid_credits,declared_dividend_credits,payment_missed`. `machines` are summed over the org's workplaces; `employees` counts contracts and `positions` counts workers at its workplaces, so norm societies are not blank. |
| `flows.csv` | workplace × cycle, from `Produced` | `workplace,kind,org,machines,workers,tick_hours,output_good,units,in_grain,in_ore,in_materials,in_food,in_wares,in_machines`. Group by `kind` for the recipe Sankey; a workplace with no `Produced` this cycle has zeros. |
| `trades.csv` | every `Trade` on an order book | `tick,phase,instrument,buyer,seller,buy_order,sell_order,qty,price_credits,value_credits`. `phase` is `round` when the fill happened inside a householder's own command and `tick` when a standing-plan order filled in the tick; `instrument` is a good or `share:o3`. |
| `moves.csv` | goods or money that moved outside the books, one row per good | `tick,kind,from,to,good,qty,amount_credits` with `kind` in `seeded`, `sale_accepted`, `transferred`, `drew`, `store_returned`; `good=credits,qty=0` for a money-only row. |
| `depth.csv` | instrument × tick, after the tick; header only where there are no order books | `tick,instrument,best_bid_credits,bid_qty,bid_levels,best_ask_credits,ask_qty,ask_levels,last_price_credits` for the six goods and every share book. |

In a moneyless preset the trade tape and depth files carry only the header and `moves.csv` carries the Common Store draws.
