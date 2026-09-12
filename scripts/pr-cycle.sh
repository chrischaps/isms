#!/usr/bin/env bash
# Push the current branch, open a PR, wait for CI, squash-merge, and return to main.
# Usage: scripts/pr-cycle.sh "<title>" "<body-file>"
set -euo pipefail
title="$1"; body_file="$2"
branch="$(git branch --show-current)"
git push -q -u origin "$branch"
url="$(gh pr create --title "$title" --body-file "$body_file")"
num="${url##*/}"
echo "PR #$num: $url"
# CI takes a few seconds to register; poll until a check row exists, then watch.
for _ in $(seq 1 40); do
  if gh pr checks "$num" >/dev/null 2>&1; then break; fi
  sleep 5
done
gh pr checks "$num" --watch --interval 15 --fail-fast
gh pr merge "$num" --squash --delete-branch
git switch -q main && git pull -q
echo "merged #$num -> main $(git rev-parse --short HEAD)"
