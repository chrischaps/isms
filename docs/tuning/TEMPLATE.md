# <Preset> — tuning report NN (<one-line purpose: first raw run / Phase 0 exit / …>)

**Sessions:** S0.x · **Date:** YYYY-MM-DD · **Command:** `make sim-check PRESET=<preset>` (= `cargo run -p isms-sim --release -- run --preset <preset> --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/<preset>.toml` as committed with this report. Values that differ from the previous report:

| Tunable | Previous | Now | Kind |
|---|---|---|---|
| `section.key` | old | new | config / new tunable (session, ADR, Q) |

## Result: stable / not stable

```
<paste one `table()` block per seed, seeds 1–5>
```

| Target (GDD §17, as derived for this preset) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | | |
| Price index within ±30 % of the basket (market presets only) | | |
| No persistent Food stock-out (≤ 3 consecutive cycles without Food available) | | |
| Materials sinks balanced (investment share 0.05–0.6) | | |
| Rejected householder commands = 0 | | |

## What changed and why

1. <one numbered item per tunable moved, with the evidence table or the run that motivated it>

## Neutrality (GDD Appendix B item 6)

<Does this change move this preset's headline metrics because of the system's own logic or because of our constants? Compare against the other presets' latest reports. Name the advocate (Appendix B item 1) who would call the outcome fair.>

## Open items carried forward

- <Q numbers, engine changes deferred, metrics that pass narrowly>
