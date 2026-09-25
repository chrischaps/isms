# Freeport — tuning report 03 (E-7: the runner's wage answers its staffing)

**Sessions:** E-7 (`s3.3-runner-wage`) · **Date:** 2026-09-25 · **Command:** `make sim-check PRESET=freeport` (= `cargo test -p isms-sim --release -- --ignored stability_freeport`, seeds 1–5, 5 epochs), and `cargo run -p isms-sim --release -- run --preset freeport --param params.population.floor=12` for the low-floor epoch the card's gate asks for
**Params:** `presets/_base.toml` and `presets/freeport.toml` as committed with this report. Values that differ from report 02:

| Tunable | Report 02 | Now | Kind |
|---|---|---|---|
| `householder.legacy_wage_step` | — (the offer was the median open offer, or the legacy wage) | 0.05 | new tunable (E-7, Q164) |
| `householder.legacy_wage_min` | — | 1.0 | new tunable (E-7, Q164) |
| `householder.legacy_wage_max` | — | 2.0 | new tunable (E-7, Q164) |

The rule (Q164, TDD 9.3 amended; the labour-side twin of Q162): at every cycle close each market workplace reads its board. The step falls by one when the org's shelf of the workplace's output grew (the firm has more than it can sell, whatever the board says) and otherwise rises by one when an offer of the org's stood unfilled at the close (the wage is too low for the hands there are); it saturates at the band's edges. The offer is `legacy_wage × (1 + step × legacy_wage_step)`, clamped to `legacy_wage_min..=legacy_wage_max`: never under the legacy wage, at most twice it. The median of open offers is retired as the base. An open offer the firm cannot honour — no place it can pay a cycle for at the board's wage, a glutted shelf, or a wage the board has moved off — is withdrawn, and re-posted at the board's wage while a place remains.

## Result: stable (all five presets)

```
freeport seed 1
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42  100.0       0.89       1.14  0.023   0.300         0     22         2         0
    1      42  100.0       0.89       1.14  0.027   0.245         0     26         2         0
    2      42  100.0       0.89       1.14  0.022   0.280         0     26         2         0
    3      42  100.0       0.89       1.14  0.010   0.335         0     31         2         0
    4      42  100.0       0.89       1.14  0.010   0.335         0     31         2         0
freeport seed 2
    0      42  100.0       0.89       1.14  0.018   0.293         0     26         2         0
    1      42  100.0       0.89       1.14  0.015   0.337         0     31         2         0
    2      42  100.0       0.89       1.14  0.016   0.303         0     26         2         0
    3      42  100.0       0.89       1.14  0.025   0.285         0     24         2         0
    4      42  100.0       0.89       1.14  0.021   0.264         0     23         2         0
freeport seed 3
    0      42  100.0       0.89       1.14  0.028   0.239         0     23         2         0
    1      42  100.0       0.89       1.14  0.015   0.305         0     24         2         0
    2      42  100.0       0.89       1.14  0.014   0.283         0     21         2         0
    3      42  100.0       0.89       1.14  0.029   0.305         0     25         2         0
    4      42  100.0       0.89       1.14  0.021   0.273         0     29         2         0
freeport seed 4
    0      42  100.0       0.89       1.14  0.012   0.352         0     26         2         0
    1      42  100.0       0.89       1.14  0.028   0.236         0     23         2         0
    2      42  100.0       0.89       1.14  0.017   0.278         0     28         2         0
    3      42  100.0       0.89       1.14  0.022   0.295         0     22         2         0
    4      42  100.0       0.89       1.14  0.029   0.236         0     23         2         0
freeport seed 5
    0      42  100.0       0.89       1.14  0.015   0.292         0     26         2         0
    1      42  100.0       0.89       1.14  0.020   0.262         0     26         2         0
    2      42  100.0       0.89       1.14  0.027   0.284         0     24         2         0
    3      42  100.0       0.89       1.14  0.021   0.305         0     26         2         0
    4      42  100.0       0.89       1.14  0.014   0.283         0     21         2         0
```

Main before this change (report 02, `dc89d35`): need 100.0, index 0.89–1.14, hardship 0, invest 0.226–0.270, unemp 22–30, stock-out 2, rejected 0, consumption Gini 0.006–0.008. The Republic tracks Freeport line for line as before; the Commonwealth's band is the same 0.89–1.14 with its usual hardship counts (0–10, need 97.0–100.0); the Commune and the Directorate have no wages and are unchanged.

| Target (GDD §17) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 100.0 % every epoch | yes |
| Price index within ±30 % of the basket | 0.89–1.14 every epoch (unchanged) | yes |
| No persistent stock-out | 2, the first two cycles of every epoch, as before | yes |
| Materials sinks balanced (0.05–0.6) | 0.236–0.352 (0.226–0.270 before) | yes |
| Rejected householder commands | 0 | yes |

What moved at the forty-householder floor: the consumption Gini, 0.006–0.008 → 0.010–0.029, because wages now differ between firms (the per-citizen mean cycle wage spans 42–65 credits within an epoch where it spanned 41–64), and the investment share, up a little, because a firm whose shelf grows lowers its offer and keeps more for Machines. Worst-epoch unemployment is 21–31 against 22–30: unchanged within the noise.

## The low-floor epoch (the card's gate)

`--param params.population.floor=12`, seed 1, before and after; the same shape on seeds 2–3.

| | need % | hardship | stock-out cycles | real output | worst unemp |
|---|---|---|---|---|---|
| main (`7591cfe`) | 73.4 / 75.0 | 12 | 18 / 16 | 0 from cycle 1 to cycle 10, then a few hundred | 11 |
| E-7 | 86.1 / 86.1 | 12 | 9 / 9 | 0 to cycle 4, then 10, 30, 30, 87, 395, 748, 1 282, and 100 % need from cycle 10 | 9 |

What happens, from the per-cycle detail (`--detail`): twelve householders take the twelve lowest-numbered open offers, one each — three farms, three mines, three workshops, the machine shop, two Builders — and the three mills and three foundries get nobody. On main the workshops pay 64 a day for nothing, miss payroll at cycle 3's close, fire their hand, and *re-hire the same hand through their own stale 8.00 offer* the next hour (the offer posted at cycle 0 was still open), to miss payroll again; the mills' offers, also 8.00, never win a tie-break, and the town runs out of Food at cycle 4 for good. With E-7 every board closes short at cycle 0 (twelve hands for eighteen workplaces) and rises to 8.40; from cycle 1 the boards diverge — a farm's grain shelf grows (the mills buy none) and its board steps back, an idle mill's keeps rising a step a day (8.80, 9.20, 9.60, …). A broke workshop withdraws its offer (no place it can pay for), so when it frees its hand at cycle 4 the hand takes the best wage on the board: the foundries and the mills tie, and the tie-break sends four to the foundries and one to a mill. The foundries' Materials shelf then grows with nobody to buy it, their boards fall, and when the farms break at cycle 7 the four hands they free go to the mills, now the best offer in town at 11.20: Food from cycle 8, need 100 % from cycle 10, and the town stays fed.

Five hungry cycles remain, and the sweep below says no value of the step or the ceiling shortens them: the clock is how long an idle-paying firm takes to break, because the script's householder never leaves a job for a better wage (Q165). The wage decides *where* the freed hands go, and it decides it right; it cannot yet decide *when*.

| variant (floor 12, seeds 1–2, two epochs) | need % | worst unemp |
|---|---|---|
| committed (step 0.05, max 2.0) | 86.1 | 8–9 |
| `legacy_wage_step = 0.10` | 86.1 | 9–10 |
| `legacy_wage_step = 0.02` | 84.1–86.1 | 9–11 |
| `legacy_wage_max = 1.5` | 86.1 | 8–9 |
| `legacy_wage_max = 3.0` | 86.1 | 8–9 |

At the forty floor the same variants leave need at 100 % and unemployment within 22–31; `max` 1.5, 2.0 and 3.0 are identical there, so no board in a full town ever climbs ten steps. The defaults stay.

## Neutrality (GDD Appendix B item 6)

One rule for every market workplace in every market preset, no per-preset value; Freeport and the Republic move identically, and the Republic's minimum wage still sits under every offer because the band's floor is the legacy wage. The Commonwealth's coops admit members instead of posting offers, so their boards only ever fall and their offer is the legacy wage as before. The metrics moved because of the system's logic (a firm that cannot staff bids more for hands; a firm that cannot sell bids less), not because a constant was chosen for a preset.

## Open items carried forward

- Q164's second half: the ask's cost basis is still labour at the legacy wage (E-1's cost-plus), so a wage rise does not pass into the price. Whether it should — a mill paying 11.20 asking as if it paid 8.00 sells at a loss — is the next question for the runner, and it is the mechanism by which a wage would reach the price graph in a lab week.
- Q165: a householder that quits for a better wage, if Chris wants the sim to answer wages on the worker side too; the floor-12 epoch is its gate.
- E-1's shelf step runs past the band without saturating (a Food shelf that grew forty cycles needs forty sell-outs to lift the ask); E-7's board saturates. The two should agree, and E-1's is the one to change.
- The shelf reading counts inventory net of what rests in the firm's own asks, so a seeded shelf coming back off the book reads as growth (Mill No. 3's 320 Food); a small E-1 follow-up.
