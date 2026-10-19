#!/usr/bin/env python3
"""Source-derived per-level route facts.

The route matrix needs entry, exit, secret route, required mechanic and boss
facts per level.  This tool derives them from the extracted data plus the two
ROM sources the extraction does not cover (the `$377B` secret-entry overlay), so
a matrix row is reproducible instead of transcribed.

Facts and their sources:

  entry        manifest header (`mario_spawn_x/y`, camera), the overworld node
               table (`extracted/map/nodes.bin`: level index, progress slot)
  exit         the `$4A`/`$4B` metatiles in the decompressed layout.  The
               in-level probe is `tile & $FE == $4A`; `$4A` is a plain bell and
               `$4B` the "route already cleared" twin (`sExitTileFlag = $FF`)
  secret entry `main.asm` `$55C0`: 8-byte per-level records at ROM `$377B`
               (the header's Y/X/cam-Y/cam-X block), applied only when
               `sLevelProgressFlags` bit 3 or the secret-route item set the flag
  boss         non-zero cells in the 48-byte `sSceneMap` (`levelNN.scene.bin`)
               plus `boss_def_index`/`boss_def_portable`
  mechanics    level properties (moon physics, scroll limit, autoscroll), the
               tile classes the player probe models (`$4C..$4F` water,
               `$52..$57` moon zone, `$50/$51` hazard), door tiles (`$07`),
               side-pipe walls (`$0C..$0F`), and the handler names behind the
               level's anims (`objects/dispatch.rs`)

Usage:
    python3 tools/route_matrix.py                 # all 32 levels
    python3 tools/route_matrix.py --levels 12,19,28,29
    python3 tools/route_matrix.py --levels 12 --json out.json

Exit status: 0 the matrix was built; 2 inputs are missing or malformed.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

import os

ROOT = Path(__file__).resolve().parent.parent
DECOMP = Path(os.environ.get("SML2_DECOMP_ROOT") or (ROOT / "decomp" if (ROOT / "decomp").exists() else ROOT.parent / "sml2decomp"))
LEVELS = ROOT / "extracted" / "levels"
OBJECTS = ROOT / "extracted" / "objects.json"
NODES = ROOT / "extracted" / "map" / "nodes.bin"
ROM = DECOMP / "baserom.gb"
DISPATCH = ROOT / "crates" / "sml2-core" / "src" / "objects" / "dispatch.rs"

TABLE_B_FROM_LEVEL = 0x0A
# Mirrors `scene::boss_def_index` and `scene::boss_def_portable`.
BOSS_LADDER = {0x05: 0, 0x09: 1, 0x0D: 2, 0x10: 3, 0x13: 4, 0x17: 5}
BOSS_PORTABLE = set(range(9))
SECRET_OVERLAY_ROM = 0x377B
SECRET_RECORD_LEN = 8

EXIT_TILE_MASK = 0xFE
EXIT_TILE = 0x4A
WATER_MASK = 0xFC
WATER_TILE = 0x4C
MOON_MIN, MOON_MAX = 0x52, 0x57
HAZARD_MASK = 0xFE
HAZARD_TILE = 0x50


def fail(msg: str) -> None:
    print(f"route_matrix: {msg}", file=sys.stderr)
    raise SystemExit(2)


def handler_names() -> dict[int, str]:
    """`0x5A773 => self.handler_0x5a773(...)` -> {0x5A773: "handler_0x5a773"}."""
    if not DISPATCH.is_file():
        fail(f"missing {DISPATCH}")
    out = {}
    for m in re.finditer(r"0x([0-9A-Fa-f]+)\s*=>\s*self\.(\w+)", DISPATCH.read_text()):
        out[int(m.group(1), 16)] = m.group(2)
    return out


def load_nodes() -> tuple[list[int], list[int]]:
    if not NODES.is_file():
        fail(f"missing {NODES}")
    blob = NODES.read_bytes()
    n = 130
    if len(blob) < 2 * n:
        fail(f"{NODES}: expected at least {2 * n} bytes, got {len(blob)}")
    return list(blob[:n]), list(blob[n : 2 * n])


def secret_overlay(level_id: int) -> dict | None:
    if not ROM.is_file():
        fail(f"missing {ROM} (set SML2_DECOMP_ROOT)")
    off = SECRET_OVERLAY_ROM + level_id * SECRET_RECORD_LEN
    rec = ROM.read_bytes()[off : off + SECRET_RECORD_LEN]
    if len(rec) < SECRET_RECORD_LEN:
        fail(f"{ROM}: truncated secret overlay at {off:#x}")
    if not any(rec):
        return None
    y = rec[0] | (rec[1] << 8)
    x = rec[2] | (rec[3] << 8)
    if x >= 256 * 16 or y >= 48 * 16:
        # Past the 32-level table (level 31's slot holds unrelated bytes).
        return None
    cam_y = rec[4] | (rec[5] << 8)
    cam_x = rec[6] | (rec[7] << 8)
    return {"raw": list(rec), "x": x, "y": y, "camera_x": cam_x, "camera_y": cam_y}


def tile_at(raw: bytes, col: int, row: int) -> int | None:
    if 0 <= col < 256 and 0 <= row < 48:
        return raw[row * 256 + col]
    return None


def landing_cell(raw: bytes, x: int, y: int) -> dict | None:
    """First non-air metatile below a drop-in point (`$60` is the air tile)."""
    col, row = x // 16, y // 16
    for r in range(row, min(row + 12, 48)):
        t = tile_at(raw, col, r)
        if t is not None and t != 0x60:
            return {"col": col, "row": r, "tile": t, "drop_px": (r - row) * 16}
    return None


def build(level: int, objects, nodes, names) -> dict:
    man = json.loads((LEVELS / f"level{level:02}.manifest.json").read_text())
    lid = man["level_id"]
    raw = (LEVELS / man["layout"]["file"]).read_bytes()
    scene = (LEVELS / f"level{level:02}.scene.bin").read_bytes()
    table = objects["anim_profiles_b"] if lid >= TABLE_B_FROM_LEVEL else objects["anim_profiles_a"]

    exits = [
        {"col": i % 256, "row": i // 256, "tile": v, "x": (i % 256) * 16, "y": (i // 256) * 16}
        for i, v in enumerate(raw)
        if v & EXIT_TILE_MASK == EXIT_TILE
    ]
    cells = [(i % 256, i // 256, v) for i, v in enumerate(scene) if v]
    scenes = [
        {"col": c, "row": r, "id": v, "x": c * 16, "y": r * 16} for c, r, v in cells
    ]

    def count(pred) -> dict:
        hits = [i for i, v in enumerate(raw) if pred(v)]
        if not hits:
            return {"count": 0}
        cols = [h % 256 for h in hits]
        rows = [h // 256 for h in hits]
        return {
            "count": len(hits),
            "col": [min(cols), max(cols)],
            "row": [min(rows), max(rows)],
        }

    anims = sorted({r["anim_index"] for r in man["spawns"]["records"]})
    handlers = sorted({table[a]["handler"] for a in anims if a < len(table)})

    secret = secret_overlay(lid)
    if secret is not None:
        secret["landing"] = landing_cell(raw, secret["x"], secret["y"])

    props = man["properties"]
    mechanics = []
    if props.get("space_physics") == 0x08:
        mechanics.append("timed float (moon bit 3)")
    elif props.get("space_physics") == 0x01:
        mechanics.append("hold-A float (moon bit 0)")
    elif props.get("space_physics"):
        mechanics.append(f"moon physics (space_physics={props['space_physics']:#04x})")
    if props.get("unknown"):
        mechanics.append(f"scroll limit (properties={props['unknown']:#04x})")
    if props.get("auto_scroll"):
        mechanics.append(f"autoscroll ({props['auto_scroll']:#04x})")
    if count(lambda v: v & WATER_MASK == WATER_TILE)["count"]:
        mechanics.append("water/swim")
    if count(lambda v: MOON_MIN <= v <= MOON_MAX)["count"]:
        mechanics.append("moon zones")
    if count(lambda v: v & HAZARD_MASK == HAZARD_TILE)["count"]:
        mechanics.append("hazards")
    if count(lambda v: v in (0x05, 0x07, 0x08, 0x09, 0x0A, 0x0B))["count"]:
        mechanics.append("pipes")
    if count(lambda v: 0x0C <= v <= 0x0F)["count"]:
        mechanics.append("side pipes")
    if any(e["tile"] == 0x4B for e in exits):
        mechanics.append("alternate ($4B) exit")

    boss = None
    arena_ids = sorted({s["id"] for s in scenes if 11 <= s["id"] <= 16})
    if lid in BOSS_LADDER and arena_ids:
        def_index = BOSS_LADDER[lid]  # phase 0; phases increment on retry
        boss = {
            "scene_ids": arena_ids,
            "def_index_phase0": def_index,
            "portable": def_index in BOSS_PORTABLE,
        }

    return {
        "level": level,
        "level_id": lid,
        "label": man["header_label"],
        "table": "B" if lid >= TABLE_B_FROM_LEVEL else "A",
        "records": len(man["spawns"]["records"]),
        "tileset": {
            "label": man["graphics"]["label"],
            "map": man["tileset_map"]["label"],
            "file": man["graphics"]["file"],
        },
        "entry": {
            "x": man["header"]["mario_spawn_x"],
            "y": man["header"]["mario_spawn_y"],
            "camera_x": man["header"]["camera_start_x"],
            "camera_y": man["header"]["camera_start_y"],
            "time_limit": man["header"]["time_limit"],
            "music_track": man["header"]["music_track"],
            "layout_index": man["header"]["layout_index"],
            "scene_state": man["header"]["scene_state"],
            "node": next((i for i, v in enumerate(nodes[0]) if v == lid), None),
            "progress_slot": next(
                (nodes[1][i] for i, v in enumerate(nodes[0]) if v == lid), None
            ),
        },
        "exits": exits,
        "secret_entry": secret,
        "scenes": scenes,
        "boss": boss,
        "mechanics": mechanics,
        "tiles": {
            "water": count(lambda v: v & WATER_MASK == WATER_TILE),
            "moon": count(lambda v: MOON_MIN <= v <= MOON_MAX),
            "hazard": count(lambda v: v & HAZARD_MASK == HAZARD_TILE),
            "pipe": count(lambda v: v in (0x05, 0x07, 0x08, 0x09, 0x0A, 0x0B)),
            "side_pipe": count(lambda v: 0x0C <= v <= 0x0F),
        },
        "anims": anims,
        "handlers": [
            {"addr": h, "name": names.get(h, f"0x{h:X}")} for h in handlers
        ],
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--levels", default="1..32", help="manifest numbers, e.g. 12,19 or 1..32")
    ap.add_argument("--json", type=Path)
    ap.add_argument("--table", action="store_true", help="print the compact matrix table")
    args = ap.parse_args()

    if ".." in args.levels:
        a, b = args.levels.split("..")
        levels = list(range(int(a), int(b) + 1))
    else:
        levels = [int(x) for x in args.levels.split(",") if x.strip()]

    objects = json.loads(OBJECTS.read_text())
    nodes = load_nodes()
    names = handler_names()
    rows = [build(n, objects, nodes, names) for n in levels]

    if args.json:
        args.json.write_text(json.dumps({"schema": "sml2-route-matrix/1", "levels": rows}, indent=1) + "\n")

    if args.table:
        print("level id node exit cells            secret entry      boss  mechanics")
        for r in rows:
            exits = " ".join(f"({e['col']},{e['row']})${e['tile']:02X}" for e in r["exits"]) or "none"
            sec = (
                f"({r['secret_entry']['x']},{r['secret_entry']['y']})"
                if r["secret_entry"]
                else "none"
            )
            boss = "yes" if r["boss"] else "no"
            print(
                f"{r['level']:>5} {r['level_id']:>2} {str(r['entry']['node']):>4} {exits:<22} {sec:<17} {boss:<5} "
                f"{', '.join(r['mechanics']) or 'plain'}"
            )
    else:
        print(json.dumps({"schema": "sml2-route-matrix/1", "levels": rows}, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
