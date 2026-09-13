# Isms — session gate and dev targets (TDD §0, Appendix D).
# Recipes are plain cargo/pnpm invocations so the same file runs under Git Bash
# on Windows (GNU make 3.81 from GnuWin32) and under ubuntu in CI.

ifeq ($(OS),Windows_NT)
SHELL := bash
endif

PRESET ?= freeport
# The dev database from `make db`; a .env file or the environment overrides it (CI sets its own).
export DATABASE_URL ?= postgres://isms:isms@localhost:5433/isms
EPOCHS ?= 5
SEED   ?= 1

.PHONY: check fmt fmt-check clippy test web-check db db-stop dev e2e e2e-web sim sim-check sim-all sqlx-prepare openapi-lint api-types

check: fmt-check clippy test web-check

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

# Offline like CI, so a query without cached metadata fails here first (run `make sqlx-prepare`).
clippy:
	SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings

# .env (git-ignored) supplies DATABASE_URL for the store and server tests (sqlx::test).
test:
	set -a; [ -f .env ] && . ./.env; set +a; cargo test --workspace

# Refresh the committed sqlx offline metadata (.sqlx/) after changing any query!/query_as!.
sqlx-prepare:
	set -a; [ -f .env ] && . ./.env; set +a; cargo sqlx prepare --workspace -- --all-targets

web-check:
	pnpm --dir web install --frozen-lockfile
	pnpm --dir web check

# Local Postgres for the store, the server, and sqlx::test (deploy/docker-compose.dev.yml).
db:
	docker compose -f deploy/docker-compose.dev.yml up -d --wait

db-stop:
	docker compose -f deploy/docker-compose.dev.yml down

# Postgres + server (tick_seconds=10) + Vite. Server and Vite arrive with S1.2 and S1.7.
dev: db
	@echo "API on :8080; run 'pnpm --dir web dev' in another shell for the web client on :5173"
	cargo run -p isms-server -- serve

# The GDD 9.1 core loop against a throwaway society at tick_seconds=1 (S1.6).
e2e:
	bash scripts/e2e/core-loop.sh

sim:
	cargo run -p isms-sim --release -- run --preset $(PRESET) --epochs $(EPOCHS) --seed $(SEED) --out docs/tuning/runs

sim-check:
	cargo test -p isms-sim --release -- --ignored --nocapture stability_$(PRESET)

# All five presets over seeds 1..5, tables printed in turn; --check makes a miss exit 1 (S0.18 gate).
sim-all:
	cargo run -p isms-sim --release -- all --epochs $(EPOCHS) --seeds 1..5 --out docs/tuning/runs --check

# Dump the OpenAPI document from the binary (no database needed) and lint it.
openapi-lint:
	cargo run -q -p isms-server -- openapi > target/openapi.json
	pnpm --package=@redocly/cli@1 dlx redocly lint target/openapi.json

# Regenerate the TypeScript client types from the OpenAPI document (S1.7).
api-types:
	cargo run -q -p isms-server -- openapi > target/openapi.json
	pnpm --dir web api-types

# Playwright against a live server (S1.7).
e2e-web:
	bash scripts/e2e/web.sh

# One run with the per-citizen, per-org, flow, trade, move and depth CSVs under target/sim/<preset>-<seed>/ (S0.14e).
.PHONY: sim-detail
sim-detail:
	cargo run -p isms-sim --release -- run --preset $(PRESET) --epochs $(EPOCHS) --seed $(SEED) --out docs/tuning/runs --detail
