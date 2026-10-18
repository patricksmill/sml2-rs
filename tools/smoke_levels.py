#!/usr/bin/env python3
"""Boot every level headlessly and fail on any port diagnostic.

This turns "all 126 declared handlers are native" into "every level boots and
runs": it validates the 32 extracted manifests/assets, then runs the runtime's
own play loop with `--headless` (no window, no rendering, deterministic virtual
divider clock) for a fixed frame budget, twice per level — idle and holding
right, so the spawn stream, contact, defeat and death paths actually run.

Fails on: a missing/mismatched manifest record or asset, a nonzero exit, a
panic, a `note:` line (unported bank-16 handler, unported scene opcode, scene
with no script cursor, missing edge/scene table), a non-empty `unported=[...]`
field in the run summary, or a `NOT APPLIED` / `NOT RELOADED` scene tileset
swap.

Usage:
    python3 tools/smoke_levels.py                 # all 32, idle + right
    python3 tools/smoke_levels.py --levels 12,19,28,29
    python3 tools/smoke_levels.py --frames 900 --json smoke.json
    python3 tools/smoke_levels.py --passes idle

Exit status: 0 every level booted and ran clean; 1 a level failed; 2 inputs or
the runtime binary are missing.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEVELS = ROOT / "extracted" / "levels"
DEFAULT_BIN = ROOT / "target" / "debug" / "sml2-runtime"

SUMMARY_RE = re.compile(r"^headless end at frame (\d+): (.*)$")
UNPORTED_RE = re.compile(r"unported=\[([^\]]*)\]")
FAIL_MARKERS = ("note:", "NOT APPLIED", "NOT RELOADED", "panicked at")


def fail(msg: str) -> None:
    print(f"smoke_levels: {msg}", file=sys.stderr)
    raise SystemExit(2)


def require(cond: bool, msg: str, problems: list[str]) -> None:
    if not cond:
        problems.append(msg)


def check_inputs() -> list[str]:
    """Structural checks on the extracted level set; returns problems."""
    problems: list[str] = []
    index_path = LEVELS / "index.json"
    if not index_path.is_file():
        fail(f"missing {index_path}")
    index = json.loads(index_path.read_text())
    entries = {e.get("level_id"): e for e in index.get("levels", [])}
    require(len(entries) == 32, f"index.json lists {len(entries)} levels, expected 32", problems)
    for n in range(1, 33):
        lid = n - 1
        man = LEVELS / f"level{n:02}.manifest.json"
        if not man.is_file():
            problems.append(f"level{n:02}: manifest missing")
            continue
        m = json.loads(man.read_text())
        require(m.get("schema", "").startswith("sml2-level-manifest"), f"level{n:02}: bad schema", problems)
        require(m.get("level_id") == lid, f"level{n:02}: manifest level_id {m.get('level_id')} != {lid}", problems)
        e = entries.get(lid)
        if e is None:
            problems.append(f"level{n:02}: no index.json record")
        else:
            recs = m.get("spawns", {}).get("records", [])
            require(
                e.get("spawn_count") == len(recs),
                f"level{n:02}: index spawn_count {e.get('spawn_count')} != {len(recs)} records",
                problems,
            )
            require(
                e.get("layout_file") == m.get("layout", {}).get("file"),
                f"level{n:02}: index layout file disagrees with manifest",
                problems,
            )
        require(bool(m.get("spawns", {}).get("records")), f"level{n:02}: no spawn records", problems)
        layout = m.get("layout", {}).get("file")
        require(bool(layout) and (LEVELS / str(layout)).is_file(), f"level{n:02}: layout file missing", problems)
        bg = LEVELS / f"level{n:02}.bgblock.bin"
        require(bg.is_file() and bg.stat().st_size == 4096, f"level{n:02}: bgblock.bin not 4096 bytes", problems)
        ob = LEVELS / f"level{n:02}.objblock.bin"
        require(ob.is_file() and ob.stat().st_size == 4096, f"level{n:02}: objblock.bin not 4096 bytes", problems)
        for suffix, size in (("edge", 48), ("scene", 48)):
            p = LEVELS / f"level{n:02}.{suffix}.bin"
            require(p.is_file() and p.stat().st_size == size, f"level{n:02}: {suffix}.bin not {size} bytes", problems)
    return problems


def run_pass(binary: Path, level: int, frames: int, tape: str | None) -> dict:
    cmd = [str(binary), "--level", str(level), "--headless", str(frames)]
    if tape:
        cmd += ["--keys", tape]
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=300)
    out = proc.stdout
    err = proc.stderr
    summary = None
    for line in out.splitlines():
        m = SUMMARY_RE.match(line.strip())
        if m:
            summary = {"frame": int(m.group(1)), "state": m.group(2)}
    unported: list[str] = []
    if summary:
        u = UNPORTED_RE.search(summary["state"])
        if u and u.group(1).strip():
            unported = [x.strip() for x in u.group(1).split(",")]
    hits = [ln.strip() for ln in out.splitlines() if any(k in ln for k in FAIL_MARKERS)]
    hits += [ln.strip() for ln in err.splitlines() if any(k in ln for k in FAIL_MARKERS)]
    return {
        "exit": proc.returncode,
        "summary": summary,
        "unported": unported,
        "markers": hits,
        "stdout_tail": out.splitlines()[-3:],
        "stderr_tail": err.splitlines()[-3:],
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", type=Path, default=DEFAULT_BIN)
    ap.add_argument("--levels", default="0..31", help="e.g. 0..31, 12,19,28,29")
    ap.add_argument("--frames", type=int, default=600, help="idle frames (walk pass is 2x)")
    ap.add_argument("--passes", choices=("both", "idle", "walk"), default="both")
    ap.add_argument("--json", type=Path)
    args = ap.parse_args()

    problems = check_inputs()
    if problems:
        for p in problems:
            print(f"input: {p}")
        print(f"smoke_levels: {len(problems)} input problem(s)", file=sys.stderr)
        return 2
    binary = args.bin if args.bin.is_absolute() else ROOT / args.bin
    if not binary.is_file():
        fail(f"runtime binary {binary} not found (cargo build -p sml2-runtime)")

    if ".." in args.levels:
        a, b = args.levels.split("..")
        levels = list(range(int(a), int(b) + 1))
    else:
        levels = [int(x) for x in args.levels.split(",") if x.strip()]

    report = {"schema": "sml2-smoke-levels/1", "frames": args.frames, "levels": []}
    failed = 0
    print("level idle walk  mario            cam              obj  result")
    for level in levels:
        passes = []
        if args.passes in ("both", "idle"):
            passes.append(("idle", None))
        if args.passes in ("both", "walk"):
            # Hold right and jump every ninth frame: walks the level, clears
            # small steps, and pulls the camera far enough to run the spawn
            # stream instead of idling at the entrance.
            unit = "R" * 8 + "A"
            passes.append(("walk", (unit * (args.frames * 2 // len(unit) + 1))[: args.frames * 2]))
        row = {"level": level, "passes": []}
        reasons: list[str] = []
        for name, tape in passes:
            frames = args.frames if tape is None else args.frames * 2
            r = run_pass(binary, level, frames, tape)
            r["name"] = name
            r["frames"] = frames
            row["passes"].append(r)
            if r["exit"] != 0:
                reasons.append(f"{name}: exit {r['exit']}")
            if r["summary"] is None:
                reasons.append(f"{name}: no summary line")
            if r["unported"]:
                reasons.append(f"{name}: unported {r['unported']}")
            if r["markers"]:
                reasons.append(f"{name}: {'; '.join(r['markers'][:2])}")
        row["ok"] = not reasons
        row["reasons"] = reasons
        report["levels"].append(row)
        if not row["ok"]:
            failed += 1
        s = row["passes"][0]["summary"]
        if s:
            m = re.search(r"mario=\(\$?([0-9A-Fx]+),\$?([0-9A-Fx]+)\)", s["state"])
            c = re.search(r"camera=\(\$?([0-9A-Fx]+),\$?([0-9A-Fx]+)\)", s["state"])
            pos = f"({m.group(1)},{m.group(2)})" if m else "?"
            cam = f"({c.group(1)},{c.group(2)})" if c else "?"
        else:
            pos = cam = "?"
        last = row["passes"][-1]["summary"]
        objs = "-"
        if last:
            o = re.search(r"objects=(\d+)", last["state"])
            objs = o.group(1) if o else "-"
        print(f"{level:>5} {'ok' if row['passes'][0]['exit'] == 0 else 'FAIL':>4} "
              f"{'ok' if len(row['passes']) < 2 or row['passes'][1]['exit'] == 0 else 'FAIL':>5}  "
              f"{pos:<16} {cam:<16} {objs:>3}  {'ok' if row['ok'] else 'FAIL: ' + '; '.join(reasons[:2])}")
    if args.json:
        args.json.write_text(json.dumps(report, indent=1) + "\n")
    print()
    if failed:
        print(f"smoke_levels: {failed}/{len(levels)} level(s) failed", file=sys.stderr)
        return 1
    print(f"smoke_levels: {len(levels)} level(s) booted and ran clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
