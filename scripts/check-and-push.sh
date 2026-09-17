#!/usr/bin/env bash
# Check-then-push for non-engine work committed straight to main (CLAUDE.md, Workflow).
#
#   bash scripts/check-and-push.sh [--no-push]      (or: make push)
#
# Takes the commit at HEAD, checks it out in a sibling worktree (../Isms-check) and runs
# only the half of `make check` its diff against origin/main can break. The working tree
# you called it from is never touched, so the next task can start while this runs.
# Green: the sha is recorded in .git/checked-sha and pushed to main; scripts/hooks/pre-push
# lets no other sha through. Red: nothing is pushed and the log path is printed.
# Engine changes (isms-core, isms-sim, presets) are refused: they go through a PR.
set -euo pipefail

push=1
[ "${1:-}" = "--no-push" ] && push=0

main_wt=$(git rev-parse --show-toplevel)
common=$(cd "$(git rev-parse --git-common-dir)" && pwd)
check_wt="${ISMS_CHECK_WT:-$(dirname "$main_wt")/Isms-check}"
target="${ISMS_CHECK_TARGET:-$main_wt/target/check}"
log="$common/check.log"
lock="$common/check.lock"

# GNU make on Windows lives outside Git Bash's PATH.
[ -d "/c/Program Files (x86)/GnuWin32/bin" ] && PATH="/c/Program Files (x86)/GnuWin32/bin:$PATH"
command -v cygpath >/dev/null && target=$(cygpath -m "$target")

if [ "$(git symbolic-ref --short -q HEAD)" != "main" ]; then
  echo "check-and-push: not on main; a branch goes through a PR (scripts/pr-cycle.sh)" >&2
  exit 1
fi

sha=$(git rev-parse HEAD)
git fetch -q origin main
base=$(git rev-parse origin/main)
if [ "$base" = "$sha" ]; then
  echo "check-and-push: nothing to push; HEAD is origin/main"
  exit 0
fi
if ! git merge-base --is-ancestor "$base" "$sha"; then
  echo "check-and-push: origin/main has moved; git pull --rebase first" >&2
  exit 1
fi
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "check-and-push: note: uncommitted changes are not part of this check"
fi

# What the diff can break decides what runs. Anything unrecognised gets the full check.
engine=0 rust=0 web=0
while IFS= read -r f; do
  case "$f" in
    crates/isms-core/* | crates/isms-sim/* | presets/*) engine=1 ;;
    docs/* | *.md) ;;
    web/*) web=1 ;;
    crates/isms-api-types/*) rust=1 web=1 ;;
    crates/* | Cargo.toml | Cargo.lock | .sqlx/* | rust-toolchain*) rust=1 ;;
    *) rust=1 web=1 ;;
  esac
done < <(git diff --name-only "$base" "$sha")

if [ "$engine" = 1 ]; then
  echo "check-and-push: $(git rev-parse --short "$base")..$(git rev-parse --short "$sha") touches the engine (isms-core, isms-sim, presets)." >&2
  echo "Engine changes reach main through a PR with green CI: move them to a branch and use scripts/pr-cycle.sh." >&2
  exit 1
fi

targets=""
[ "$rust" = 1 ] && targets="fmt-check clippy test"
[ "$web" = 1 ] && targets="$targets web-check"

if ! mkdir "$lock" 2>/dev/null; then
  echo "check-and-push: a check is already running for $(cat "$lock/sha" 2>/dev/null || echo '?')." >&2
  echo "Rerun when it finishes (a crashed run leaves $lock behind; remove it by hand)." >&2
  exit 1
fi
trap 'rm -rf "$lock"' EXIT
echo "$sha" >"$lock/sha"

short=$(git rev-parse --short "$sha")
if [ -n "$targets" ]; then
  if [ -d "$check_wt" ]; then
    git -C "$check_wt" checkout -q -f --detach "$sha"
    git -C "$check_wt" clean -qfd
  else
    git worktree add -q --detach "$check_wt" "$sha"
  fi
  [ -f "$main_wt/.env" ] && cp "$main_wt/.env" "$check_wt/.env"

  echo "check-and-push: $short: make$targets  (in $check_wt, log $log)"
  if ! CARGO_TARGET_DIR="$target" make -C "$check_wt" $targets >"$log" 2>&1; then
    echo "check-and-push: $short is RED after ${SECONDS}s; nothing pushed. Last lines of $log:" >&2
    tail -n 25 "$log" >&2
    exit 1
  fi
  echo "check-and-push: $short is green after ${SECONDS}s"
else
  echo "check-and-push: $short changes only docs; nothing to run"
fi

echo "$sha" >"$common/checked-sha"
if [ "$push" = 1 ]; then
  git push origin "$sha:refs/heads/main"
else
  echo "check-and-push: --no-push; $short is cleared for a later push"
fi
