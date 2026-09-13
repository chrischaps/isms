#!/usr/bin/env bash
# Rebase on main, run make check, push, open a PR, wait for CI, squash-merge, return to main.
# Usage: scripts/pr-cycle.sh "<title>" "<body-file>"   (SKIP_CHECK=1 to skip the local gate)
set -euo pipefail
title="$1"; body_file="$2"
branch="$(git branch --show-current)"
# Merge rule (two tracks share main): rebase on main, then re-run the gate on the rebased tree.
git fetch -q origin
if ! git rebase -q origin/main; then
  echo "rebase on origin/main has conflicts; resolve, then re-run" >&2; exit 1
fi
if [ "${SKIP_CHECK:-}" != "1" ]; then
  export PATH="/c/Program Files (x86)/GnuWin32/bin:$PATH"
  make check
fi
git push -q -u --force-with-lease origin "$branch"
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
# In a secondary worktree main is checked out elsewhere; just refresh the ref.
if git switch -q main 2>/dev/null; then
  git pull -q
else
  git fetch -q origin main:main 2>/dev/null || git fetch -q origin main
fi
echo "merged #$num -> main $(git rev-parse --short origin/main)"
