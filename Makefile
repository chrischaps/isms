# Isms — session gate and dev targets (TDD §0, Appendix D).
# Recipes are plain cargo/pnpm invocations so the same file runs under Git Bash
# on Windows (GNU make 3.81 from GnuWin32) and under ubuntu in CI.

ifeq ($(OS),Windows_NT)
SHELL := bash
endif

PRESET ?= freeport
EPOCHS ?= 5
SEED   ?= 1

.PHONY: check fmt fmt-check clippy test web-check dev sim sim-check api-types

check: fmt-check clippy test web-check

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

web-check:
	pnpm --dir web install --frozen-lockfile
	pnpm --dir web check

dev:
	@echo "make dev arrives with S1.2 (Postgres + server + Vite)"; exit 1

sim:
	cargo run -p isms-sim --release -- run --preset $(PRESET) --epochs $(EPOCHS) --seed $(SEED) --out docs/tuning/runs

sim-check:
	cargo test -p isms-sim --release -- --ignored --nocapture stability_$(PRESET)

api-types:
	@echo "make api-types arrives with S1.7"; exit 1
