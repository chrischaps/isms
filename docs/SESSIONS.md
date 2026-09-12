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
