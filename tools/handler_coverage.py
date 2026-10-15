#!/usr/bin/env python3
"""Compare spawn-reachable handler addresses vs objects.rs dispatch.

Writes handler_coverage.json at repo root. Exit 1 if a spawn references an
unknown handler (not ported match arm and not note_unported-only).
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OBJECTS_JSON = ROOT / "extracted" / "objects.json"
INDEX = ROOT / "extracted" / "levels" / "index.json"
OBJECTS_RS = ROOT / "crates" / "sml2-core" / "src" / "objects.rs"
OUT = ROOT / "handler_coverage.json"

MATCH_ARM = re.compile(r"0x([0-9A-Fa-f]{5,6})\s*=>")


def sprite_map_index(level_id: int) -> int:
    return 1 if level_id > 10 else 0


def load_spawn_handlers() -> dict[str, set[int]]:
    with open(OBJECTS_JSON) as f:
        obj = json.load(f)
    with open(INDEX) as f:
        index = json.load(f)

    by_level: dict[str, set[int]] = {}
    all_handlers: set[int] = set()

    for entry in index["levels"]:
        level_id = int(entry["level_id"])
        manifest_path = ROOT / "extracted" / "levels" / f"level{level_id + 1:02d}.manifest.json"
        with open(manifest_path) as f:
            man = json.load(f)
        mode = sprite_map_index(level_id)
        prof_key = "anim_profiles_a" if mode == 0 else "anim_profiles_b"
        profiles = obj[prof_key]
        hs: set[int] = set()
        records = man.get("spawns", {}).get("records", [])
        for rec in records:
            anim = int(rec["anim_index"])
            if anim >= len(profiles):
                continue
            h = int(profiles[anim]["handler"])
            hs.add(h)
            all_handlers.add(h)
        by_level[f"level{level_id + 1:02d}"] = sorted(hs)

    return {"all": all_handlers, "by_level": by_level}


def ported_from_rust() -> set[int]:
    text = OBJECTS_RS.read_text()
    return {int(m.group(1), 16) for m in MATCH_ARM.finditer(text)}


def main() -> None:
    spawn = load_spawn_handlers()
    ported = ported_from_rust()
    all_h = spawn["all"]
    unported = sorted(all_h - ported)
    report = {
        "spawn_distinct_handlers": len(all_h),
        "ported_match_arms": len(ported),
        "unported_spawn_reachable": unported,
        "by_level": spawn["by_level"],
    }
    OUT.write_text(json.dumps(report, indent=2) + "\n")
    print(f"wrote {OUT.name}: {len(all_h)} distinct, {len(unported)} unported")
    if unported:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
