# Sessions

Append-only log. One entry per session card (template: `docs/tdd.md` Appendix C).

## S0.1 — Repository scaffold — 2026-09-12 — PR #1
Built: Cargo workspace (edition 2024, resolver 3, pinned workspace deps, `unsafe_code = forbid`, clippy pedantic as warnings promoted to errors by `make check`), six empty crates, `web/` Vite 8 + React 19 + TS 6 placeholder with `pnpm check` (tsc, oxlint, vitest), Makefile, CI workflow, `presets/_base.toml` with every GDD App. A value and TDD App. A tunable plus the new ones from QUESTIONS/ADRs, five preset TOMLs, lexicon placeholders and `KEYS.txt`, Freeport welcome copy, `CLAUDE.md` (TDD App. D plus the libm and Windows notes), `docs/` layout with QUESTIONS.md pre-seeded (Q1–Q18).
Deviations from TDD: the Vite template now ships oxlint instead of ESLint; kept oxlint (same role, zero config). `make` on Windows is GNU make 3.81 from GnuWin32, run from Git Bash; recipes are plain cargo/pnpm calls. No ADR needed.
Provisional answers added to QUESTIONS.md: Q1–Q18 (from the pre-build TDD review); ADR-0003/0004/0005 are reserved for S0.3a/S0.4.
New tunables: `legacy_treasury_credits` (384), `legacy_hire_inventory_cycles_cap` (2), `legacy_offer_max_hours` (8), `legacy_offer_notice_cycles` (1), `living_cost_food` (24), `living_cost_wares` (4), `meter_start` (100), `food_full_output_meter` (50), per-good pantry caps, Republic `tax_rate`/`need_floor_food`, Commonwealth `capital_levy`.
Next session should know: toolchain pinned to 1.92.0; `_base.toml` keys are nested by topic (`params.labor.*`, `params.needs.*`, …) and money keys carry a `_credits` suffix that the S0.2 loader converts to cents. `presets/test/` is reserved for test-only fixtures. arm64 CI runners are not free on a private repo (affects S0.13b).
make check: green · new tests: 1 (web placeholder) · sim-check: n/a

## S0.2a — Core types, Params, preset loader — 2026-09-12 — PR #2
Built: `isms-core` modules `ids` (u32 newtypes, Tick/Cycle/Epoch aliases), `kinds` (Good, Product, WorkplaceKind with `job_family()`, JobFamily, Need, Effort + EffortTable, OrgKind, ContractKind, Channel, ClientKind incl. `Plan` per Q6, CitizenKind), `money` (Money cents newtype with credits<->cents serde helpers, exact-to-the-cent check), `constitution` (eight axis enums, OfficeSpec, Constitution with `has_money()`), `policy` (Policy with per-preset Option fields; credits fields renamed `*_credits` in TOML), `params` (nested Params mirroring `_base.toml`, `deny_unknown_fields`, no defaults), `config` (base + preset deep-merge overlay, Preset struct, cross-axis validation, `load_all`). Presets now use one `seeded_workplaces` key instead of legacy_firms/legacy_coops.
Deviations from TDD: constitution sets are `BTreeSet` rather than `EnumSet` for canonical serialization; `EnumSet` is reserved for `Capabilities` (S0.2b). The cross-axis constraints were placed in the loader now (Q14) rather than waiting for S0.2b; S0.2b adds the invalid-combination fixture tests.
Provisional answers added to QUESTIONS.md: none new.
New tunables: none beyond S0.1's.
Next session should know: `WORKSPACE_PRESETS_DIR` resolves presets relative to the crate at compile time (tests and sim); `Preset` round-trips through JSON (credits serialized as floats). `Capabilities`, the per-preset capability table test, lexicon KEYS test, and six invalid-combination tests are S0.2b.
make check: green · new tests: 11 · sim-check: n/a

## S0.2b — Capabilities, constraint fixture, lexicon test — 2026-09-12 — PR #3
Built: `Capabilities::derive(constitution, policy, params)` with the TDD 5.2 fields (money/order_books/administered/common_store flags, contract and org sets, labor/pay/capital/redistribution/governance, monitoring as overridden by policy plus the resolved sigma, offices, published land slots and rate limit) and `allows_contract`/`allows_org`; the per-preset capability table test; eight invalid axis-combination tests; Params TOML round-trip; preset-name-mismatch test; `lexicon` module (`required_keys` from KEYS.txt, `load_lexicon` with completeness check) and the every-preset-defines-every-key test; `scripts/pr-cycle.sh` (push, PR, wait for CI to register and pass, squash-merge).
Deviations from TDD: `Capabilities` uses `BTreeSet` instead of `EnumSet` (canonical list serialization for the API, no derive conflicts); `enumset` removed from the workspace. `proposal_kinds` is omitted until Phase 2 defines `ProposalKind`. `Capabilities::allows(&Command)` arrives with `Command` in S0.4.
Provisional answers added to QUESTIONS.md: none new.
New tunables: none.
Next session should know: PR #2 was merged before its CI check registered (the watch returned "no checks reported"); main's CI run for it passed afterwards. Use `scripts/pr-cycle.sh` from now on. S0.2 done gate is complete.
make check: green · new tests: 10 · sim-check: n/a

## S0.3a — World, events, apply, ledger — 2026-09-12 — PR #4
Built: `world` (World with SocietyMeta, Citizen/Household/LaborState/Skill/Needs/CitizenFlags/StandingPlan, Org with `Ownership::Shares { issued, holdings: BTreeMap<ShareHolder, u64> }`, Workplace/Assignment, LandRegistry from `params.land`, Dwelling/Owner, Instrument/Order/OrderBook, CommonStore, StateStock with a `till`, Contract/ContractBody, Offer/OfferBody, Offices/Proposal placeholders, `EscrowKey`-keyed escrow and share escrow, NextIds; `canonical_bytes()` = postcard, `hash()` = blake3); `event` (all TDD 5.4 variants with payloads, `Actor`, `WorkerOutput`, `CitizenDelta`/`WorkplaceDelta` carrying new absolute values, `CycleAggregates` stub, `kind()` and `ALL_KINDS`); `explain` (RuleId closed enum with `doc()`, Num, Explain builder); `ledger` (Party, Holder, Asset, LedgerMeta with seeded/produced/consumed/depreciated/burned, `conservation_check` incl. negative-balance and per-org share checks); `apply` for SocietyCreated, EpochStarted, Seeded, CitizenJoined, HouseholderJoined, PlanChanged, LaborSet, Transferred, TickResolved, with `UNIMPLEMENTED` registry and a test that every kind is exactly one of implemented or listed. ADR-0003/0004/0005 written.
Deviations from TDD: `TickResolved` deltas carry the citizen's new state (meters, budget, debt, skill) rather than arithmetic deltas; food/wares eaten leave the pantry to the `consumed` sink inside the same event (still no holder-to-holder movement, per D4). `Ledger` is not a separate struct: holders are the World's own fields and `ledger::holdings` sums them. `LedgerMeta.burned_money` is separate from per-good `burned`.
Provisional answers added to QUESTIONS.md: none new (ADRs cover the structural ones).
New tunables: none.
Next session should know: postcard cannot deserialize `#[serde(untagged)]`, `#[serde(flatten)]`, or fields with `skip_serializing_if`; none may appear anywhere in `World` or `Event`. The credits helpers now deserialize plain `f64`. `apply::credit`/`debit` are `pub(crate)` movers for later cards; escrow holders are keyed in `world.escrow` and moved by the market/contract events (S0.7/S0.8). S0.3b builds `test_support` (WorldBuilder that emits events, Harness with fold==live, proptest strategies, golden helper).
make check: green · new tests: 9 · sim-check: n/a

## S0.3b — Test harness — 2026-09-12 — PR #5
Built: `isms_core::test_support` (feature `test-support`, also on under `cfg(test)`; the crate dev-depends on itself with the feature so integration tests see it): `WorldBuilder` (humans/householders/pantry/balance_extra/with_preset/seed; builds only by emitting events), `Harness` (apply/apply_all with conservation + fold==live after every step, `check_every_step` toggle), `fold`/`assert_fold_equals_live` (JSON field diff on mismatch), `check_golden` (postcard bytes + .jsonl twin, `UPDATE_GOLDEN=1`), `assert_deterministic`, and `strategies` (proptest `Step`s named by index and fraction, resolved against the live world so every generated event is well-formed; `arb_scenario`, `run_scenario`). `tests/harness.rs` holds the S0.3 gate: builder/fold/golden/determinism tests, `apply` never panics + conserves (Freeport and Commune), meters and balances in range. First golden `s0_3b_builder` committed.
Deviations from TDD: none. `Harness::cmd`/`tick` arrive with `handle`/`tick` in S0.4.
Provisional answers added to QUESTIONS.md: none.
New tunables: none.
Next session should know: the crate's `Step` enum is the place to add command-shaped steps once `handle` exists; keep every step resolvable (never generate an id or amount that could be invalid). S0.3 done gate complete.
make check: green · new tests: 9 (+3 proptests) · sim-check: n/a

## S0.4 — Clock, RNG, tick skeleton, handle skeleton — 2026-09-12 — PR #6
Built: `tick` module (`TickInput::next_for` with `derive_seed` = blake3(society_seed, epoch, tick); `TickError::{EpochEnded, WrongTick}`; `TickBuilder` on a scratch clone with `emit` applying to the scratch world, seeded `ChaCha8Rng`, per-tick shuffled active citizens; the ten phases and the 8a–8m cycle-end steps as named functions; 8m emits `CycleClosed` with population counts and the collapse counter; phase 9 emits `EpochEnded { Scheduled }` after the last cycle or `{ Collapse }` when `collapse_enabled` and the low-population counter reaches `collapse_cycles`; phase 10 appends `TickResolved`; `start_epoch(world, rules, epoch) -> Vec<Event>` per ADR-0003, body deferred). `command` module (`Envelope` with `system`/`citizen`/`on_behalf_of`/`via` builders, the full `Command` catalog, `RejectCode` closed enum, `Reject`, `Capabilities::allows(&Command)`, `handle` gating epoch-ended then capabilities then dispatch; `Join` (endowment with Explain where money exists, handle uniqueness), `Seen` (throttled per tick), `EndEpoch` (operator) implemented; everything else `NotImplemented`). `rules` module. `apply` now handles `CitizenSeen`, `CycleClosed`, `EpochEnded`. `SocietyMeta.low_population_cycles` added; `CycleClosed` carries it. Harness gained `cmd`, `cmd_dry`, `tick`, `try_tick`, `run_cycle`, `run_cycles`.
Deviations from TDD: `TickInput` carries the derived 32-byte seed rather than the raw inputs; the collapse counter lives in `SocietyMeta` and is set by `apply(CycleClosed)` so replay reproduces it.
Provisional answers added to QUESTIONS.md: none (Q7 and Q10 applied).
New tunables: none.
Next session should know: fixtures with fewer than 40 humans collapse at cycle 4 unless `collapse_enabled` is set false via `WorldBuilder::with_preset`. `Harness::cmd` takes `Envelope` by value. `TickBuilder.citizen_deltas`/`workplace_deltas` are `BTreeMap`s keyed by id so phase code can update a delta more than once per tick. S0.4 done gate complete.
make check: green · new tests: 9 · sim-check: n/a

## S0.5 — Needs, consumption, hardship — 2026-09-12 — PR #7
Built: `needs` module: meters in integer tenths (`TENTHS`, `FULL`; `Needs::at_start`), `effort_for_decay` (Q19), `output_multiplier(needs, housed, destitute, params) -> (f64, Explain)` (Food term linear 1.0 -> floor over 50 -> 0, unhoused x0.7, destitute floor 0.25), `phase_5_needs` (auto-eat +4, Wares when Comfort can absorb +6, decay per effort 3.2/4/5.2, Shelter -2 unhoused / +2 housed, Comfort -1 or -2 unhoused, destitute Comfort forced to 0, clamp, low-food tick counting, next tick's `output_mult`, consumption to the ledger), `cycle_end_8h_hardship_and_fatigue` (hardship only when every tick of the cycle was below 20, consecutive counter, destitution at 3, fatigue debt = min(cap, high-effort half + hardship half), `budget` = base - debt, Hardship/Destitution Began/Ended events). `apply` handles the four flag events and `CitizenDormant`/`CitizenReturned` (flag only; freeze semantics are S0.9). `LaborState.output_mult` added. `TickBuilder.consumed` and phase 10 now builds a `CitizenDelta` for every non-dormant citizen from the scratch world. Harness gained `set_needs` (synthetic delta that does not advance the clock).
Deviations from TDD: meters are `u16` tenths rather than 0–100 integers so the GDD's fractional rates are exact; the multiplier's `Explain` is computed on demand by `output_multiplier` rather than carried in every delta (TickResolved size budget).
Provisional answers added to QUESTIONS.md: Q19 (decay effort = highest active allocation), Q20 (eating cannot lift the meter: hardship is sticky as specified; flagged for tuning and for Chris).
New tunables: none.
Next session should know: `LaborSet` to a `WorkplaceId` that does not exist is accepted by `apply` (tests use it to set effort); S0.6's `SetLabor` command must validate the assignment (Q16). High-effort fatigue reads `consecutive_high_effort_cycles`, which S0.6 maintains.
make check: green · new tests: 8 (+1 proptest) · sim-check: n/a

## S0.6a — Labor and production — 2026-09-12 — PR #8
Built: `labor` module: `skill_level` (k ln(1 + h/h0), capped, via libm), `skill_mult`, `capital_mult` (1 + coeff ln(1 + machines/workers), 1.0 with no workers), `phase_3_labor` (integer tick-hours per assignment, pro-rata to budget, Q4/Q16), `phase_4_production` (per-worker true output base x skill x effort x needs x capital x hours with an Explain, attribution = true x max(0, 1 + N(0, sigma)) sampled only when sigma > 0, integer units with carried remainder capped by inputs (Q21), input consumption and output to the org inventory, per-cycle worker accumulators for payroll, skill accrual), `cycle_end_8g_skill_and_effort` (idle-family decay -1 per 10 idle cycles, high-effort streak counter, cycle accumulator resets), `set_labor` (max workplaces, assignment required, sum of hours within budget, duplicates rejected), `skill_of`. New events `Assigned`/`Unassigned` (Q22). `apply` handles `OrgFounded` (fee burned from the founder), `WorkplaceAdded` (Materials from org inventory, slot claimed), `Produced`, `Assigned`, `Unassigned`. `Assignment` gained cycle accumulators and `Skill` a per-cycle hours counter; `WorkplaceDelta` carries the accumulators. `WorldBuilder` gained `org`, `workplace`, `org_inventory`, `assign`.
Deviations from TDD: `SetLabor` uses integer tick-hours (Q5); the Builder produces nothing until S0.11 (Q17).
Provisional answers added to QUESTIONS.md: Q21 (input-capped remainder resets), Q22 (`Assigned`/`Unassigned` events).
New tunables: none.
Next session should know: skill grows within the very first tick, so second-tick expectations must include `skill_mult`; unhoused citizens run at x0.7 from tick 1. `max_workers_per_workplace` and land-slot limits are not enforced yet (S0.6b: `FoundOrg`/`AddWorkplace` commands, machines install/uninstall/depreciation, the 9th Farm and 7th worker rejections).
make check: green · new tests: 9 · sim-check: n/a

## S0.6b — Orgs, land slots, machines, depreciation — 2026-09-12 — PR #9
Built: `orgs` module: `managed_org` (manager check, `on_behalf_of` must agree), `slot_for` (slot-limited kinds need a free slot, `NoSlotAvailable`), `check_room` (`max_workers_per_workplace`, `WorkplaceFull`; S0.10/S0.16 call it before `Assigned`), `found_org` (firms/coops/associations; collectives are seeded; destitute cannot found; fee burned (Q9); first workplace moves 20 Materials from the pantry to the org then `WorkplaceAdded`), `add_workplace` (manager, org Materials), `install_machines`/`uninstall_machines`, `appoint_manager` (controlling owner for firms, current manager for member orgs, System for society orgs), `controlling_owner` (> 50% of issued, ADR-0005), `cycle_end_8f_depreciation` (wear += machines x rate; whole units emitted as `MachinesDepreciated` with Explain; fraction carried). `apply` handles `ManagerAppointed`, `MachinesInstalled`/`Uninstalled`/`Depreciated`. New tunable `founding.initial_shares` = 100.
Deviations from TDD: none.
Provisional answers added to QUESTIONS.md: none.
New tunables: `initial_shares` (100).
Next session should know: the builder golden `s0_3b_builder` was regenerated because `SocietyCreated` embeds `Params` and a param was added; any param addition will do that until the S0.12 scenario golden replaces it as the meaningful one. S0.6 done gate complete (9th Farm and 7th worker rejected, machines and depreciation conserve).
make check: green · new tests: 7 · sim-check: n/a

## S0.7 — Money, transfers, escrow, direct sales — 2026-09-12 — PR #10
Built: `transfers` module: `acting_party` (citizen, or an org via `on_behalf_of` when the citizen manages it), `check_pantry_room` (per-good caps, Q12; orgs uncapped), `check_has`, `transfer` (money or goods to any citizen or org; self-deal rejected), `offer_sale` (escrows the seller's goods at offer time; shares and dwellings are `NotImplemented` until S0.10/S0.11), `accept_sale` (addressee check, price paid atomically, pantry caps both ways), `cancel_sale`, `post_wanted`/`remove_wanted`. `apply` handles `SaleOffered`/`Accepted`/`Cancelled`, `WantedPosted`/`Removed` with `EscrowKey::Offer` escrow. Strategies gained command-shaped `CmdStep`s run through `handle` with rejections ignored (`arb_cmd_scenario`, `run_cmd_scenario`).
Deviations from TDD: "escrow on both sides" is implemented as seller-side escrow at offer plus atomic payment at accept (a buyer who has not yet accepted has nothing to escrow).
Provisional answers added to QUESTIONS.md: none.
New tunables: none.
Next session should know: `Transfer` is now real, so the S0.4 gating test expects `SelfDeal` rather than `NotImplemented` for a self-transfer. `CmdStep` is the place to add order steps (S0.8 drafts already do).
make check: green · new tests: 5 (+1 proptest) · sim-check: n/a

## S0.8 — Order books — 2026-09-12 — PR #11
Built: `market` module: `OrderBook` per instrument with per-tick VWAP accumulators, `place_order` (escrow money for bids at `remaining x limit`, goods for asks; pantry cap counts open bids; share instruments `NotImplemented` until S0.10; default expiry = end of the next cycle), continuous matching on submission at the resting order's price with price-time priority and partial fills, `cancel_order`, `phase_6_markets` (expiry, VWAP per instrument, basket price index over priced goods, Q23), `last_price` (trade, else start price, Q8), `depth`. `apply` handles `OrderPlaced`/`Cancelled`/`Expired`/`Trade` (price-improvement refunds to the bidder, fully filled orders removed, accumulators; `TickResolved` resets them and sets `World.price_index`). `TickResolved` gained `vwap`. Strategies gained `PlaceOrder`/`CancelOrder` command steps. ADR-0001 written. Golden `s0_8_order_tape` (20 scripted orders) committed.
Deviations from TDD: none.
Provisional answers added to QUESTIONS.md: Q23 (index ignores Dwellings until they have prices).
New tunables: none.
Next session should know: the S0.7 conservation proptest now also cancels resting orders before asserting the escrow drains, because the shared `CmdStep` strategy places orders. `open_bid_qty` is `pub` for the plan executor (S0.9).
make check: green · new tests: 6 (+1 proptest) · sim-check: n/a

## S0.9 — Standing plan executor and dormancy — 2026-09-12 — PR #12
Built: `plan` module: `set_standing_plan` (validated against the constitution: labor plan kind, money rules, standing orders, vote defaults), `default_max_price` (last x 1.25 to the cent), `phase_2_standing_plans` for market systems (standing orders refreshed each tick or each cycle by cancel-and-replace, the Food shortfall bid at the limit within the saving floor and pantry room counting open bids, the Wares rule sized to refill Comfort, Q24), `cycle_end_8k_dormancy` (humans absent for 7 full cycles: open orders cancelled, then `CitizenDormant`; householders exempt), `away_digest`/`touches`. `Seen` from a dormant citizen emits `CitizenReturned`. `apply` suspends/reactivates the citizen's contracts on dormancy/return. Plan commands run through `handle` on the scratch world with `ClientKind::Plan` (Q6).
Deviations from TDD: none.
Provisional answers added to QUESTIONS.md: Q24 (Wares quantity and refresh semantics), Q25 (presence and the freeze).
New tunables: none.
Next session should know: every joined citizen carries the default plan (keep Food at 24), so any fixture with money and a resting Food ask will see plan bids; tests that need a quiet book set `keep_food_at_least = 0` via `PlanChanged`. Long-running fixtures without `Seen` go dormant at cycle 7; raise `dormancy_absent_cycles` in `with_preset` when that is not the point.
make check: green · new tests: 8 · sim-check: n/a

## S0.11b — Credit, defaults, associations — 2026-09-13 — PR #16
Built: `credit` module: `schedule` (simple interest, equal integer-cent installments, remainder on the last), `offer_credit` (lender escrows the principal; `CreditOffered` now carries `by`), `accept_credit` (addressee, self-deal, destitute long-term, collateral ownership; share collateral escrowed under the contract), `cycle_end_8c_credit_installments` (`CreditInstallment` with Explain, `CreditRepaid`, or `CreditMissed`), `phase_7_defaults` (missed installments default on the first tick of the next cycle: collateral to the lender, contract ended, borrower flagged), `pledged` (a pledged dwelling cannot be sold or let), and associations: `request_membership` (`MembershipRequested`, new event), `admit_member` (manager), `leave_org` (a leaving manager vacates the chair). `ContractBody::Credit.missed` added; `CreditAccepted` carries the rate.
Deviations from TDD: none.
Provisional answers added to QUESTIONS.md: Q35, Q36.
New tunables: none.
Next session should know: S0.11 done gate complete (the 100-credit, 5-cycle, 2% loan repays exactly 22 per cycle). S0.12a (seeding, fill, emigration) is drafted.
make check: green · new tests: 5 · sim-check: n/a

## S0.14b — Legacy inventory seeding (engine session pre-authorised by the plan) — 2026-09-13 — PR #21
Built: `params.seeding.legacy_inventory` (per workplace kind, goods seeded into each legacy org at `start_epoch`, counted in `seeded`; ADR-0004, Q45): Mills start with 320 Food and 120 Grain, Foundries with 60 Ore. Seeding test extended; goldens regenerated.
Deviations from TDD: none (ADR-0004 anticipated `legacy_inventory`).
Provisional answers added to QUESTIONS.md: Q45.
New tunables: `seeding.legacy_inventory`.
Next session should know: with this seed the householder Freeport passes every GDD 17 target at the GDD's own needs values (`food_meter_per_unit` stays 4); the Q20 trap is real but no longer triggered without humans. Seeding Materials into the Machine Shop pushed the investment-share metric to 0.66 (seeded Materials are consumed but never produced), so it is not seeded. S0.14 is the report and the green `make sim-check`.
make check: green · new tests: 0 (1 extended) · sim-check: red (investment share 0.65 in one epoch on seeds 2-5; this line was first written as green before the sweep finished, corrected in S0.14)

## S0.14 — Freeport tuning pass (config only) — 2026-09-13 — PR #22
Built: `docs/tuning/freeport-01.md` (the Phase 0a exit report); `householder.legacy_machine_buy_payroll_mult` 1.0 -> 1.5 (Q43), which removes the one-epoch-per-seed 0.65 investment share by staggering legacy machine buys; goldens regenerated for the new constant. `needs.food_meter_per_unit` stays at the GDD value of 4 (report 00 proposed 6; not needed once day one has stock).
Deviations from TDD: none.
Provisional answers added to QUESTIONS.md: Q46 (structural unemployment of 15-22 with 40 householders); Q20 and Q43 updated.
New tunables: none.
Next session should know: **Phase 0a exit gate met**: `make sim-check PRESET=freeport` is green on seeds 1-5 (need 99.4-100%, index 1.02-1.14, stock-out 2, investment share 0.27-0.29, 0 rejections; about 18 s wall clock). Q20 (sticky hardship) is still open and will bite the first human who runs out of money; decide it before Phase 1. Phase 0b (S0.15+, the other four presets' rule slices) can start from `main`.
make check: green · new tests: 0 · sim-check: green on seeds 1-5

## S1.0 — Q20 decision and Phase 1 dev infrastructure — 2026-09-13 — PR #23
Built: `needs.food_meter_per_unit` 4 -> 6 (Q20 decided by Chris: one Food per tick recovers +2 at normal effort, so a human at 0 climbs out of hardship within a cycle); the three goldens regenerated; the S0.5 table test pins the GDD value of 4 in its fixture so it still checks the mechanic it was written for. `deploy/docker-compose.dev.yml` (postgres:17 on host port 5433, `ISMS_DB_PORT` override), `.env.example`, `make db` / `make db-stop` / `make dev` (server and Vite pieces arrive with S1.2 and S1.7), CI `check` job gains a postgres:17 service with `DATABASE_URL` and `SQLX_OFFLINE=true`, and `scripts/pr-cycle.sh` now rebases on `origin/main` and re-runs `make check` before pushing (the two-track merge rule).
Deviations from TDD: none.
Provisional answers added to QUESTIONS.md: none new; Q20 moved to decided.
New tunables: none.
Next session should know: this is the first card of the Phase 1 track, run from the worktree `../Isms-p1` in parallel with Phase 0b in `../Isms-0b`. Engine-touching PRs from either track carry `[engine]` in the title. Port 5432 on this box belongs to another project's container, hence 5433. sim-check numbers are byte-identical to S0.14 (householders never sat below 20, so the +2 never fires for them).
make check: green · new tests: 0 (1 fixture pinned) · sim-check: green on seeds 1-5
