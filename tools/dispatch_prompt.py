#!/usr/bin/env python3
"""Generate bounded handler or queue worker prompts; see tools/README.md."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run_handler_task(addr: int, *, dependencies=(), budget=800, state_table_contract=None) -> None:
    command = [sys.executable, str(ROOT / "tools/handler_task.py"),
               "--addr", hex(addr), "--max-states", str(budget)]
    if state_table_contract:
        command.extend(["--state-table-contract", state_table_contract])
    for dependency in dependencies:
        command.extend(["--dependency", dependency])
    # Always regenerate: a cached brief may refer to stale source or the old slicer.
    subprocess.run(command, check=True, cwd=ROOT)


def handler_prompt(addr: int, *, module: str | None = None, dependencies=(), budget=800, state_table_contract=None) -> str:
    folder = (ROOT / "crates/sml2-core/src/objects").resolve()
    if module is None:
        registry = folder / 'mod.rs'
        entry = None
        if registry.is_file():
            entry = re.search(
                rf'#\[path = "([^"]+)"\]\s+mod handler_0x{addr:x};',
                registry.read_text(),
            )
        if entry is None and registry.is_file():
            # Native modules can use descriptive names. The address-based
            # adapter remains the dispatch identity and mirrors the native path.
            entry = re.search(
                rf'#\[path = "compat/([^" ]+)"\]\s+mod handler_0x{addr:x}_adapter;',
                registry.read_text(),
            )
        if entry is None:
            raise ValueError('choose a descriptive native file with --module, such as objects/enemies/boo.rs')
        module = str((folder / entry[1]).relative_to(ROOT))
    path = (ROOT / module).resolve()
    if not path.is_relative_to(folder) or path.suffix != '.rs' or path.stem in {'mod', 'dispatch', 'staging', 'helpers', 'tests'}:
        raise ValueError('choose a handler module within crates/sml2-core/src/objects/')
    if path.exists() and len(path.read_text().splitlines()) >= 400:
        raise ValueError(f'{module} is already at least 400 lines; choose a new focused sibling module')
    category = path.relative_to(folder).parts[0]
    native_categories = {'enemies', 'platforms', 'items', 'projectiles', 'effects', 'spawning'}
    if category in native_categories:
        adapter = folder / 'compat' / path.relative_to(folder)
    else:
        adapter = path.with_name('adapter.rs' if path.stem == 'behavior' else f'{path.stem}_adapter.rs')
    if adapter.exists() and len(adapter.read_text().splitlines()) >= 400:
        raise ValueError(f'{adapter.name} is already at least 400 lines; choose a focused adapter')
    registration = path.parent / "mod.rs"
    # Grouped files retain their established module ownership and visibility.
    if category in native_categories or path.stem == 'behavior':
        registration = folder / 'mod.rs'
    test_paths = set()
    for owner in (path, adapter):
        if owner.is_file():
            test_paths.update(
                (owner.parent / relative).resolve()
                for relative in re.findall(r'#\[path = "([^"]*tests/[^"]+)"\]', owner.read_text())
            )
    if not test_paths:
        test_paths.add(path.parent / 'tests' / f'{path.stem}.rs')
    test_files = '\n'.join(f'  - {test.relative_to(ROOT)} (local tests)' for test in sorted(test_paths))
    run_handler_task(addr, dependencies=dependencies, budget=budget, state_table_contract=state_table_contract)
    task = ROOT / "tasks/handlers" / f"0x{addr:05X}.md"
    if '**Slice status: COMPLETE**' not in task.read_text():
        raise ValueError('incomplete brief; resolve dependencies before dispatch')
    return f"""Port object handler ROM 0x{addr:05X}.

Read ONLY:
  - PORTING.md
  - docs/idiomatic-rust-porting.md
  - {task.relative_to(ROOT)}
  - crates/sml2-core/src/actor.rs
  - {module} (create if absent)
  - {adapter.relative_to(ROOT)} (create if absent)
{test_files}

Touch ONLY:
  - {module} (implementation and local tests)
  - {adapter.relative_to(ROOT)} (slot compatibility and world input/effect adapter)
{test_files}
  - crates/sml2-core/src/objects/dispatch.rs (one arm)
  - {registration.relative_to(ROOT)} (module registrations only, if new)

Read the supplied brief's exact existing helper contracts when listed.
Do NOT read: DESIGN.md, PORT_QUEUE, captures/, notes/, whole main.asm.
Rules: PORTING.md. Keep a handler module below approximately 400 lines;
choose a new focused sibling module when the existing family exceeds that size.
Domain logic uses typed Actor fields, states and shared native behavior.
No slot offsets, register names or HRAM scratch belong in handler logic.
Keep raw slot conversion in Actor::from_raw_slot / Actor::to_raw_slot at the adapter.
Resolve missing typed shared-helper prerequisites before implementing the handler;
existing byte-oriented helpers establish source contracts, not the native API.

Deliverables:
  1. Native domain implementation + explicit adapter + one dispatch arm for 0x{addr:05X}
  2. Native behavior tests and raw-slot byte assertions at the conversion boundary,
     including source boundaries, frame gates and real producer/consumer paths
  3. Targeted new tests, followed by cargo test -p sml2-core

Reply ESCALATE after 2 failed fix attempts.
"""


def queue_prompt(item_id: str, files=()) -> str:
    live = json.loads((ROOT / "PORT_QUEUE.json").read_text())
    item = next((e for e in live["queue"] if e["id"] == item_id), None)
    if item is None:
        raise SystemExit(f"unknown or non-pending queue id: {item_id}")
    if not files:
        raise ValueError('queue prompts require an explicit scope: pass --file for each allowed file')
    if any("/objects/" in p for p in files) and "crates/sml2-core/src/objects/tests/engine.rs" not in files:
        files = [*files, "crates/sml2-core/src/objects/tests/engine.rs"]
    for file in files:
        path = Path(file)
        if (path.is_absolute() or '..' in path.parts
                or not (ROOT / path).resolve().is_relative_to(ROOT.resolve())):
            raise ValueError(f"task file must stay inside the repository: {file}")
        if not (ROOT / file).is_file():
            raise ValueError(f"missing task file: {file}")
    file_block = "\n".join(f"  - {p}" for p in files)
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
    ap.add_argument("--file", action="append", default=[], help="allowed repository file for a queue prompt; repeat per file")
    ap.add_argument("--max-states", type=int, default=800)
    ap.add_argument("--dependency", action="append", default=[])
    ap.add_argument("--state-table-contract", help="checked mixed declaration/state predicate passed to handler_task")
    args = ap.parse_args()
    if args.file and args.addr is not None:
        ap.error('--file is only valid with --queue; handlers use --module')
    if args.addr is not None:
        print(handler_prompt(args.addr, module=args.module, dependencies=args.dependency, budget=args.max_states, state_table_contract=args.state_table_contract))
    elif args.queue:
        print(queue_prompt(args.queue, files=args.file))
    else:
        raise SystemExit("need --addr or --queue")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        raise SystemExit(f"dispatch error: {exc}")
