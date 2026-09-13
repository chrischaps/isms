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

.PHONY: check fmt fmt-check clippy test web-check db db-stop dev sim sim-check sim-all sqlx-prepare openapi-lint api-types

check: fmt-check clippy test web-check

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

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
	@echo "server: cargo run -p isms-server -- serve (S1.2); web: pnpm --dir web dev (S1.7)"

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

api-types:
	@echo "make api-types arrives with S1.7"; exit 1
