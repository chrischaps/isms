# Sessions

Append-only log. One entry per session card (template: `docs/tdd.md` Appendix C).

## S0.1 — Repository scaffold — 2026-09-12 — PR #1
Built: Cargo workspace (edition 2024, resolver 3, pinned workspace deps, `unsafe_code = forbid`, clippy pedantic as warnings promoted to errors by `make check`), six empty crates, `web/` Vite 8 + React 19 + TS 6 placeholder with `pnpm check` (tsc, oxlint, vitest), Makefile, CI workflow, `presets/_base.toml` with every GDD App. A value and TDD App. A tunable plus the new ones from QUESTIONS/ADRs, five preset TOMLs, lexicon placeholders and `KEYS.txt`, Freeport welcome copy, `CLAUDE.md` (TDD App. D plus the libm and Windows notes), `docs/` layout with QUESTIONS.md pre-seeded (Q1–Q18).
Deviations from TDD: the Vite template now ships oxlint instead of ESLint; kept oxlint (same role, zero config). `make` on Windows is GNU make 3.81 from GnuWin32, run from Git Bash; recipes are plain cargo/pnpm calls. No ADR needed.
Provisional answers added to QUESTIONS.md: Q1–Q18 (from the pre-build TDD review); ADR-0003/0004/0005 are reserved for S0.3a/S0.4.
New tunables: `legacy_treasury_credits` (384), `legacy_hire_inventory_cycles_cap` (2), `legacy_offer_max_hours` (8), `legacy_offer_notice_cycles` (1), `living_cost_food` (24), `living_cost_wares` (4), `meter_start` (100), `food_full_output_meter` (50), per-good pantry caps, Republic `tax_rate`/`need_floor_food`, Commonwealth `capital_levy`.
Next session should know: toolchain pinned to 1.92.0; `_base.toml` keys are nested by topic (`params.labor.*`, `params.needs.*`, …) and money keys carry a `_credits` suffix that the S0.2 loader converts to cents. `presets/test/` is reserved for test-only fixtures. arm64 CI runners are not free on a private repo (affects S0.13b).
make check: green · new tests: 1 (web placeholder) · sim-check: n/a
