#!/usr/bin/env python3
"""Check per-level dynamic anim reachability, not behavior parity.

Starts from level/boss spawns, follows slot/record morphs to a fixpoint, and
requires every result to dispatch to an implemented handler or verified no-op.
New/unclassified $FFD5 writers fail the check. Interaction edges are conservative.
Exit 0: closed; 1: failed bounds/dispatch/writer checks; 2: invalid inputs.
See tools/README.md for commands and evidence limits."""

from __future__ import annotations

import argparse
import bisect
import json
import re
import sys
from collections import deque
from pathlib import Path

import os

ROOT = Path(__file__).resolve().parent.parent
DECOMP = Path(os.environ.get("SML2_DECOMP_ROOT") or (ROOT / "decomp" if (ROOT / "decomp").exists() else ROOT.parent / "sml2decomp"))
ASM = DECOMP / "main.asm"
OBJECTS = ROOT / "extracted" / "objects.json"
COVERAGE = ROOT / "handler_coverage.json"
LEVELS = ROOT / "extracted" / "levels"

TABLE_B_FROM_LEVEL = 0x0A  # `sprite_map_index`: level >= $0A selects table B
LEVEL_BANK = 22  # $58000
BOSS_BANK = 2  # bank-2 boss code, pseudo-address $8000..

# Every write to the slot anim byte `$FFD5`, keyed by main.asm line.  The
# parser asserts this set is exactly what the file contains.
#   kind "store":  a literal (or `xor a` / read-modify-write) store
#   kind "owner":  a rewrite inside one handler; applies to the anims whose
#                  table entry dispatches to that handler
#   kind "interact": an interaction-dispatch arm reached from any slot
#   kind "record": writes the spawn record's anim byte (ptr+3) and the slot
SLOT_WRITES = {
    3008: {"kind": "interact", "mode": "add", "op": -1, "name": "anim-1 $85DC"},
    3032: {"kind": "interact", "mode": "set", "op": 0, "name": "anim=0 $85FC (defeat/clear)"},
    3057: {"kind": "interact", "mode": "add", "op": +2, "name": "anim+2 $8611"},
    3064: {"kind": "interact", "mode": "add", "op": +1, "name": "anim+1 $8633"},
    3125: {"kind": "interact", "mode": "set", "op": 16, "name": "anim=16 $8695"},
    3167: {"kind": "interact", "mode": "set", "op": 16, "name": "anim=16 $86DA"},
    3191: {"kind": "record", "mode": "set", "op": 18, "name": "slot+record=18 $86E4/$86F9"},
    4047: {"kind": "boss_owner", "mode": "set", "op": 3, "name": "boss anim=3 $9443"},
    4675: {"kind": "boss_owner", "mode": "set", "op": 4, "name": "boss anim=4 $98F8"},
    27551: {"kind": "owner", "mode": "set", "op": 1, "name": "anim=1 $58380"},
    29719: {"kind": "owner", "mode": "add", "op": -1, "name": "anim-1 $5918B"},
    30053: {"kind": "owner", "mode": "set", "op": 62, "name": "anim=62 $593A5"},
}
# Interaction arms are gated on the slot's `$FFE2` type byte.  `gate` reads the
# anim profile's declared `type_sets`; a type the profile scanner did not see
# does not fire the arm here (and is reported as an unmodelled case only when
# it changes the answer).
INTERACTION_GATES = {
    3008: lambda ts: any((t & 0x03) == 0x03 for t in ts),  # bit1 set, bit0 set
    3032: lambda ts: True,  # $85FC is reached from several kill arms
    3057: lambda ts: 6 in ts,  # type == 6, nothing held
    3064: lambda ts: any((t & 0x02) and not (t & 0x01) for t in ts),
    3125: lambda ts: any(t == 0x89 for t in ts),
    3167: lambda ts: any(t == 0x85 for t in ts),
    3191: lambda ts: any(t & 0x80 for t in ts),  # bit7, low bits zero
}
# Owner-local rewrites are attributed by handler, not by type byte.
OWNER_MORPHS = {27551: 1, 29719: -1, 30053: 62}
BOSS_OWNER_MORPHS = {4047: 3, 4675: 4}


def fail(msg: str) -> None:
    print(f"anim_closure: {msg}", file=sys.stderr)
    raise SystemExit(2)


def load_inputs():
    for p in (ASM, OBJECTS, COVERAGE):
        if not p.is_file():
            fail(f"missing input {p}")
    objects = json.loads(OBJECTS.read_text())
    coverage = json.loads(COVERAGE.read_text())
    manifests = {}
    for n in range(1, 33):
        p = LEVELS / f"level{n:02}.manifest.json"
        if not p.is_file():
            fail(f"missing manifest {p}")
        manifests[n] = json.loads(p.read_text())
    return objects, coverage, manifests


def parse_asm(text: str):
    """Labels, slot-anim writes, spawn-anim writes.

    Label addresses are ROM offsets: bank 2's space is $8000.. and bank $16's
    is $58000.., so ``addr >> 14`` recovers the bank.
    """
    lab_re = re.compile(r"^(Unknown[A-Za-z]*_0x([0-9A-Fa-f]{4,6})):")
    call_re = re.compile(r"^call\s+(Unknown[A-Za-z]*_0x([0-9A-Fa-f]{4,6}))")
    lines = text.splitlines()
    labels = []
    slot_lines = []
    spawn_lines = []
    call_sites = []  # (line, target name)
    for i, raw in enumerate(lines):
        line = raw.strip()
        m = lab_re.match(line)
        if m:
            labels.append((i, int(m.group(2), 16), m.group(1)))
        mc = call_re.match(line)
        if mc:
            call_sites.append((i + 1, mc.group(1)))
        if "$FFD5" in line and not re.match(r"ldh a, \[\$FFD5\]", line):
            slot_lines.append(i + 1)
        if re.search(r"ld\s*\[sObjectSpawnAnim\],\s*a", line):
            value, how = None, "computed"
            for k in range(i - 1, max(i - 18, -1), -1):
                s = lines[k].strip()
                if not s or s.startswith(";"):
                    continue
                mv = re.match(r"ld a,\s*(\d+)\s*$", s)
                if mv:
                    value, how = int(mv.group(1)), "immediate"
                    break
                if s == "xor a":
                    value, how = 0, "xor"
                    break
                if re.match(r"ld a,\s*\[", s):
                    break
            spawn_lines.append({"line": i + 1, "value": value, "how": how})
    lab_idx = [x[0] for x in labels]
    for w in spawn_lines:
        pos = bisect.bisect_right(lab_idx, w["line"] - 1) - 1
        w["addr"] = labels[pos][1] if pos >= 0 else None
        w["label"] = labels[pos][2] if pos >= 0 else None
    by_name = {name: addr for _, addr, name in labels}
    callers = {}
    for line, name in call_sites:
        target = by_name.get(name)
        if target is not None:
            callers.setdefault(target, []).append(line)
    return labels, slot_lines, spawn_lines, callers


def enclosing_handler(addr: int | None, handlers: set[int]) -> int | None:
    """Largest handler entry at or below ``addr`` in the same bank."""
    if addr is None:
        return None
    bank = addr >> 14
    best = None
    for h in handlers:
        if h >> 14 == bank and h <= addr and (best is None or h > best):
            best = h
    return best


def owners_for_line(line: int, labels, callers, handlers, depth: int = 0):
    """Handlers that can execute ``line``.

    A spawn or rewrite inside a `call`ed helper belongs to the helper's
    callers, not to whatever handler happens to contain the helper's code.
    Resolution recurses a few levels; the returned list is deduped.
    """
    pos = bisect.bisect_right([x[0] for x in labels], line - 1) - 1
    if pos < 0:
        return []
    addr, name = labels[pos][1], labels[pos][2]
    if addr in handlers:
        return [addr]
    if name.startswith("UnknownCall_") and addr in callers and depth < 4:
        out = []
        for caller_line in callers[addr]:
            out.extend(owners_for_line(caller_line, labels, callers, handlers, depth + 1))
        if out:
            return sorted(set(out))
    h = enclosing_handler(addr, handlers)
    return [h] if h else []


def table_for(level_id: int, objects):
    if level_id >= TABLE_B_FROM_LEVEL:
        return objects["anim_profiles_b"]
    return objects["anim_profiles_a"]


def evaluate_levels(manifests, objects, implemented, sites, owner_sites):
    """Fixpoint over anim values per level; returns (reports, violations)."""
    level_reports = []
    violations = []
    for n in sorted(manifests):
        m = manifests[n]
        lid = m["level_id"]
        table = table_for(lid, objects)
        table_len = len(table)
        produced = {r["anim_index"] for r in m["spawns"]["records"]}
        steps = []
        changed = True
        while changed:
            changed = False
            handlers = {table[a].get("handler") for a in produced if 0 <= a < table_len}
            # Dynamic spawn writes whose parent handler now runs.
            for s in sites:
                if s["table"] != "level" or not s["value"]:
                    continue
                hit = next((o for o in s["owners"] if o in handlers), None)
                if hit is not None and s["value"] not in produced:
                    produced.add(s["value"])
                    steps.append(f"spawn {s['value']} from {hit:#07x} (line {s['line']})")
                    changed = True
            # Handler-local anim rewrites: every anim that dispatches to it.
            for line, info in owner_sites.items():
                hit = next((o for o in info["owners"] if o in handlers), None)
                if hit is None:
                    continue
                mode, op = SLOT_WRITES[line]["mode"], info["op"]
                for a in list(produced):
                    if a < table_len and table[a].get("handler") == hit:
                        b = op if mode == "set" else a + op
                        if b not in produced and 0 <= b < 256:
                            produced.add(b)
                            steps.append(f"{info['name']}: {a}->{b}")
                            changed = True
            # Interaction arms: type-gated, reachable from any slot.
            for line, gate in INTERACTION_GATES.items():
                mode, op = SLOT_WRITES[line]["mode"], SLOT_WRITES[line]["op"]
                for a in list(produced):
                    if a >= table_len:
                        continue
                    types = table[a].get("type_sets") or []
                    if not gate(types):
                        continue
                    b = op if mode == "set" else a + op
                    if b not in produced and b >= 0:
                        produced.add(b)
                        steps.append(f"{SLOT_WRITES[line]['name']}: {a}->{b}")
                        changed = True
        bad = []
        for a in sorted(produced):
            if a < 0 or a >= table_len:
                bad.append({"anim": a, "reason": f"outside table (len {table_len})"})
                continue
            h = table[a].get("handler")
            if h not in implemented:
                bad.append({"anim": a, "reason": f"handler {h:#07x} not implemented"})
        for b in bad:
            violations.append({"level": n, **b})
        level_reports.append(
            {
                "level": n,
                "level_id": lid,
                "table": "B" if lid >= TABLE_B_FROM_LEVEL else "A",
                "table_len": table_len,
                "records": len(m["spawns"]["records"]),
                "produced_anims": sorted(produced),
                "steps": steps,
                "violations": bad,
            }
        )
    return level_reports, violations


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", type=Path)
    ap.add_argument("--quiet", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    objects, coverage, manifests = load_inputs()
    labels, slot_lines, spawn_lines, callers = parse_asm(ASM.read_text())

    # --- completeness: every $FFD5 writer is a declared, classified site ---
    found = set(slot_lines)
    declared = set(SLOT_WRITES)
    problems = []
    if found - declared:
        problems.append(
            "unclassified $FFD5 write(s) at main.asm line(s): "
            + ", ".join(str(x) for x in sorted(found - declared))
        )
    if declared - found:
        problems.append(
            "declared $FFD5 write(s) missing from main.asm: "
            + ", ".join(str(x) for x in sorted(declared - found))
        )

    implemented = set(coverage["level_tables"]["implemented_bodies"]) | set(
        coverage["level_tables"]["rom_no_ops"]
    )
    boss_implemented = set(coverage["boss_table"]["implemented_bodies"]) | set(
        coverage["boss_table"]["rom_no_ops"]
    )

    # --- spawn writes classified by owning table ---------------------------
    sites = []
    for w in spawn_lines:
        bank = (w["addr"] >> 14) if w["addr"] is not None else None
        if bank == LEVEL_BANK:
            sites.append(
                {**w, "table": "level", "owners": owners_for_line(w["line"], labels, callers, implemented)}
            )
        elif bank == BOSS_BANK:
            sites.append(
                {**w, "table": "boss", "owners": owners_for_line(w["line"], labels, callers, boss_implemented)}
            )
        else:
            sites.append({**w, "table": "other", "owners": []})

    # --- owner-local morphs (level handlers) ------------------------------
    owner_sites = {}
    for line, op in OWNER_MORPHS.items():
        hs = owners_for_line(line, labels, callers, implemented)
        if not hs:
            problems.append(f"$FFD5 writer at line {line} has no enclosing level handler")
        owner_sites[line] = {"op": op, "owners": hs, "name": SLOT_WRITES[line]["name"]}
    boss_owner_sites = {}
    for line, op in BOSS_OWNER_MORPHS.items():
        hs = owners_for_line(line, labels, callers, boss_implemented)
        if not hs:
            problems.append(f"$FFD5 writer at line {line} has no enclosing boss handler")
        boss_owner_sites[line] = {"op": op, "owners": hs, "name": SLOT_WRITES[line]["name"]}

    # --- per-level fixpoint ------------------------------------------------
    level_reports, violations = evaluate_levels(manifests, objects, implemented, sites, owner_sites)

    # --- boss spawn writes -------------------------------------------------
    bh = objects.get("boss_anim_handlers") or []
    boss_sites = []
    for s in sites:
        if s["table"] != "boss" or not s["value"]:
            continue
        h = bh[s["value"]] if 0 <= s["value"] < len(bh) else None
        ok = h in boss_implemented
        boss_sites.append(
            {"line": s["line"], "anim": s["value"], "owners": s["owners"], "handler": h, "ok": ok}
        )
        if not ok:
            violations.append(
                {"level": "boss", "anim": s["value"], "reason": f"boss handler {h} not implemented"}
            )
    # Boss owner-local rewrites must stay inside the boss table too.
    boss_morphs = []
    for line, info in boss_owner_sites.items():
        target = info["op"]
        h = bh[target] if 0 <= target < len(bh) else None
        ok = h in boss_implemented
        boss_morphs.append({"line": line, "anim": target, "owners": info["owners"], "handler": h, "ok": ok})
        if not ok:
            violations.append(
                {"level": "boss", "anim": target, "reason": f"boss morph handler {h} not implemented"}
            )

    if args.self_test:
        # Sensitivity: the checker must fail when a producer goes out of range
        # or a table entry points at a body that is not implemented.  Mutate a
        # deep copy only; the committed data is untouched.
        import copy

        mutated = copy.deepcopy(manifests)
        mutated[1]["spawns"]["records"].append({"anim_index": 200})
        _, range_v = evaluate_levels(mutated, objects, implemented, sites, owner_sites)
        mutated_objects = copy.deepcopy(objects)
        mutated_objects["anim_profiles_a"][9]["handler"] = 0xDEAD
        _, handler_v = evaluate_levels(manifests, mutated_objects, implemented, sites, owner_sites)
        checks = [
            ("out-of-range spawn anim 200", range_v),
            ("unimplemented table handler $0DEAD", handler_v),
        ]
        failed = [name for name, v in checks if not v]
        for name, v in checks:
            print(f"  self-test {name}: {'detected' if v else 'NOT DETECTED'}")
        if failed:
            print(f"anim_closure: self-test failed: {', '.join(failed)}", file=sys.stderr)
            return 1

    report = {
        "schema": "sml2-anim-closure/2",
        "slot_writes": {str(k): v for k, v in sorted(SLOT_WRITES.items())},
        "spawn_writes": sites,
        "levels": level_reports,
        "boss_spawn_writes": boss_sites,
        "boss_morphs": boss_morphs,
        "problems": problems,
        "violations": violations,
    }
    if args.json:
        args.json.write_text(json.dumps(report, indent=1) + "\n")

    if not args.quiet:
        print("level tbl len records produced steps  result")
        for L in level_reports:
            print(
                f"{L['level']:>5} {L['table']:>3} {L['table_len']:>3} {L['records']:>7} "
                f"{len(L['produced_anims']):>8} {len(L['steps']):>5}  "
                f"{'FAIL' if L['violations'] else 'closed'}"
            )
        print("\ndynamic spawn writes ($5B36D, bank $16):")
        for s in sites:
            if s["table"] == "level":
                print(
                    f"  line {s['line']:>5}  anim {str(s['value']):>8}  owner "
                    f"{','.join('$%05X' % o for o in s['owners']) or '-'}"
                )
        print("boss spawner writes ($B224, bank 2):")
        for b in boss_sites + boss_morphs:
            print(
                f"  line {b['line']:>5}  anim {b['anim']:>3}  handler "
                f"{('%d' % b['handler']) if b['handler'] is not None else '-':>7}  "
                f"{'ok' if b['ok'] else 'FAIL'}"
            )
        if problems:
            print("\nPROBLEMS:")
            for p in problems:
                print(f"  {p}")
        if violations:
            print("\nVIOLATIONS:")
            for v in violations:
                print(f"  level {v['level']} anim {v['anim']}: {v['reason']}")
        else:
            print(
                f"\nclosed: {sum(len(L['produced_anims']) for L in level_reports)} level/anim pairs, "
                f"{len(boss_sites) + len(boss_morphs)} boss anim writes, 0 out-of-range or unported landings"
            )
    return 1 if violations or problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
