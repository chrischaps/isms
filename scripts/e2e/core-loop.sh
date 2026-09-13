#!/usr/bin/env bash
# The GDD 9.1 core loop, end to end, against a throwaway society at
# tick_seconds=1 (TDD 15, S1.6): join -> work -> get paid -> buy Food ->
# found a firm -> hire -> the Chronicle mentions it.
# Needs DATABASE_URL (a dev or CI Postgres) and jq.
set -euo pipefail
cd "$(dirname "$0")/../.."
# A fresh database per run: earlier runs leave societies that would replay for minutes at startup.
BASE_URL="${DATABASE_URL:-postgres://isms:isms@localhost:5433/isms}"
export DATABASE_URL="${BASE_URL%/*}/isms_e2e_$(date +%s)_$RANDOM"
PORT="${E2E_PORT:-18080}"
export ISMS_URL="http://127.0.0.1:$PORT"
export ISMS_CONFIG="$(mktemp)"
export RUST_LOG="${RUST_LOG:-warn}"
SERVER=target/debug/isms-server
ISMS=target/debug/isms

say() { printf '\n== %s\n' "$*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
# poll "<description>" <seconds> <command...>: run the command until it exits 0
poll() {
  local what="$1" secs="$2"; shift 2
  local i
  for ((i = 0; i < secs; i++)); do
    if "$@" >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  fail "$what did not happen within ${secs}s"
}

cargo build -q -p isms-server -p isms-cli
"$SERVER" migrate   # creates the database when missing

say "seed a Freeport with 40 householders, one tick per second"
SID="$("$SERVER" seed --preset freeport --name "e2e-$(date +%s)-$RANDOM" --tick-seconds 1 \
  --param params.population.collapse_enabled=false)"
echo "society $SID"

"$SERVER" serve --bind "127.0.0.1:$PORT" --base-url "$ISMS_URL" &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT
poll "server up" 60 curl -fsS "$ISMS_URL/healthz"

say "sign in and join (step 1: Situation)"
TOKEN="$("$SERVER" session --email e2e@example.test)"
"$ISMS" login --session "$TOKEN" >/dev/null
"$ISMS" join "$SID" --handle marlow
KEY="$("$ISMS" key create --label e2e)"
"$ISMS" login --key "$KEY" >/dev/null           # the rest of the loop runs as an agent would
"$ISMS" me
"$ISMS" home

say "take a job from the notice board and set labor (step 2: Obligation)"
poll "legacy job offers" 30 bash -c "\"$ISMS\" board --json | jq -e '[.offers[] | select(.kind==\"employment\")] | length > 0'"
OFFER="$("$ISMS" board --json | jq -r '[.offers[] | select(.kind=="employment")][0].id')"
"$ISMS" accept "$OFFER"
WP="$("$ISMS" home --json | jq -r '.labor.employment[0].body.employment.workplace')"
"$ISMS" labor set --workplace "$WP" --hours 8 --effort normal

say "get paid at cycle end (step 3: Compensation)"
poll "a payslip" 90 bash -c "\"$ISMS\" payslips --json | jq -e '.payslips | length > 0'"
"$ISMS" payslips

say "buy Food on the book (step 4: Consumption)"
"$ISMS" order bid food 2 @ 4.00
poll "Food in the pantry" 30 bash -c "\"$ISMS\" home --json | jq -e '.household.pantry.food >= 1'"

say "buy Materials and found a firm (step 5: Surplus)"
"$ISMS" order bid materials 20 @ 12.00
poll "20 Materials in the pantry" 180 bash -c "\"$ISMS\" home --json | jq -e '(.household.pantry.materials // 0) >= 20'"
ORG="$("$ISMS" org found firm "Iron & Sons" --workplace mine --json | jq -r '.events[] | select(.kind=="OrgFounded") | .payload.OrgFounded.org')"
echo "founded org $ORG"
NEWWP="$("$ISMS" org list --json | jq -r --argjson o "$ORG" '.orgs[] | select(.id==$o) | .workplaces[0].id')"
JOB="$("$ISMS" org offer "$ORG" --workplace "$NEWWP" --hourly 8.00 --places 2 --json | jq -r '.events[] | select(.kind=="EmploymentOffered") | .payload.EmploymentOffered.offer')"

say "a second person takes the job; the Chronicle notices (step 6: Society)"
# Householders are all employed in the first cycles (Q46), so the hire is another human.
HAND_CONFIG="$(mktemp)"
HAND_TOKEN="$("$SERVER" session --email hand@example.test)"
ISMS_CONFIG="$HAND_CONFIG" "$ISMS" login --session "$HAND_TOKEN" >/dev/null
ISMS_CONFIG="$HAND_CONFIG" "$ISMS" join "$SID" --handle hand
ISMS_CONFIG="$HAND_CONFIG" "$ISMS" accept "$JOB"
poll "an employee" 30 bash -c "\"$ISMS\" org list --json | jq -e --argjson o $ORG '.orgs[] | select(.id==\$o) | .employees >= 1'"
poll "the founding in the Chronicle" 30 bash -c "\"$ISMS\" chronicle --json | jq -e '[.headlines[] | select(.text | test(\"founds Iron & Sons\"))] | length > 0'"
"$ISMS" chronicle
"$ISMS" say square "founded Iron & Sons; hiring"
"$ISMS" read square

say "PASS: the core loop ran end to end against society $SID"
