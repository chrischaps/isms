# Neutrality review 00 — the five presets side by side (S0.18, Phase 0 exit)

**Date:** 2026-09-13 · **Inputs:** `freeport-01.md`, `commune-01.md`, `directorate-01.md`, `republic-01.md`, `commonwealth-01.md`; `make sim-all EPOCHS=5` with `--check`; `cargo test -p isms-sim --release -- --ignored invariants`.

GDD Appendix B asks three things of every preset: that a sincere adherent of the system would recognise it (item 1), that its hardships are the system's own and not a constant of ours (item 2), and that its comforts are the system's own provision and not a gift (item 3). Item 6 asks this review: where the five presets differ, is the difference the constitution's or ours?

## Method

Every preset is loaded from `_base.toml` plus its own file. `_base.toml` holds every constant shared by all five (recipes, needs, prices, the householder's plan, the legacy scripts). A preset file may set only its constitution (the eight axes, contracts, org kinds, communication, offices), its `[policy]` (the fields its own governance would set) and a `[params]` override. The review lists every place a preset differs from `_base.toml`, names whose choice it is, and reads the headline metrics against that.

## The constitution axes (by design; GDD 6)

| Axis | Freeport | Commune | Directorate | Republic | Commonwealth |
|---|---|---|---|---|---|
| ownership | private | collective | collective | private | cooperative |
| pricing | market | none | administered | market | market |
| compensation | contract | need | scale | contract | share |
| labor | free | norm | assigned | free | free |
| capital | open | none | none | open | public_bank |
| redistribution | none | total | provision | tax_transfer | provision |
| monitoring | high | low | medium | high | medium |
| governance | none | direct | committee | representative | representative |

These are the game's five ideal types and nothing in them is tuned.

## Every `[policy]` field, named to its system

| Field | Preset | Value | Whose rule |
|---|---|---|---|
| `work_norm_hours` | Commune | 6 | the assembly's norm (GDD 6.2); Q14's value |
| `rationing` | Commune | need_first | the assembly's rule (Q48) |
| `materials_split` | Commune, Directorate | 0.5 / 0.3 / 0.2 | the assembly's, the committee's (Q57) |
| `plan_bonus_fraction` | Directorate | 0.25 | the committee's (Q5) |
| `ratchet` | Directorate | true | the committee's; magnitude `ratchet_mult` = 1.0 (Q47) |
| `price_list_credits` | Directorate | food 2.50, wares 5.00, inputs at start prices | the committee's; food and wares moved by `directorate-01` so the list covers the wage fund |
| `wage_grades_credits` | Directorate | 6.00 … 9.00 by skill band 20 | the committee's (Q5) |
| `minimum_food_ration` | Directorate, Commonwealth | 8 (in kind), 24 (as money from the bank pool) | the committee's, the legislature's (Q75, Q92) |
| `tax_rate`, `tax_brackets` | Republic | 0.20 flat | the legislature's; Q14's value (Q67) |
| `need_floor_food` | Republic | 24 | the legislature's; Q14's value (Q68) |
| `minimum_wage_credits` | Republic | 6.00 | the legislature's; set below the legacy wage so it exists without binding (Q82) |
| `public_dwellings`, `public_bank` | Republic | 0, false | the legislature's; unread until Phase 3 (Q71) |
| `capital_levy` | Commonwealth | 0.01 | the legislature's; Q14's value (Q93) |
| `lending_rule` | Commonwealth | formula | the bank board's; `vote` waits for Phase 4 (Q94) |

Two of these were moved by a tuning pass and both are recorded with the failure they cured: the Directorate's price list (the till emptied by cycle 20 at 1.20/3.00) and the Republic's minimum wage (0 meant no enforcement). Neither moved a headline metric of another preset.

## `[params]` overrides per preset

None. Every preset seeds the same eighteen workplaces and forty dwellings. Every constant added by Phase 0b lives in `_base.toml` and applies wherever its mechanism exists: `labor.balance_weights` (Commune and Directorate placement), `labor.skill_band`, `governance.ratchet_mult`, `store.surplus_goods`, `coop.default_share_rule`, `coop.max_machines_per_member` (never binds), `bank.*`, `union.*`.

## Seeding paths (the four things a preset starts with that Freeport does not)

| Path | Who | What it is |
|---|---|---|
| goods into the Common Store instead of the org | Commune | the society's own stock (Q52, Q57) |
| goods and the legacy treasuries into the state stock and the till | Directorate | the state as one purse (Q60) |
| the Public Investment Bank, a society-owned association | Commonwealth | the levy pool (Q92) |
| the householder manager as a coop's first member | Commonwealth | the seeding of a coop must give it a member (Q90) |

## Headline metrics side by side (seeds 1–5, five epochs each; `make sim-all`)

| Metric | Freeport | Commune | Directorate | Republic | Commonwealth |
|---|---|---|---|---|---|
| need-fulfillment | 99.8–100 % | 100 % | 100 % | 100 % | 96.5–100 % |
| price index | 1.02–1.14 | – | – | 1.02–1.14 | 1.02–1.14 |
| consumption Gini | 0.007–0.009 | 0.000 | 0.028 | 0.008–0.010 | 0.009–0.050 |
| investment share (ratio of sums, Q100) | 0.249–0.257 | 0.067 | 0.067 | 0.250–0.262 | 0.141–0.247 |
| worst hardship | 0–1 | 0 | 0 | 0 | 0–9 |
| worst unemployment | 15–18 | 0 | 0 | 15–18 | 0 |
| stock-out run | 2 | 0 | 0 | 2 | 0–1 |
| rejected commands | 0 | 0 | 0 | 0 | 0 |

Freeport and the Republic moved from their reports under Q97 (hunger before savings); the reports' `invest` columns were means of ratios (Q100). Neither moved a pass to a fail or back.

## Reading the differences (items 2 and 3)

- **The Commune and the Directorate have no hardship and no unemployment** because their systems place everyone (the norm, the assignment) and feed everyone (the store, the ration). That is their provision, not a gift: the Commune's members work a six-hour norm for it and the Directorate's citizens buy at list from a till their wages came out of. The Commune's Gini of exactly zero is need-first rationing with enough stock; the Directorate's 0.028 is its wage grades.
- **Freeport and the Republic carry 15–18 unemployed of 40** in every epoch. This is Q46 (structural: the legacy firms hire to a demand-capped headcount) and it is not a constant of ours that the other presets escape; the Commune and the Directorate simply have no such concept. The Republic's tax takes a fifth of every wage and its floor never binds (`republic-01`); its hardship is zero because Q97 now lets a saver eat.
- **The Commonwealth's hardship (0–9 in a bad cycle) is the system's own** in a precise sense: a member of a glutted coop that admits nobody and sells little gets a small share, and the provision is only as deep as a one-per-cent levy on a few hundred Machines makes the pool (`commonwealth-01`). Three householder rules were needed to get here and none is a favour to the preset: Q97 applies in every money society, Q99 stops a mover jumping into nothing, Q98 never binds.
- **The price index is identical in the three market presets** because the legacy firms price on the reference basket in all of them; it says nothing yet about the systems.

## Flags for review

1. The Commonwealth's pool drains in the last quarter of every epoch; the levy rate and the floor are the legislature's (`policy`), and the first human legislature will meet this.
2. Lone stewards of glutted coops capture the coop's whole surplus (Farm and Mill stewards end an epoch with four thousand credits while late joiners hold two hundred). The admission vote exists (Q95) but the householder uses the steward's `AdmitMember`, and nobody founds a coop in Phase 0.
3. `coop.max_machines_per_member` never binds and could be retired if a later pass shows it inert.
4. The investment band is judged on a ratio of sums since S0.17c; the four earlier reports print means of ratios and were not rewritten.
5. `lending_rule = vote` and `public_bank` are read as their formula / not at all until the ballots and the treasury-funded bank exist (Phase 3–4).

## Open items carried into Phase 1 for Chris

Q20 (sticky hardship: decided, `food_meter_per_unit` 6) and Q46 (structural unemployment in the two market-with-wages presets) from 0a; Q47 (ratchet 1.0), Q68 (the need floor as money, Shelter deferred), Q72/Q73 as merged (membership-only coops, no wage advance; Q86/Q87 in the file), Q92–Q94 (bank shape and formula), Q101 (dues and strike pay), Q97 (hunger before savings, universal).

## Phase 0 exit checklist

- [x] five `stability_<preset>` tests green on seeds 1–5 (`make sim-all EPOCHS=5` with `--check`, this session)
- [x] five `<preset>-01.md` reports
- [x] `neutrality-00.md` (this note)
- [x] `tests/invariants.rs` green: one epoch per preset in `make check`, five epochs × five seeds ignored and run once by hand this session
- [x] `.github/workflows/sim-nightly.yml` committed (schedule and `workflow_dispatch`)
- [x] every Q entry answered (Q1–Q103)
- [x] `docs/SESSIONS.md` complete through S0.18
- [x] goldens regenerated at every shape change
- [ ] cross-arch job dispatched once against main after S0.18 merges (`gh workflow run "cross-arch determinism"`)
