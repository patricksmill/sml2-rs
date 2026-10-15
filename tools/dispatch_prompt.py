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
        "crates/sml2-core/src/objects.rs",
        "crates/sml2-core/src/scene.rs",
    ],
    "boss_clear_ending_handoff": ["crates/sml2-core/src/scene.rs", "crates/sml2-core/src/player_sim.rs"],
    "boss_anim_4_5_chain": ["crates/sml2-core/src/objects.rs"],
    "remaining_handler_classes": [],  # use --addr instead
}


def run_handler_task(addr: int) -> None:
    subprocess.run(
        [sys.executable, str(ROOT / "tools" / "handler_task.py"), f"--addr", hex(addr)],
        check=True,
        cwd=ROOT,
    )


def handler_prompt(addr: int) -> str:
    task = ROOT / "tasks" / "handlers" / f"0x{addr:05X}.md"
    if not task.is_file():
        run_handler_task(addr)
    return f"""Port bank-16 handler ROM 0x{addr:05X}.

Read ONLY:
  - {task.relative_to(ROOT)}
  - crates/sml2-core/src/objects/ (create handlers/, dispatch.rs, staging.rs, helpers.rs as needed; until split lands, minimal edits in objects.rs ONLY for this handler + test + one dispatch arm)

Do NOT read: DESIGN.md, PORT_QUEUE, captures/, notes/, whole main.asm.

Deliverables:
  1. Implementation + one dispatch arm for 0x{addr:05X}
  2. #[test] with slot/staging byte assertions
  3. cargo test -p sml2-core (filter to your new test name)

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
    ap.add_argument("--addr", type=lambda s: int(s, 0))
    ap.add_argument("--queue", type=str)
    args = ap.parse_args()
    if args.addr is not None:
        print(handler_prompt(args.addr))
    elif args.queue:
        print(queue_prompt(args.queue))
    else:
        raise SystemExit("need --addr or --queue")


if __name__ == "__main__":
    main()
