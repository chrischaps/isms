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
SERVER=target/debug/isms-server

# The run's database does not exist yet, so the sqlx macros build from .sqlx.
SQLX_OFFLINE=true cargo build -q -p isms-server
"$SERVER" migrate
"$SERVER" seed --preset freeport --name "web-$(date +%s)-$RANDOM" --tick-seconds 1 --param params.population.collapse_enabled=false >/dev/null
# A second society for the operator test, which holds, ends and restarts a clock nobody else is using.
"$SERVER" seed --preset freeport --name "operator-$(date +%s)-$RANDOM" --tick-seconds 1 --param params.population.collapse_enabled=false >/dev/null
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
export ISMS_SESSION ISMS_SESSION_NEW ISMS_SESSION_WORK ISMS_SESSION_MARKET ISMS_SESSION_ORG ISMS_SESSION_HAND ISMS_SESSION_LEND ISMS_SESSION_BORROW ISMS_SESSION_CIVIC ISMS_SESSION_ADMIN
(cd web && pnpm exec playwright test "$@")
