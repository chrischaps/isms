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
bash scripts/pr-merge.sh "$num"
