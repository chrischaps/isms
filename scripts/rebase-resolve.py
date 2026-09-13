"""Resolve the conflicts two parallel tracks produce on rebase.

docs/QUESTIONS.md: rows are numbered and both tracks append, so the branch's
rows are renumbered after the highest number already on main. Every `Q<n>`
reference in the branch's other changed files is rewritten to match.
docs/SESSIONS.md is union-merged by .gitattributes and never conflicts.
Anything else is a real conflict and is left for a human.

Usage (inside a stopped rebase): python scripts/rebase-resolve.py
"""

import io
import re
import subprocess
import sys


def sh(*args):
    return subprocess.run(args, capture_output=True, text=True, check=False).stdout


conflicted = sh("git", "diff", "--name-only", "--diff-filter=U").split()
if not conflicted:
    print("nothing to resolve")
    sys.exit(0)

renumbered = {}
for path in conflicted:
    if path.endswith((".postcard", ".jsonl")) and "/golden/" in path:
        # Both tracks regenerated a golden: take main's, then regenerate on top.
        sh("git", "checkout", "--ours", path)
        sh("git", "add", path)
        print(f"took main's {path}; run UPDATE_GOLDEN=1 cargo test -p isms-core golden and commit")
        continue
    if not path.endswith("docs/QUESTIONS.md"):
        print(f"cannot resolve {path} automatically", file=sys.stderr)
        sys.exit(1)
    text = io.open(path, encoding="utf-8").read()
    pattern = re.compile(r"<<<<<<< [^\n]*\n(.*?)\n=======\n(.*?)\n>>>>>>> [^\n]*\n", re.S)
    while True:
        m = pattern.search(text)
        if not m:
            break
        theirs, ours = m.group(1), m.group(2)
        taken = [int(x) for x in re.findall(r"^\| Q(\d+) \|", text[: m.start()] + theirs, re.M)]
        nxt = max(taken) + 1
        rows = []
        for line in ours.split("\n"):
            r = re.match(r"^\| Q(\d+) \|", line)
            if r:
                old = int(r.group(1))
                renumbered[old] = nxt
                line = re.sub(r"^\| Q\d+ \|", f"| Q{nxt} |", line, count=1)
                nxt += 1
            rows.append(line)
        text = text[: m.start()] + theirs + "\n" + "\n".join(rows) + "\n" + text[m.end() :]
    io.open(path, "w", encoding="utf-8", newline="\n").write(text)
    sh("git", "add", path)

if renumbered:
    # Rewrite references in the files this commit touches (docs and sources alike).
    changed = sh("git", "diff", "--name-only", "HEAD", "REBASE_HEAD").split()
    changed += sh("git", "diff", "--name-only", "--cached").split()
    for path in sorted(set(changed)):
        if path.endswith("QUESTIONS.md"):
            continue
        try:
            text = io.open(path, encoding="utf-8").read()
        except (OSError, UnicodeDecodeError):
            continue
        new = text
        for old, nxt in sorted(renumbered.items(), reverse=True):
            new = re.sub(rf"\bQ{old}\b", f"Q{nxt}", new)
        if new != text:
            io.open(path, "w", encoding="utf-8", newline="\n").write(new)
            sh("git", "add", path)
    print("renumbered: " + ", ".join(f"Q{o} -> Q{n}" for o, n in renumbered.items()))
print("resolved")
