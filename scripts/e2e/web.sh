#!/usr/bin/env bash
# Playwright against a live server (S1.7): seed a society, serve it, mint a
# browser session, run the browser tests through the Vite dev server's proxy.
set -euo pipefail
cd "$(dirname "$0")/../.."
# A fresh database per run: earlier runs leave societies that would replay for minutes at startup.
BASE_URL="${DATABASE_URL:-postgres://isms:isms@localhost:5433/isms}"
export DATABASE_URL="${BASE_URL%/*}/isms_e2e_$(date +%s)_$RANDOM"
PORT="${E2E_PORT:-18080}"
export ISMS_API="http://127.0.0.1:$PORT"
export RUST_LOG="${RUST_LOG:-warn}"
# CARGO_TARGET_DIR lets a run build beside a dev server that holds target/debug/isms-server.exe open.
SERVER="${CARGO_TARGET_DIR:-target}/debug/isms-server"

# The run's database does not exist yet, so the sqlx macros build from .sqlx.
SQLX_OFFLINE=true cargo build -q -p isms-server
"$SERVER" migrate
"$SERVER" seed --preset freeport --name "web-$(date +%s)-$RANDOM" --tick-seconds 1 --param params.population.collapse_enabled=false >/dev/null
# A second society for the operator test, which holds, ends and restarts a clock nobody else is using.
"$SERVER" seed --preset freeport --name "operator-$(date +%s)-$RANDOM" --tick-seconds 1 --param params.population.collapse_enabled=false >/dev/null
# A lab Commune for the assembly test (S2.6): governance, offices and the Common Store; not public.
"$SERVER" seed --preset commune --class lab --name "commune-$(date +%s)-$RANDOM" --tick-seconds 1 --param params.population.collapse_enabled=false >/dev/null
# A second one for the Store and Ledger test (S2.7): the assembly test counts its electorate, so the two cannot share a society.
"$SERVER" seed --preset commune --class lab --name "ledger-$(date +%s)-$RANDOM" --tick-seconds 1 --param params.population.collapse_enabled=false >/dev/null
ISMS_OPERATORS="admin@example.test" "$SERVER" serve --bind "127.0.0.1:$PORT" --base-url "$ISMS_API" &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT
up=0
for _ in $(seq 1 60); do curl -fsS "$ISMS_API/healthz" >/dev/null 2>&1 && up=1 && break; sleep 1; done
[ "$up" = 1 ] || { echo "server did not come up" >&2; exit 1; }

ISMS_SESSION="$("$SERVER" session --email web@example.test)"
ISMS_SESSION_NEW="$("$SERVER" session --email new-$(date +%s)@example.test)"
ISMS_SESSION_WORK="$("$SERVER" session --email work-$(date +%s)@example.test)"
ISMS_SESSION_MARKET="$("$SERVER" session --email market-$(date +%s)@example.test)"
ISMS_SESSION_ORG="$("$SERVER" session --email org-$(date +%s)@example.test)"
ISMS_SESSION_HAND="$("$SERVER" session --email hand-$(date +%s)@example.test)"
ISMS_SESSION_LEND="$("$SERVER" session --email lend-$(date +%s)@example.test)"
ISMS_SESSION_BORROW="$("$SERVER" session --email borrow-$(date +%s)@example.test)"
ISMS_SESSION_CIVIC="$("$SERVER" session --email civic-$(date +%s)@example.test)"
ISMS_SESSION_ADMIN="$("$SERVER" session --email admin@example.test)"
ISMS_SESSION_ASSEMBLY="$("$SERVER" session --email assembly-$(date +%s)@example.test)"
ISMS_SESSION_COMMUNE="$("$SERVER" session --email commune-$(date +%s)@example.test)"
export ISMS_SESSION ISMS_SESSION_NEW ISMS_SESSION_WORK ISMS_SESSION_MARKET ISMS_SESSION_ORG ISMS_SESSION_HAND ISMS_SESSION_LEND ISMS_SESSION_BORROW ISMS_SESSION_CIVIC ISMS_SESSION_ADMIN ISMS_SESSION_ASSEMBLY ISMS_SESSION_COMMUNE
(cd web && pnpm exec playwright test "$@")
