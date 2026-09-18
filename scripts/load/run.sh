#!/usr/bin/env bash
# The S1.15 load test (TDD 17): a lab Freeport of 100 householders and 20
# scripted players at tick_seconds=5 for an epoch, against the performance
# budget. Mirrors scripts/agents/run.sh (fresh database, side port, the
# harness's scripted brains as the agents) and adds the measurements:
# tick time from the server log, command latency from the journals, the
# events table and the snapshot from the database, RSS sampled as it runs.
#
#   bash scripts/load/run.sh                       # the full epoch (42 days at 5 s an hour: 84 min)
#   EPOCH_CYCLES=2 bash scripts/load/run.sh        # a smoke run
#   LOAD_SOCIETIES=10 EPOCH_CYCLES=1 bash scripts/load/run.sh   # ten societies for the RSS line
#
# The report lands in docs/playtest/load/<RUN>.md. ASSERT=1 (the default) fails
# the script on any budget line the run breaks.
set -euo pipefail
cd "$(dirname "$0")/../.."
RUN="${RUN:-load-$(date +%Y%m%d-%H%M)}"
PLAYERS="${PLAYERS:-20}"
HOUSEHOLDERS="${HOUSEHOLDERS:-100}"
TICK_SECONDS="${TICK_SECONDS:-5}"
EPOCH_CYCLES="${EPOCH_CYCLES:-42}"
LOAD_SOCIETIES="${LOAD_SOCIETIES:-1}"
PORT="${LOAD_PORT:-18095}"
BASE_URL="${DATABASE_URL:-postgres://isms:isms@localhost:5433/isms}"
DB="isms_$(echo "$RUN" | tr -c 'A-Za-z0-9\n' '_')"
export DATABASE_URL="${BASE_URL%/*}/$DB"
export ISMS_URL="http://127.0.0.1:$PORT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/check}"
SERVER="$CARGO_TARGET_DIR/debug/isms-server"
export ISMS_SERVER_BIN="$PWD/$SERVER"
OUT="agents/runs/$RUN"
mkdir -p "$OUT" docs/playtest/load

say() { printf '\n== %s\n' "$*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
poll() {
  local what="$1" secs="$2"; shift 2
  local i
  for ((i = 0; i < secs; i++)); do
    if "$@" >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  fail "$what did not happen within ${secs}s"
}

say "build the server and the harness"
SQLX_OFFLINE=true cargo build -q -p isms-server
pnpm --dir agents install --frozen-lockfile --silent

# The floor is filled with householders at seeding; the players join after it
# and step 8l trims the fill back to floor - humans at the first cycle end, so
# the society settles at HOUSEHOLDERS + PLAYERS citizens.
FLOOR=$((HOUSEHOLDERS + PLAYERS))
say "a fresh database ($DB) and $LOAD_SOCIETIES lab Freeport(s): floor $FLOOR, $EPOCH_CYCLES-day epoch, $TICK_SECONDS s an hour"
RUST_LOG=warn "$SERVER" migrate
SID=""
for ((n = 1; n <= LOAD_SOCIETIES; n++)); do
  id="$(RUST_LOG=warn "$SERVER" seed --preset freeport --class lab --name "load-$RUN-$n" --tick-seconds "$TICK_SECONDS" --seed "$n" \
    --param "params.time.epoch_cycles=$EPOCH_CYCLES" \
    --param "params.population.floor=$FLOOR" \
    --param "params.population.cap=$((FLOOR * 2))" \
    --param "params.time.closing_window_minutes=1" \
    --param params.population.collapse_enabled=false | tail -n 1)"
  [ -n "$SID" ] || SID="$id"
done
export ISMS_SOCIETY="$SID"
echo "players join society $SID"

say "serve, with the tick log in JSON and RSS sampled every 30 s"
SERVER_LOG="$OUT/server.jsonl"
RSS_LOG="$OUT/rss.tsv"
: > "$RSS_LOG"
ISMS_LOG_JSON=1 RUST_LOG="warn,isms_server::actor=debug" "$SERVER" serve --bind "127.0.0.1:$PORT" --base-url "$ISMS_URL" > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!
rss_of() {
  # Git Bash: the Windows pid is ps -W's WINPID column; elsewhere ps knows rss itself.
  if command -v tasklist >/dev/null 2>&1; then
    local winpid
    winpid="$(ps -W -p "$SERVER_PID" 2>/dev/null | awk 'NR==2{print $4}')"
    # In KB, like ps; MSYS_NO_PATHCONV keeps Git Bash from turning /FI into a path.
    MSYS_NO_PATHCONV=1 tasklist /FI "PID eq $winpid" /FO CSV /NH 2>/dev/null | awk -F'","' '{gsub(/[^0-9]/,"",$5); print $5}'
  else
    ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' '
  fi
}
(
  while kill -0 "$SERVER_PID" 2>/dev/null; do
    printf '%s\t%s\n' "$(date +%s)" "$(rss_of)" >> "$RSS_LOG"
    sleep 30
  done
) &
SAMPLER_PID=$!
trap 'kill "$SAMPLER_PID" "$SERVER_PID" 2>/dev/null || true' EXIT
poll "server up" 60 curl -fsS "$ISMS_URL/healthz"

say "run the cohort: $PLAYERS scripted players until the epoch ends"
pnpm --dir agents play --run "$RUN" --players "$PLAYERS" --brain scripted ${AGENTS_ARGS:-}

say "the archive and the rollover"
pnpm --dir agents rollover -- --run "$RUN" --wait 180 || echo "(no rollover within the wait; the measurements stand)"
# A load run's cohort report stays with the run; docs/playtest/load/ gets the measurements.
rm -f "docs/playtest/runs/$RUN.md"

say "stop the server; replay and measure the snapshot"
kill "$SAMPLER_PID" 2>/dev/null || true
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
REBUILD_LOG="$OUT/rebuild.log"
RUST_LOG=info "$SERVER" rebuild --society "$SID" > "$REBUILD_LOG" 2>&1

say "the report"
python scripts/load/measure.py \
  --run "$RUN" --run-dir "$OUT" --db "$DB" --society "$SID" --societies "$LOAD_SOCIETIES" \
  --players "$PLAYERS" --householders "$HOUSEHOLDERS" --tick-seconds "$TICK_SECONDS" \
  --epoch-cycles "$EPOCH_CYCLES" --assert "${ASSERT:-1}" \
  --out "docs/playtest/load/$RUN.md"
say "PASS: $RUN"
