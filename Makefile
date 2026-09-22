# Isms — session gate and dev targets (TDD §0, Appendix D).
# Recipes are plain cargo/pnpm invocations so the same file runs under Git Bash
# on Windows (GNU make 3.81 from GnuWin32) and under ubuntu in CI.

ifeq ($(OS),Windows_NT)
SHELL := bash
# From PowerShell or cmd, GnuWin32 make hands recipes to cmd.exe, where `bash` is
# WSL's, and every script target hangs with no output. Git Bash sets MSYSTEM.
ifndef MSYSTEM
$(error Run make from Git Bash (the isms-dev launcher opens one); from PowerShell the recipes go to WSL bash and hang)
endif
endif

PRESET ?= freeport
# The dev database from `make db`; a .env file or the environment overrides it (CI sets its own).
export DATABASE_URL ?= postgres://isms:isms@localhost:5433/isms
EPOCHS ?= 5
SEED   ?= 1

.PHONY: check push hooks fmt fmt-check clippy test web-check agents-check agents e2e-agents load db db-stop dev e2e e2e-web sim sim-check sim-all sqlx-prepare openapi-lint api-types

check: fmt-check clippy test web-check agents-check

# Non-engine work on main: check HEAD in ../Isms-check (only what its diff can break), push on green.
push:
	bash scripts/check-and-push.sh

# Once per clone: the pre-push hook that lets only a checked sha reach main.
hooks:
	git config core.hooksPath scripts/hooks

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

# The synthetic-player harness (S1.16): typecheck, lint, and its recorded-API tests (no model calls).
agents-check:
	pnpm --dir agents install --frozen-lockfile
	pnpm --dir agents check

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
	pnpm --dir agents api-types

# Playwright against a live server (S1.7).
e2e-web:
	bash scripts/e2e/web.sh

# One run with the per-citizen, per-org, flow, trade, move and depth CSVs under target/sim/<preset>-<seed>/ (S0.14e).
.PHONY: sim-detail
sim-detail:
	cargo run -p isms-sim --release -- run --preset $(PRESET) --epochs $(EPOCHS) --seed $(SEED) --out docs/tuning/runs --detail

# A synthetic cohort against a throwaway lab society (S1.16): report in docs/playtest/runs/<RUN>.md.
# BRAIN=scripted costs nothing; mixed and llm need ANTHROPIC_API_KEY in the environment or .env.
RUN ?= $(shell date +%Y%m%d-%H%M)
PLAYERS ?= 8
TICK_SECONDS ?= 10
EPOCH_CYCLES ?= 7
BRAIN ?= mixed
# PRESET=commune plays the Commune personas (S2.10).
PRESET ?= freeport
agents:
	set -a; [ -f .env ] && . ./.env; set +a; RUN=$(RUN) PRESET=$(PRESET) PLAYERS=$(PLAYERS) TICK_SECONDS=$(TICK_SECONDS) EPOCH_CYCLES=$(EPOCH_CYCLES) BRAIN=$(BRAIN) bash scripts/agents/run.sh

# The S1.15 load test (TDD 17): 100 householders and 20 scripted players at 5 s an hour for an epoch; report in docs/playtest/load/.
load:
	set -a; [ -f .env ] && . ./.env; set +a; bash scripts/load/run.sh

# The scripted cohort as a regression: two fast days, every probe refused, no 5xx, no turn in error (CI).
# Freeport, then the Commune (S2.10): two fast days each with eight scripted players, red on any new fuzzer defect.
e2e-agents:
	set -a; [ -f .env ] && . ./.env; set +a; RUN=e2e-$(shell date +%s)-freeport PRESET=freeport PLAYERS=8 TICK_SECONDS=1 EPOCH_CYCLES=2 BRAIN=scripted ASSERT_CLEAN=1 bash scripts/agents/run.sh
	set -a; [ -f .env ] && . ./.env; set +a; RUN=e2e-$(shell date +%s)-commune PRESET=commune PLAYERS=8 TICK_SECONDS=1 EPOCH_CYCLES=2 BRAIN=scripted ASSERT_CLEAN=1 bash scripts/agents/run.sh
