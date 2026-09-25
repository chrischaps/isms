#!/usr/bin/env bash
# A synthetic cohort (S1.16) against a throwaway lab society: fresh database,
# seed a lab society of PRESET (freeport or commune, S2.10), serve it on a side
# port, run the players to the epoch's end, write the report, then replay the
# log with `rebuild` as the conservation check. Mirrors scripts/e2e/core-loop.sh.
#
#   RUN=<name> PRESET=commune PLAYERS=8 TICK_SECONDS=10 EPOCH_CYCLES=7 BRAIN=mixed bash scripts/agents/run.sh
#
# BRAIN=scripted needs no ANTHROPIC_API_KEY and is what CI runs (make e2e-agents, both presets).
# BRAIN=jev plays the [jev] personas on TypeSafe's Jev through OpenRouter (SJ.1); needs OPENROUTER_API_KEY.
# A town of players (SJ.3): CAST=freeport-town PLAYERS=16 puts sixteen players among the floor's other
# twenty-four householders (the engine fills to POPULATION_FLOOR, 40 by default, counting the humans);
# the persona list is config.toml's `personas_for.<CAST>`, cycled to PLAYERS.
# ASSERT_CLEAN=1 fails the script when the report has a fuzzer defect or a 5xx.
set -euo pipefail
cd "$(dirname "$0")/../.."
RUN="${RUN:-$(date +%Y%m%d-%H%M)}"
PRESET="${PRESET:-freeport}"
PLAYERS="${PLAYERS:-8}"
TICK_SECONDS="${TICK_SECONDS:-10}"
EPOCH_CYCLES="${EPOCH_CYCLES:-7}"
BRAIN="${BRAIN:-mixed}"
POPULATION_FLOOR="${POPULATION_FLOOR:-40}"
CAST="${CAST:-}"
PORT="${AGENTS_PORT:-18090}"
BASE_URL="${DATABASE_URL:-postgres://isms:isms@localhost:5433/isms}"
export DATABASE_URL="${BASE_URL%/*}/isms_agents_$(echo "$RUN" | tr -c 'A-Za-z0-9\n' '_')"
export ISMS_URL="http://127.0.0.1:$PORT"
# The harness narrates the run; the server stays quiet unless asked (a .env may say info).
export RUST_LOG="${AGENTS_RUST_LOG:-warn}"
# A separate target directory: the dev stack may hold target/debug/isms-server.exe open.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/check}"
SERVER="$CARGO_TARGET_DIR/debug/isms-server"
export ISMS_SERVER_BIN="$PWD/$SERVER"

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
# The run database does not exist yet; the committed .sqlx metadata stands in for it.
SQLX_OFFLINE=true cargo build -q -p isms-server
pnpm --dir agents install --frozen-lockfile --silent

# The population floor's householders fill the preset's dwellings before the players join (SJ.2, Q159):
# a lab seeds one dwelling per householder and one more per player, so renting a home is a choice the players actually have.
say "a fresh database and a lab $PRESET ($EPOCH_CYCLES-day epoch, $TICK_SECONDS s an hour, a floor of $POPULATION_FLOOR householders)"
RUST_LOG=warn "$SERVER" migrate
SID="$(RUST_LOG=warn "$SERVER" seed --preset "$PRESET" --class lab --name "lab-$RUN" --tick-seconds "$TICK_SECONDS" \
  --param "params.time.epoch_cycles=$EPOCH_CYCLES" \
  --param "params.time.closing_window_minutes=${CLOSING_WINDOW_MINUTES:-1}" \
  --param "params.initial_dwellings=${INITIAL_DWELLINGS:-$((POPULATION_FLOOR + PLAYERS))}" \
  --param "params.population.floor=$POPULATION_FLOOR" \
  --param params.population.collapse_enabled=false | tail -n 1)"
export ISMS_SOCIETY="$SID"
echo "society $SID in $DATABASE_URL"

"$SERVER" serve --bind "127.0.0.1:$PORT" --base-url "$ISMS_URL" &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT
poll "server up" 60 curl -fsS "$ISMS_URL/healthz"

say "the lab society is not public"
curl -fsS "$ISMS_URL/public/societies" | grep -q "\"id\":$SID," && fail "the lab society is listed on /public/societies"
[ "$(curl -s -o /dev/null -w '%{http_code}' "$ISMS_URL/public/s/$SID/stats")" = "404" ] || fail "/public/s/$SID/stats answered"

say "run the cohort: $PLAYERS players, brain $BRAIN${CAST:+, cast $CAST}"
# AGENTS_CMD=record records one player's every exchange for the replay test (agents/test/replay.test.ts).
pnpm --dir agents "${AGENTS_CMD:-play}" --run "$RUN" --preset "$PRESET" --players "$PLAYERS" --brain "$BRAIN" ${CAST:+--cast "$CAST"} ${AGENTS_ARGS:-}

say "the epoch's end: a closing statement, the archive, the rollover (S1.15)"
# The lab seed keeps the statements window to a minute, so the next epoch starts inside the run.
pnpm --dir agents rollover -- --run "$RUN" --wait "${ROLLOVER_WAIT:-180}" || fail "the epoch did not archive and roll over (see agents/runs/$RUN/rollover.json)"

say "replay the log (conservation and determinism)"
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
"$SERVER" rebuild --society "$SID"

REPORT="docs/playtest/runs/$RUN.md"
say "report: $REPORT"
if [ "${ASSERT_CLEAN:-0}" = "1" ]; then
  # A regression run's report stays with the run; only a real playtest's belongs in docs/.
  rm -f "$REPORT"; REPORT="agents/runs/$RUN/report.md"
  # Every accepted probe or server error must be a known, filed defect (agents/known-defects.txt).
  found="$(sed -n '/^## Fuzzer findings/,/^Refusals that name\|^## What players/p' "$REPORT" | grep '^| ' | grep -v '^| player'     | grep -v -F -f <(grep -v '^#' agents/known-defects.txt) || true)"
  [ -z "$found" ] || fail "the fuzzer found a defect not in agents/known-defects.txt (see $REPORT):
$found"
  ! grep -Eq '"status":5[0-9][0-9]' agents/runs/"$RUN"/*.journal.jsonl || fail "a 5xx in a journal (see agents/runs/$RUN)"
  ! grep -q '"ended_by":"error"' agents/runs/"$RUN"/*.journal.jsonl || fail "a turn ended in error (see agents/runs/$RUN)"
fi
say "PASS: run $RUN played society $SID"
