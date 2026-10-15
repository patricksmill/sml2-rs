#!/usr/bin/env python3
"""Emit a small handler porting brief for one bank-16 anim handler.

Usage:
  python3 tools/handler_task.py --addr 0x58201
  python3 tools/handler_task.py --anim 9 --mode a

Writes tasks/handlers/0x{ADDR}.md (ADDR = handler ROM offset from objects.json).
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DECOMP = Path(os.environ.get("SML2_DECOMP_ROOT", ROOT / ".." / "sml2decomp")).resolve()
OBJECTS_JSON = ROOT / "extracted" / "objects.json"
MAIN_ASM = DECOMP / "main.asm"
BASEROM = DECOMP / "baserom.gb"
OUT_DIR = ROOT / "tasks" / "handlers"

LABEL_RE = re.compile(
    r"^(Unknown(?:Jump|Call|RJump|Data)_0x([0-9A-Fa-f]+)):",
    re.MULTILINE,
)


def load_objects() -> dict:
    with open(OBJECTS_JSON) as f:
        return json.load(f)


def handler_for_anim(data: dict, anim: int, mode: str) -> int:
    key = "anim_profiles_a" if mode == "a" else "anim_profiles_b"
    profiles = data[key]
    if anim < 0 or anim >= len(profiles):
        raise SystemExit(f"anim {anim} out of range for mode {mode} ({len(profiles)} entries)")
    return int(profiles[anim]["handler"])


def anims_for_handler(data: dict, addr: int) -> list[tuple[str, int]]:
    out: list[tuple[str, int]] = []
    for mode, key in (("a", "anim_profiles_a"), ("b", "anim_profiles_b")):
        for i, prof in enumerate(data[key]):
            if int(prof["handler"]) == addr:
                out.append((mode, i))
    return out


def collect_labels(asm_text: str) -> list[tuple[int, int, str]]:
    """(rom_offset, line_no_1based, label_name) sorted by offset."""
    labels: list[tuple[int, int, str]] = []
    for m in LABEL_RE.finditer(asm_text):
        off = int(m.group(2), 16)
        line_no = asm_text[: m.start()].count("\n") + 1
        labels.append((off, line_no, m.group(1)))
    labels.sort(key=lambda t: t[0])
    return labels


def excerpt_asm(asm_text: str, addr: int, window: int = 90) -> tuple[str, int | None]:
    labels = collect_labels(asm_text)
    line_start: int | None = None
    for off, line_no, name in labels:
        if off == addr:
            line_start = line_no
            break
    if line_start is None:
        for off, line_no, name in labels:
            if off <= addr < off + 0x80:
                line_start = line_no
                break
    if line_start is None:
        return "(no label near address; see ROM hex below)\n", None
    lines = asm_text.splitlines()
    i0 = max(0, line_start - 1)
    i1 = min(len(lines), i0 + window)
    chunk = "\n".join(f"{i + 1:6d}| {lines[i]}" for i in range(i0, i1))
    return chunk, line_start


def rom_slice(addr: int, nbytes: int = 96) -> str:
    if not BASEROM.is_file():
        return f"(missing {BASEROM})\n"
    with open(BASEROM, "rb") as f:
        f.seek(addr)
        data = f.read(nbytes)
    return " ".join(f"{b:02X}" for b in data)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--addr", type=lambda s: int(s, 0))
    ap.add_argument("--anim", type=int)
    ap.add_argument("--mode", choices=("a", "b"), default="a")
    ap.add_argument("--window", type=int, default=90)
    args = ap.parse_args()

    data = load_objects()
    if args.addr is not None:
        addr = args.addr
    elif args.anim is not None:
        addr = handler_for_anim(data, args.anim, args.mode)
    else:
        raise SystemExit("need --addr or --anim")

    anims = anims_for_handler(data, addr)
    asm_text = MAIN_ASM.read_text() if MAIN_ASM.is_file() else ""
    excerpt, line_start = excerpt_asm(asm_text, addr, args.window)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path = OUT_DIR / f"0x{addr:05X}.md"

    anim_lines = ", ".join(f"mode-{m} anim {i}" for m, i in anims) or "(no anim profile entry)"

    body = f"""# Handler 0x{addr:05X}

**Anims:** {anim_lines}

**Rust:** add `objects/handlers/` implementation; one arm in `objects/dispatch.rs`.

**Policy:** [`PORTING.md`](../../PORTING.md) — one handler, one test, no whole-file edits.

## ASM excerpt (`main.asm`)

Start line hint: {line_start or "unknown"}

```
{excerpt}
```

## ROM bytes @ 0x{addr:05X}

```
{rom_slice(addr)}
```

## Calls to chase (grep in ../sml2decomp)

Search `UnknownCall_0x` targets in the excerpt; port via `objects/helpers.rs` when already present.
"""
    out_path.write_text(body)
    print(f"wrote {out_path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
