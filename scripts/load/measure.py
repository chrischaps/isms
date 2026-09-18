"""Fold a load run (scripts/load/run.sh) into one report against the TDD 17
budget: tick and command times from the server's JSON log, the harness
journals for the client's view of the same commands, the events table and
the latest snapshot from Postgres (through the dev container's psql), the
world's bytes from `rebuild`, and the RSS samples. Exit 1 on a broken
budget line when --assert 1.
"""

import argparse
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

# TDD 17: single society of 500 citizens on 2 vCPU / 4 GB.
BUDGET = {
    "tick_ms": 500,        # per tick, persistence included
    "command_p99_ms": 50,  # a command's p99
    "world_mb": 16,        # the World's canonical bytes
    "events_gb": 3,        # one epoch's events
    "rss_gb": 2,           # ten societies concurrent
}
TICKS_PER_CYCLE = 24
EPOCH_CYCLES = 42
# A tick that took longer than this was interrupted by the machine sleeping
# (Instant keeps counting on Windows); it is listed, not measured.
SLEEP_MS = 60_000


def percentile(xs, p):
    if not xs:
        return None
    s = sorted(xs)
    k = max(0, min(len(s) - 1, round(p / 100 * (len(s) - 1))))
    return s[k]


def dist(xs):
    if not xs:
        return "none"
    return f"p50 {percentile(xs, 50):.0f} ms, p90 {percentile(xs, 90):.0f} ms, p99 {percentile(xs, 99):.0f} ms, max {max(xs):.0f} ms over {len(xs)}"


def psql(db, sql):
    cmd = ["docker", "exec", "isms-dev-db", "psql", "-U", "isms", "-d", db, "-Atc", sql]
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout.strip()


def act_tools(agents_dir):
    src = (agents_dir / "src" / "tools" / "act.ts").read_text(encoding="utf-8")
    return set(re.findall(r"^export const (\w+) = act\(", src, re.M))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", required=True)
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--db", required=True)
    ap.add_argument("--society", type=int, required=True)
    ap.add_argument("--societies", type=int, default=1)
    ap.add_argument("--players", type=int, default=20)
    ap.add_argument("--householders", type=int, default=100)
    ap.add_argument("--tick-seconds", type=int, default=5)
    ap.add_argument("--epoch-cycles", type=int, default=42)
    ap.add_argument("--assert", dest="do_assert", default="1")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    run_dir = Path(a.run_dir)
    agents_dir = Path(__file__).resolve().parents[2] / "agents"

    # The server log: "tick resolved" and "command resolved" lines of the society under load.
    ticks, slept, cmds = [], [], Counter()
    cmd_ms, cmd_ms_no_seen = [], []
    for line in (run_dir / "server.jsonl").read_text(encoding="utf-8", errors="replace").splitlines():
        if '"tick resolved"' not in line and '"command resolved"' not in line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        f = rec.get("fields", {})
        if f.get("society") != a.society or "ms" not in f:
            continue
        ms = float(f["ms"])
        if f.get("message") == "tick resolved":
            (slept if ms > SLEEP_MS else ticks).append((f.get("tick"), ms, rec.get("timestamp")))
        else:
            cmds[f.get("kind")] += 1
            cmd_ms.append(ms)
            if f.get("kind") != "Seen":
                cmd_ms_no_seen.append(ms)
    tick_ms = [ms for _, ms, _ in ticks]

    # The client's view: every act tool call in every journal, HTTP included.
    # A call over a second went through the harness's one retry after a 429
    # (the per-citizen rate limit, TDD 10.2), so it measures the limiter, not the server.
    acts = act_tools(agents_dir)
    client_ms, client_read_ms = [], []
    for j in run_dir.glob("*.journal.jsonl"):
        for line in j.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            if rec.get("kind") != "turn":
                continue
            for c in rec.get("calls", []):
                if c.get("tool") == "end_turn":
                    continue
                (client_ms if c.get("tool") in acts else client_read_ms).append(float(c.get("ms", 0)))
    retried = sum(1 for ms in client_ms + client_read_ms if ms >= 1000)

    # The database: events by kind, the table, the latest snapshot.
    by_kind = []
    for row in psql(a.db, f"SELECT kind, count(*), sum(pg_column_size(payload)) FROM events WHERE society_id = {a.society} AND epoch = 0 GROUP BY kind ORDER BY 3 DESC").splitlines():
        kind, n, size = row.split("|")
        by_kind.append((kind, int(n), int(size)))
    payload_bytes = sum(s for _, _, s in by_kind)
    event_count = sum(n for _, n, _ in by_kind)
    table_bytes = int(psql(a.db, "SELECT pg_total_relation_size('events')"))
    all_events = int(psql(a.db, "SELECT count(*) FROM events"))
    ticks_done = int(psql(a.db, f"SELECT count(*) FROM events WHERE society_id = {a.society} AND epoch = 0 AND kind = 'TickResolved'"))
    snap = psql(a.db, f"SELECT octet_length(world), epoch, tick FROM snapshots WHERE society_id = {a.society} ORDER BY last_seq DESC LIMIT 1").split("|")
    snap_bytes = int(snap[0]) if snap and snap[0] else 0

    # The world's own bytes, logged by `rebuild` (colour codes stripped).
    world_bytes = None
    plain = re.sub(r"\x1b\[[0-9;]*m", "", (run_dir / "rebuild.log").read_text(encoding="utf-8", errors="replace"))
    m = re.search(r"world_bytes=(\d+)", plain)
    if m:
        world_bytes = int(m.group(1))

    # RSS samples, in KB.
    rss = []
    for line in (run_dir / "rss.tsv").read_text(encoding="utf-8", errors="replace").splitlines():
        parts = line.split("\t")
        if len(parts) == 2 and parts[1].strip().isdigit():
            rss.append(int(parts[1]))

    ticks_per_epoch = TICKS_PER_CYCLE * a.epoch_cycles
    table_per_tick = table_bytes / a.societies / max(1, ticks_done)
    events_per_epoch_gb = table_per_tick * TICKS_PER_CYCLE * EPOCH_CYCLES / 1e9
    rss_gb = max(rss) / 1e6 if rss else None
    tick_p99 = percentile(tick_ms, 99)
    cmd_p99 = percentile(cmd_ms, 99)

    lines = [
        ("tick, p99 (max) with persistence", f"{tick_p99:.0f} ms ({max(tick_ms):.0f} ms) over {len(tick_ms)} ticks" + (f"; {len(slept)} interrupted by sleep, listed below" if slept else "") if tick_ms else "no ticks logged", f"<= {BUDGET['tick_ms']} ms", bool(tick_ms) and tick_p99 <= BUDGET["tick_ms"]),
        ("command p99, actor side (validate, persist, apply)", f"{cmd_p99:.0f} ms over {len(cmd_ms)} commands ({len(cmd_ms_no_seen)} besides Seen: p99 {percentile(cmd_ms_no_seen, 99):.0f} ms)" if cmd_ms else "not logged (needs isms_server::actor=debug)", f"<= {BUDGET['command_p99_ms']} ms", bool(cmd_ms) and cmd_p99 <= BUDGET["command_p99_ms"]),
        ("World, canonical bytes", f"{world_bytes / 1e6:.2f} MB (snapshot {snap_bytes / 1e6:.2f} MB zstd)" if world_bytes else f"not logged (snapshot {snap_bytes / 1e6:.2f} MB zstd)", f"<= {BUDGET['world_mb']} MB", world_bytes is not None and world_bytes <= BUDGET["world_mb"] * 1e6),
        ("events per 42-day epoch, one society", f"{events_per_epoch_gb:.3f} GB from {table_bytes / 1e6:.1f} MB of table after {ticks_done} of {ticks_per_epoch} ticks ({table_per_tick / 1e3:.1f} kB per tick, {event_count} events)", f"<= {BUDGET['events_gb']} GB", events_per_epoch_gb <= BUDGET["events_gb"]),
        (f"RSS, max over the run ({a.societies} societies)", f"{rss_gb:.2f} GB over {len(rss)} samples" if rss else "not sampled", f"<= {BUDGET['rss_gb']} GB for ten societies", rss_gb is not None and rss_gb <= BUDGET["rss_gb"]),
    ]

    out = [
        f"# Load run {a.run}",
        "",
        f"S1.15 load test (TDD 17): {a.societies} lab Freeport(s), {a.householders} householders and {a.players} scripted players (the harness's brains, ADR-0010) at `tick_seconds={a.tick_seconds}`, {a.epoch_cycles}-day epoch; {ticks_done} ticks resolved; {all_events} events in the database.",
        "",
        "| budget line | measured | budget | |",
        "|---|---|---|---|",
    ]
    ok_all = True
    for name, measured, budget, ok in lines:
        ok = bool(ok)
        ok_all = ok_all and ok
        out.append(f"| {name} | {measured} | {budget} | {'pass' if ok else 'FAIL'} |")
    out += ["", "## Events by kind (the society under load)", "", "| kind | count | payload bytes | share |", "|---|---|---|---|"]
    for kind, n, size in by_kind[:12]:
        out.append(f"| {kind} | {n} | {size:,} | {100 * size / max(1, payload_bytes):.1f}% |")
    tr = next((s for k, n, s in by_kind if k == "TickResolved"), 0)
    trn = next((n for k, n, s in by_kind if k == "TickResolved"), 0)
    out += [
        "",
        f"`TickResolved` is {100 * tr / max(1, payload_bytes):.0f}% of the payload bytes, {tr / max(1, trn) / 1e3:.1f} kB each (T5: the JSONB question). The table with its index and TOAST is {table_bytes / max(1, payload_bytes):.2f}x the payload bytes.",
        "",
        "## Tick time",
        "",
        dist(tick_ms) + "." if tick_ms else "No ticks logged.",
    ]
    if slept:
        out += ["", "Interrupted by the machine sleeping (the clock kept counting; the scheduler caught up afterwards):", ""]
        for tick, ms, at in slept:
            out.append(f"- tick {tick}: {ms / 60_000:.0f} min, logged at {at}")
    out += [
        "",
        "## Command time",
        "",
        f"Actor side: {dist(cmd_ms)}; besides `Seen`: {dist(cmd_ms_no_seen)}. By kind: " + ", ".join(f"{k} {n}" for k, n in cmds.most_common(8)) + "." if cmd_ms else "Actor side: not logged.",
        "",
        f"Client side, HTTP included: commands {dist(client_ms)}; reads {dist(client_read_ms)}. {retried} calls took a second or more: the harness's one retry after a 429 from the per-citizen rate limit, which the scripted players hit when several act in the same tick.",
        "",
        "Measured on the developer's machine with the dev Postgres in Docker Desktop, not on the 2 vCPU / 4 GB VPS the budget names; `synchronous_commit` is at its default.",
        "",
    ]
    Path(a.out).write_text("\n".join(out), encoding="utf-8")
    print("\n".join(out))
    print(f"\nreport: {a.out}")
    if a.do_assert == "1" and not ok_all:
        sys.exit(1)


if __name__ == "__main__":
    main()
