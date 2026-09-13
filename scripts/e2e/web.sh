#!/usr/bin/env bash
# Playwright against a live server (S1.7): seed a society, serve it, mint a
# browser session, run the browser tests through the Vite dev server's proxy.
set -euo pipefail
cd "$(dirname "$0")/../.."
export DATABASE_URL="${DATABASE_URL:-postgres://isms:isms@localhost:5433/isms}"
PORT="${E2E_PORT:-18080}"
export ISMS_API="http://127.0.0.1:$PORT"
export RUST_LOG="${RUST_LOG:-warn}"
SERVER=target/debug/isms-server

cargo build -q -p isms-server
"$SERVER" migrate
"$SERVER" seed --preset freeport --name "web-$(date +%s)-$RANDOM" --tick-seconds 3600 >/dev/null
"$SERVER" serve --bind "127.0.0.1:$PORT" --base-url "$ISMS_API" &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do curl -fsS "$ISMS_API/healthz" >/dev/null 2>&1 && break; sleep 1; done

ISMS_SESSION="$("$SERVER" session --email web@example.test)"
export ISMS_SESSION
(cd web && pnpm exec playwright test "$@")
