# The Cooperative Commonwealth — tuning report 01 (first stable run, S0.17c)

**Sessions:** S0.17b, S0.17c · **Date:** 2026-09-13 · **Command:** `make sim-check PRESET=commonwealth` (= `cargo run -p isms-sim --release -- run --preset commonwealth --epochs 5 --seeds 1..5 --check`)
**Params:** `presets/_base.toml` and `presets/commonwealth.toml` as committed with this report. The Commonwealth is Freeport's market with every org a cooperative (GDD 6.5): members instead of employees, a share-out instead of wages, a capital levy on Machines that funds a Public Investment Bank, and a basic provision paid from the bank's pool. Values that differ from `freeport-01.md`:

| Tunable | Freeport | Commonwealth | Kind |
|---|---|---|---|
| `constitution.compensation` | wage | share | the system's own axis: members are paid only by share-out (Q87) |
| `constitution.capital` | private | public_bank | the system's own axis: the bank is a seeded society-owned association whose treasury is the levy pool (Q92) |
| `constitution.redistribution` | none | provision | the system's own axis: `minimum_food_ration` = 24 Food, paid as money from the pool before lending (Q92) |
| `policy.capital_levy` | none | 0.01 of installed Machines' value per cycle | the legislature's own policy; the value is Q14's (Q93) |
| `policy.lending_rule` | none | formula | first come first served, one loan per coop, `params.bank` rate 100 bp, cap 30.00, term 10 (Q94) |
| `params.coop.default_share_rule` | none | hours_weighted | S0.17b |
| `params.coop.max_machines_per_member` | none | 12 | S0.17c (Q98); never binds in five epochs, see below |

No numeric tunable was moved by this pass. What moved was the householder script, three times, each recorded as a Q entry with the failure it cures (Q97, Q98, Q99), and one statistic (Q100).

## Result: stable

```
commonwealth seed 1: 5 epochs, 519457 events, 0 rejected commands, 0.7s
epoch  cycles  need%  index_min  index_max  gini   invest  hardship  unemp  stockout  rejected
    0      42   98.6       1.02       1.14  0.043   0.147         4      0         1         0
    1      42   99.6       1.02       1.14  0.016   0.178         0      0         1         0
    2      42  100.0       1.02       1.14  0.009   0.247         0      0         0         0
    3      42   99.6       1.02       1.14  0.031   0.155         1      0         1         0
    4      42   98.3       1.02       1.14  0.037   0.141         3      0         1         0
seeds 2-5: need 96.5-99.8, index 1.02-1.14, invest 0.143-0.179, hardship 0-9, unemp 0, stock-out 0-1, 0 rejected (515 500-519 000 events).
```

Seed 1, epoch 0, the bank's books from the CSV:

| cycle | pool (credits) | loans outstanding | floor paid | surplus per member | mean tenure (cycles) | hardship |
|---|---|---|---|---|---|---|
| 0 | 0.7 | 0 | 0 | 53 | 0 | 0 |
| 5 | 5 | 38 | 0 | 100 | 3.2 | 0 |
| 10 | 75 | 100 | 0 | 106 | 4.9 | 0 |
| 20 | 390 | 76 | 0 | 133 | 9.3 | 0 |
| 30 | 408 | 83 | 61 | 91 | 13.9 | 0 |
| 41 | 8 | 18 | 38 | 40 | 19.2 | 4 |

The pool peaks near 520 credits around cycle 25 and the floor spends 1 081 credits over the epoch, almost all of it after cycle 28; the bank lends 38-106 credits at a time to coops buying their first Machine and every loan is repaid.

| Target (GDD §17, as derived for this preset) | Result over seeds 1–5, 5 epochs each | Pass |
|---|---|---|
| Need-fulfillment ≥ 95 % | 96.5–100.0 % every epoch | yes |
| Price index within ±30 % of the basket | 1.02–1.14 every epoch | yes |
| No persistent Food stock-out (≤ 3 consecutive cycles with no Food asks resting) | 0–1 | yes |
| Materials sinks balanced (investment share 0.05–0.6) | 0.141–0.247 (ratio of sums, Q100) | yes |
| Rejected householder commands = 0 | 0 | yes |

## What changed and why

1. **The bank, the levy and the provision (S0.17c, the card).** Each cycle end every coop pays `capital_levy` of its installed Machines' value (at the last Machines price) into the bank's treasury; the pool then pays the society's Food floor (24 Food at the last price, to whoever has less than that in balance plus pantry) exactly as the Republic's treasury does; what is left is lent by formula to coops that applied. The coop steward applies for the price of one Machine when it cannot afford one and buys when the loan lands. Loans are ordinary credit contracts with the bank as lender, so installments flow back to the pool at 8c.
2. **Before this pass the preset sat at 93–97 % need with the investment share out of band** (S0.17b's hand-off). The detail CSVs (`--detail`, S0.14e) showed three faults, none of them in the bank:
   - **Hunger before savings (Q97).** The householder keeps a tenth of lifetime wages and never spends it; a coop member whose share-outs stop starved with a hundred credits in hand, and the floor counted that reserve as means. With an empty pantry the reserve no longer applies to the Food bid. This is the rule that moved the headline: hardship was 4–12 a cycle, now 0–9 at the worst cycle of the worst epoch.
   - **Movers must have somewhere to go (Q99).** The mobility rule sent a member out whenever another coop had a place open, but a glutted coop admits nobody (the Q2 hiring cap), so in a glutted economy ten of forty citizens sat unplaced for half an epoch. The glut test now lives in the coop module and the request and mobility rules count only coops that would admit someone.
   - **A coop of one buys no more than twelve Machines per member (Q98).** The Builders' coops, one steward each and forty dwellings of rent, had turned every credit into Machines (350 each) and every unit of Materials the Foundries made into Machine Shop output. With Q97 and Q99 in place the cap never binds in five epochs (caps of 8, 10 and 12 run event-for-event alike); it stays as the guard against that run.
   - Tried and dropped: letting a coop under its reserve bid for inputs with half of what it holds. It produced its way into a deeper glut and need fell to 87 %.
3. **The statistic (Q100).** One cycle in which the Foundries made two units of Materials and the Machine Shop consumed thirty-eight scored an investment share of 19 and carried seed 5, epoch 3 to 0.619 as a mean of ratios; the epoch's ratio of sums is 0.156. The epoch share is now the ratio of sums. The four earlier reports' `invest` columns were means of ratios; re-run in this session, every epoch of every preset is inside the band under the new statistic too (Freeport 0.249–0.257, the Republic 0.250–0.262, the Commune and the Directorate 0.067).
4. **Freeport and the Republic moved.** Q97 is universal, and a handful of Freeport's and the Republic's unemployed do reach an empty pantry under their reserve: Freeport seed 1 is 594 244 events (was 594 015), the Republic 597 267 (was 596 909). Both re-run green on seeds 1–5 with need 99.8–100 %. The Commune and the Directorate are event-for-event unchanged (121 736 and 403 036).

## Neutrality (GDD Appendix B item 6)

The Commonwealth's headline is 96.5–100 % against Freeport's 99.8–100 %, and its hardship (0–9 in a bad cycle, against Freeport's 0–1) is the system's own: a member of a coop that sells nothing gets nothing, and the provision is only as deep as the levy makes the pool (item 2). A Mondragón cooperativist (item 1) would recognise membership in place of employment, the hours-weighted share, the levy on capital, the bank that lends to coops and the admission vote, and would object to two things this report does not hide: lone stewards capture a coop's whole surplus when the coop is glutted and admits nobody (Farm and Mill stewards end an epoch with four thousand credits while their late-joining members hold two hundred), and the pool drains in the last quarter of every epoch because a one-per-cent levy on a few hundred Machines is a thin purse for a floor. Neither is a constant of ours: the levy rate and the floor are the legislature's (`policy`), the admission rule is the members' (`ProposeAdmission`, S0.17c). Q97 and Q99 are corrections to the householder, not favours to the preset: Q97 applies in every money society and Q99 makes a member stay put rather than jump into nothing. Q98 never binds. Freeport's `invest` did not move as a ratio of sums except by the statistic.

## Open items carried forward

- The pool is thin late in every epoch (`floor_paid` outruns the levy from about cycle 28); whether the levy should be higher or the provision should draw on something else is the legislature's question (Q92 flagged for Chris).
- Lone stewards capture a glutted coop's surplus; the admission vote exists (Q95) but the householder uses the steward's `AdmitMember`, and nobody founds a coop.
- `lending_rule = vote` is treated as the formula until the bank board can vote (Q94); `max_machines_per_member` never binds and could be retired if a later pass shows it inert.
- Unions, collective agreements, dues and strikes are S0.17d and never trigger in the householder economy.
