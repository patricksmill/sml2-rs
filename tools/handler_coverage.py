#!/usr/bin/env python3
"""Audit declared level/boss dispatch coverage; this does not prove behavior parity.

Exit 0: declared coverage complete; 1: pending handlers; 2: invalid inputs.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
ARM = re.compile(r"^\s*0x([\da-fA-F]{4,6})\s*=>\s*(.*?)(?:,\s*)?$", re.M)


def sprite_map_index(level_id: int) -> int:
    if not 0 <= level_id < 32:
        raise ValueError(f"invalid level ID: {level_id}")
    return int(level_id >= 0x0A)


def dispatch_entries(text: str, methods: set[str], rom: bytes) -> dict[int, str]:
    # Comments cannot declare implementation. Accept only the current, explicit dispatch forms.
    text = re.sub(r"/\*.*?\*/|//[^\n]*", "", text, flags=re.S)
    entries = {}
    for match in ARM.finditer(text):
        addr, body = int(match[1], 16), match[2].strip().rstrip(',')
        if addr in entries:
            raise ValueError(f"duplicate dispatch arm: 0x{addr:X}")
        if body == '{}':
            if not 0 <= addr < len(rom) or rom[addr] != 0xC9:
                raise ValueError(f"no-op arm 0x{addr:X} is not a ROM ret")
            kind = 'rom_no_op'
        elif re.fullmatch(r"self\.note_unported\([^\n]*\)", body):
            kind = 'unported'
        else:
            call = re.fullmatch(r"self\.(\w+)\([^\n]*\)", body)
            if not call or call[1] not in methods:
                raise ValueError(f"unrecognized or missing implementation at 0x{addr:X}: {body}")
            kind = 'body'
        entries[addr] = kind
    if not entries or not re.search(r"\bh\s*=>\s*self\.note_unported\(h\)", text):
        raise ValueError('dispatch entries or explicit unknown-handler fallback missing')
    return entries


def checked_handler(value: object, rom: bytes, context: str) -> int:
    if type(value) is not int or not 0 <= value < len(rom):
        raise ValueError(f'{context}: invalid ROM handler {value!r}')
    return value


def audit(root: Path, rom: bytes) -> dict:
    obj = json.loads((root / 'extracted/objects.json').read_text())
    index = json.loads((root / 'extracted/levels/index.json').read_text())
    by_level = {}
    for entry in index['levels']:
        level = entry['level_id']
        if type(level) is not int:
            raise ValueError(f'invalid level ID: {level!r}')
        mode = sprite_map_index(level)
        key = f'level{level + 1:02d}'
        if key in by_level:
            raise ValueError(f'duplicate level: {key}')
        manifest = json.loads((root / f'extracted/levels/{key}.manifest.json').read_text())
        profiles = obj['anim_profiles_b' if mode else 'anim_profiles_a']
        handlers = set()
        for rec in manifest['spawns']['records']:
            anim = rec['anim_index']
            if type(anim) is not int or not 0 <= anim < len(profiles):
                raise ValueError(f'{key}: invalid anim index {anim!r}')
            handlers.add(checked_handler(profiles[anim]['handler'], rom, key))
        by_level[key] = sorted(handlers)
    if len(by_level) != 32:
        raise ValueError(f'expected all 32 levels, found {len(by_level)}')
    declared = set()
    for table in ('anim_profiles_a', 'anim_profiles_b'):
        if not obj[table]:
            raise ValueError(f'{table}: empty profile table')
        declared.update(checked_handler(p['handler'], rom, table) for p in obj[table])
    boss = {checked_handler(h, rom, 'boss table') for h in obj['boss_anim_handlers']}
    if not boss:
        raise ValueError('empty boss table')
    folder = root / 'crates/sml2-core/src/objects'
    sources = sorted(folder.glob('*.rs'))
    if not sources:
        raise ValueError(f'no Rust sources: {folder}')
    methods = set()
    for source in sources:
        methods.update(re.findall(r'\bfn\s+(\w+)\s*\(', source.read_text()))
    entries = dispatch_entries((folder / 'dispatch.rs').read_text(), methods, rom)
    implemented = {h for h, kind in entries.items() if kind != 'unported'}
    spawn = set().union(*(set(h) for h in by_level.values()))

    def table_report(targets: set[int]) -> dict:
        return {'distinct_handlers': len(targets),
                'implemented_bodies': sorted(h for h in targets if entries.get(h) == 'body'),
                'rom_no_ops': sorted(h for h in targets if entries.get(h) == 'rom_no_op'),
                'unported': sorted(targets - implemented)}

    return {
        'schema': 'sml2-handler-coverage/2',
        'scope': 'Declared dispatch only; dynamic reachability and behavioral verification require separate evidence.',
        'spawn_distinct_handlers': len(spawn),
        'ported_match_arms': len(implemented),
        'ported_spawn_reachable': sorted(spawn & implemented),
        'unported_spawn_reachable': sorted(spawn - implemented),
        'level_tables': table_report(declared),
        'boss_table': table_report(boss),
        'by_level': by_level,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--root', type=Path, default=ROOT)
    ap.add_argument('--rom', type=Path)
    ap.add_argument('--out', type=Path)
    args = ap.parse_args()
    try:
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', args.root / '../sml2decomp'))
        report = audit(args.root, (args.rom or decomp / 'baserom.gb').read_bytes())
        out = args.out or args.root / 'handler_coverage.json'
        out.write_text(json.dumps(report, indent=2) + '\n')
        print(f"wrote {out.name}: {len(report['ported_spawn_reachable'])}/{report['spawn_distinct_handlers']} spawn handlers implemented; "
              f"{len(report['boss_table']['unported'])} boss-table handlers pending")
        return int(bool(report['level_tables']['unported'] or report['boss_table']['unported']))
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f'coverage error: {exc}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
