# Freeport — tuning report 00 (first raw run)

**Session:** S0.13b · **Date:** 2026-09-13 · **Command:** `cargo run -p isms-sim --release -- run --preset freeport --epochs 5 --seeds 1..5 --out docs/tuning/runs`
**Params:** `presets/_base.toml` and `presets/freeport.toml` as committed with this report (all GDD Appendix A and TDD Appendix A starting values, plus `legacy_machine_buy_payroll_mult = 1.0`, Q43).

## Result: not stable

Every seed fails three of the four GDD §17 targets. Prices are fine; needs are not.

```
seed 1
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42    2.9       1.13       1.14  0.044   0.009        40     36        42         0
    1      42    2.3       1.00       1.14  0.044   0.012        40     37        42         0
    2      42   32.5       1.00       1.14  0.076   0.156        27     10        14         0
    3      42    3.7       1.00       1.14  0.110   0.036        40     34        42         0
    4      42    3.2       1.00       1.14  0.066   0.045        40     38        42         0
seed 2
    0      42    2.1       1.00       1.14  0.047   0.011        40     40        42         0
    1      42   32.5       1.00       1.14  0.064   0.168        27      6        10         0
    2      42    3.1       1.00       1.14  0.050   0.011        40     36        42         0
    3      42    3.8       1.00       1.14  0.056   0.010        40     40        42         0
    4      42    2.3       1.13       1.14  0.038   0.208        40     39        42         0
seeds 3–5: the same shape (need 2–4%, hardship 40, unemployed 34–40, stock-out all epoch).
```

| Target (GDD §17) | Result | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 2–4 % (one epoch per seed reaches ~32 %) | no |
| Price index within ±30 % of the basket | 1.00–1.14 every epoch | yes |
| No persistent stock-out | Food asks absent for the whole epoch | no |
| Materials sinks balanced (0.05–0.6 investment share) | 0.01–0.2, erratic | no |
| Rejected householder commands | 0 | yes |

## Diagnosis

The economy starts correctly (everyone employed and housed by cycle 2, Food flowing at 1.31) and then falls into a trap that the rules make permanent:

1. **Cycle 0 is a ramp.** Nobody holds Food at the start; the Mills need Grain before they can sell, so the first Food reaches most pantries around tick 6–12. By then the Food meter (100 − 4 per tick) is near 20.
2. **Eating cannot lift the meter (Q20).** One Food per tick adds exactly the 4 points that normal effort burns, so a meter that has fallen below 20 stays there forever. Cycle 1 is therefore a hardship cycle for most citizens, and cycle 3 makes them destitute, whatever the Mills do.
3. **Destitution cut them off from work.** Under the first reading of Q26, an open-ended contract counted as "long", so destitute householders could not accept the legacy firms' offers. Fixed in S0.13b (commitment is the notice period, not the term); this run already includes that fix, which is why rejections are 0 and unemployment briefly recovers in one epoch per seed.
4. Once output falls, hardship never clears, so unemployment and stock-out follow. Prices stay anchored because legacy firms price at reference costs (Q44).

Three script bugs found on the way and fixed in this session (not tuning): legacy input bids escrowed whole treasuries; cost-plus prices compounded on the last price (index reached 78–2228); plan bids rested one cent below the asks forever. See docs/QUESTIONS.md Q44.

## A trial for S0.14

`--param params.needs.food_meter_per_unit=6` (eating one Food recovers +2 per tick at normal effort):

```
seed 1
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   94.0       1.02       1.14  0.042   0.361        23     18         4         0
    1      42   93.9       1.00       1.14  0.037   0.353        24     19         4         0
    2      42   94.5       1.00       1.14  0.036   0.340        20     16         3         0
    3      42   95.2       1.00       1.14  0.034   0.593        22     21         4         0
    4      42   94.2       1.00       1.14  0.045   0.341        20     18         3         0
seeds 2 and 3: 93–95 %, hardship 20–25 at worst, stock-out 3–5 cycles, investment 0.27–0.41.
```

That single change moves need-fulfillment from 3 % to 94 % and the investment share into the band. What remains is the cycle-0 ramp (hardship peaks of 20–25 are the first cycle's starvation) and some Food stock-outs of 3–5 consecutive cycles. Candidates for S0.14, in order: a small starting pantry for householders (a seeding choice, ADR-0004 allows seeded goods), `food_meter_per_unit` 5–6, and Mill/Farm headcount at seeding.

Appendix B item 6: the change helps every preset equally (it is a needs rule, not a market rule), so it is a constants correction, not a favor to Freeport.
