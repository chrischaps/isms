#!/usr/bin/env bash
# Merge an open PR once CI is green, surviving the other track landing first:
# on a merge conflict, rebase on main (auto-resolving the two append-only
# docs), push, and wait for CI again. The branch is deleted only after the
# merge is confirmed. Usage: scripts/pr-merge.sh <pr-number>
set -euo pipefail
num="$1"
branch="$(git branch --show-current)"
for attempt in 1 2 3 4; do
  # CI can take a minute to register on a fresh push.
  for _ in $(seq 1 60); do
    if gh pr checks "$num" 2>/dev/null | grep -qE "pending|pass|fail"; then break; fi
    sleep 5
  done
  gh pr checks "$num" --watch --interval 15 --fail-fast || true
  if gh pr checks "$num" 2>/dev/null | grep -q "fail"; then
    echo "CI failed on attempt $attempt" >&2
    exit 1
  fi
  if gh pr merge "$num" --squash 2>merge.err; then
    rm -f merge.err
    state="$(gh pr view "$num" --json state --jq .state)"
    if [ "$state" = "MERGED" ]; then
      git push -q origin --delete "$branch" || true
      git fetch -q origin
      echo "merged #$num -> main $(git rev-parse --short origin/main)"
      exit 0
    fi
    echo "merge returned but PR is $state" >&2
    exit 1
  fi
  if grep -qi "conflict" merge.err; then
    echo "main moved (attempt $attempt): rebasing" >&2
    rm -f merge.err
    git fetch -q origin
    if ! git rebase -q origin/main; then
      python scripts/rebase-resolve.py
      GIT_EDITOR=true git rebase --continue
    fi
    git push -q --force-with-lease origin "$branch"
    continue
  fi
  cat merge.err >&2
  rm -f merge.err
  exit 1
done
echo "gave up after 4 attempts" >&2
exit 1
