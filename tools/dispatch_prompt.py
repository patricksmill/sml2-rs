#!/usr/bin/env python3
"""Print a Porter user message for Hermes / Cursor subagents.

  python3 tools/dispatch_prompt.py --addr 0x58201
  python3 tools/dispatch_prompt.py --queue camera_pan_timer
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

QUEUE_FILES = {
    "moon_physics_remaining_arms": ["crates/sml2-core/src/player_sim.rs"],
    "map_zone_screen_loaders": ["crates/sml2-core/src/map.rs", "crates/sml2-core/src/map_sim.rs"],
    "camera_pan_timer": ["crates/sml2-core/src/camera.rs", "crates/sml2-runtime/src/main.rs"],
    "boss_arena_phase_cascade": [
        "crates/sml2-core/src/objects/boss.rs",
        "crates/sml2-core/src/scene.rs",
    ],
    "boss_clear_ending_handoff": ["crates/sml2-core/src/scene.rs", "crates/sml2-core/src/player_sim.rs"],
    "boss_anim_4_5_chain": ["crates/sml2-core/src/objects/boss.rs", "crates/sml2-core/src/objects/dispatch.rs", "crates/sml2-core/src/objects/tests.rs"],
    "remaining_handler_classes": [],  # use --addr instead
}


def run_handler_task(addr: int, *, dependencies=(), budget=800) -> None:
    command = [sys.executable, str(ROOT / "tools/handler_task.py"),
               "--addr", hex(addr), "--max-states", str(budget)]
    for dependency in dependencies:
        command.extend(["--dependency", dependency])
    # Always regenerate: a cached brief may refer to stale source or the old slicer.
    subprocess.run(command, check=True, cwd=ROOT)


def handler_prompt(addr: int, *, module: str | None = None, dependencies=(), budget=800) -> str:
    module = module or f"crates/sml2-core/src/objects/handler_0x{addr:05X}.rs"
    path = (ROOT / module).resolve()
    folder = (ROOT / "crates/sml2-core/src/objects").resolve()
    if path.parent != folder or path.suffix != '.rs' or path.stem in {'mod', 'dispatch', 'staging', 'helpers', 'tests'}:
        raise ValueError('choose a handler sibling module under crates/sml2-core/src/objects/')
    if path.exists() and len(path.read_text().splitlines()) >= 400:
        raise ValueError(f'{module} is already at least 400 lines; choose a new focused sibling module')
    run_handler_task(addr, dependencies=dependencies, budget=budget)
    task = ROOT / "tasks/handlers" / f"0x{addr:05X}.md"
    if '**Slice status: COMPLETE**' not in task.read_text():
        raise ValueError('incomplete brief; resolve dependencies before dispatch')
    return f"""Port object handler ROM 0x{addr:05X}.

Read ONLY:
  - {task.relative_to(ROOT)}
  - {module} (create if absent)
  - crates/sml2-core/src/objects/staging.rs
  - crates/sml2-core/src/objects/helpers.rs

Touch ONLY:
  - {module} (implementation and local tests)
  - crates/sml2-core/src/objects/dispatch.rs (one arm)
  - crates/sml2-core/src/objects/mod.rs (module registration only, if new)

Read the supplied brief's exact existing helper contracts when listed.
Do NOT read: DESIGN.md, PORT_QUEUE, captures/, notes/, whole main.asm.
Rules: PORTING.md. Keep a handler module below approximately 400 lines;
choose a new focused sibling module when the existing family exceeds that size.

Deliverables:
  1. Implementation + one dispatch arm for 0x{addr:05X}
  2. Local #[test] with slot/staging byte assertions, including boundary and gate cases
  3. Targeted new tests, followed by cargo test -p sml2-core

Reply ESCALATE after 2 failed fix attempts.
"""


def queue_prompt(item_id: str) -> str:
    live = json.loads((ROOT / "PORT_QUEUE.json").read_text())
    item = next((e for e in live["queue"] if e["id"] == item_id), None)
    if item is None:
        raise SystemExit(f"unknown or non-pending queue id: {item_id}")
    files = QUEUE_FILES.get(item_id, [])
    if item_id == "remaining_handler_classes":
        raise SystemExit("use --addr for handlers, not --queue remaining_handler_classes")
    if not files:
        raise ValueError(f'no bounded file list configured for queue item: {item_id}')
    if any("/objects/" in p for p in files) and "crates/sml2-core/src/objects/tests.rs" not in files:
        files = [*files, "crates/sml2-core/src/objects/tests.rs"]
    for file in files:
        if not (ROOT / file).is_file():
            raise ValueError(f"missing task file: {file}")
    file_block = "\n".join(f"  - {p}" for p in files) if files else "  - (supervisor must list files)"
    refs = ", ".join(item.get("asm_refs", []))
    return f"""Close PORT_QUEUE item: {item_id}
Title: {item['title']}
asm_refs: {refs}

Touch ONLY:
{file_block}

Rules: PORTING.md — unit tests, no new captures.
Run: cargo test -p sml2-core
"""


def main() -> None:
    ap = argparse.ArgumentParser()
    which = ap.add_mutually_exclusive_group(required=True)
    which.add_argument("--addr", type=lambda s: int(s, 0))
    which.add_argument("--queue", type=str)
    ap.add_argument("--module", help="named handler sibling .rs path")
    ap.add_argument("--max-states", type=int, default=800)
    ap.add_argument("--dependency", action="append", default=[])
    args = ap.parse_args()
    if args.addr is not None:
        print(handler_prompt(args.addr, module=args.module, dependencies=args.dependency, budget=args.max_states))
    elif args.queue:
        print(queue_prompt(args.queue))
    else:
        raise SystemExit("need --addr or --queue")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        raise SystemExit(f"dispatch error: {exc}")
