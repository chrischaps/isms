# The Republic — tuning report 01 (first stable run, S0.17a)

**Sessions:** S0.17a · **Date:** 2026-09-13 · **Command:** `make sim-check PRESET=republic` (= `cargo run -p isms-sim --release -- run --preset republic --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/republic.toml` as committed with this report. The Republic is Freeport plus a legislature (GDD 6.4); its economy is Freeport's householders and legacy firms, taxed. Values that differ from `freeport-01.md`:

| Tunable | Freeport | Republic | Kind |
|---|---|---|---|
| `policy.tax_rate` | none | 0.20 flat, `tax_brackets = []` | the legislature's own policy (GDD 6.4); the value is Q14's |
| `policy.need_floor_food` | none | 24 | the legislature's own policy; the value is Q14's |
| `policy.minimum_wage_credits` | none | 6.00 (was 0.0) | the legislature's own policy; moved by this pass so enforcement is real and non-binding (Q82) |
| `policy.public_dwellings` | none | 0 | the legislature's own policy |

No numeric tunable was moved: the Republic is stable at the legislature's defaults on the first run. Unions, collective agreements and the public bank flag are S0.17d and Phase 3.

## Result: stable

```
republic seed 1: 5 epochs, 596909 events, 0 rejected commands, 0.9s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   99.0       1.02       1.14  0.013   0.273         2     16         2         0
    1      42   98.9       1.02       1.14  0.014   0.294         2     18         2         0
    2      42   99.0       1.02       1.14  0.013   0.273         2     16         2         0
    3      42   99.2       1.02       1.14  0.012   0.297         1     16         2         0
    4      42   99.2       1.02       1.14  0.011   0.298         1     16         2         0
seeds 2-5: need 98.9-99.2, index 1.02-1.14, invest 0.273-0.298, hardship 1-2, unemp 15-18, stock-out 2, 0 rejected (597 000-600 000 events).
```

Seed 1, epoch 0, the treasury's books from the CSV:

| cycle | treasury (credits) | tax collected | floor paid | hardship | unemployed |
|---|---|---|---|---|---|
| 0 | 490 | 490 | 0 | 0 | 0 |
| 1 | 1002 | 512 | 0 | 0 | 0 |
| 10 | 4950 | 380 | 0 | 0 | 16 |
| 20 | 9152 | 481 | 0 | 0 | 3 |
| 41 | 18620 | 445 | 0 | 1 | 7 |

| Target (GDD §17, as derived for this preset) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 98.9–99.2 % every epoch | yes |
| Price index within ±30 % of the basket | 1.02–1.14 every epoch | yes |
| No persistent Food stock-out (≤ 3 consecutive cycles with no Food asks resting) | 2 (the first two cycles of every epoch, as in Freeport) | yes |
| Materials sinks balanced (investment share 0.05–0.6) | 0.273–0.298 | yes |
| Rejected householder commands = 0 | 0 | yes |

## What changed and why

1. Nothing was tuned. The tax-transfer mechanics (S0.17a) were laid over the Freeport economy and the first sweep passed.
2. **Where the Republic differs from Freeport, and why.** Need-fulfillment is 98.9–99.2 % against Freeport's 99.4–100 %, with one or two citizens in hardship per epoch against zero to two: a fifth of every wage now goes to the treasury, so the householders who start an epoch waiting for the Mills to sell have a little less to spend when Food appears. Unemployment (15–18 of 40) and the investment share (0.27–0.30) are Freeport's (Q46). The price index is identical: the legacy firms price on the reference basket, not on what citizens keep.
3. **The floor never binds; the treasury only accumulates.** The need floor is 24 Food at the last Food price, about 31 credits of means; a householder's endowment is 1000 and its wages 48 a cycle, so no householder ever falls under it, and `floor_paid` is 0 in every cycle. Tax comes in at 380–512 a cycle and nothing spends it: `public_dwellings` is 0 (the 40 seeded dwellings house everyone), the public bank is Phase 3, and the legislature that could lower the rate or raise the floor is Phase 3. The treasury ends every epoch near 18 600 credits. That is the legislature's default budget, recorded here rather than adjusted: the first citizen the floor reaches will be a human who runs out of money (Q20 is decided, so they can eat their way back), and the first spending decision is the legislature's.

## Neutrality (GDD Appendix B item 6)

The Republic's headline metric moves below Freeport's by exactly what its own tax takes, and above nothing: no other preset's constants changed. A Nordic social democrat (item 1) would recognise the flat income tax, the treasury, the money floor, the minimum wage and the public housing stock, and would object that the treasury is hoarded rather than spent, which is a legislature's failing and not the engine's (item 2: the hardship is the system's own, the same first-cycle wait Freeport has; item 3: the floor is the system's own provision, not a gift, and it is real, just not reached). The chosen minimum wage of 6.00 sits below the legacy wage of 8.00 so that it exists without touching the householder economy; whether it should bind is the legislature's question.

## Open items carried forward

- The treasury has no outlet until Phase 3's legislature (rate, floor, public dwellings, bank funding). `public_dwellings` stays 0; the mechanism is tested with 3 in `tests/tax.rs`.
- The floor is money against the Food price; the Shelter half is public dwellings only (Q81).
- Unions, collective agreements, dues and strikes are S0.17d and never trigger in the householder economy.
