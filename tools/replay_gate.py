#!/usr/bin/env python3
"""Run the frozen replay corpus. Fail on build, runtime, format, or parity errors."""
from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CAPTURES = ('traj4_ff0', 'traj5', 'traj6', 'traj7', 'traj8', 'traj9', 'traj10',
            'traj11', 'traj12', 'traj13', 'traj14', 'traj15', 'traj16', 'traj17',
            'traj19', 'death1', 'death2', 'death3', 'crouch_pan1', 'screen_transition1')
# Frozen legacy exception observed before the object split. Exact fields and counts,
# including zeros, are intentional: neither new regressions nor new notes are exempt.
LEGACY = dict(x=0, y=53, hvel=0, vvel=0, ground=0, timer=0, dir=0, frame=0,
              camx=0, camy=0, edge=0, en_r=0, en_l=0, sptr=0, spx=0, sdir=0,
              standby=2, active=2, objslots=490, odd=204)
REQUIRED = {'x', 'y', 'hvel', 'vvel', 'ground', 'timer', 'dir', 'frame', 'camx', 'camy'}


def validate(name: str, output: str, returncode: int) -> dict[str, int]:
    if returncode:
        raise ValueError(f'runtime exited {returncode}')
    if 'note:' in output:
        raise ValueError('unexpected note')
    lines = [line for line in output.splitlines() if line.startswith('mismatches:')]
    if len(lines) != 1:
        raise ValueError(f'expected one mismatch summary, found {len(lines)}')
    fields = {}
    for token in lines[0].removeprefix('mismatches:').split():
        match = re.fullmatch(r'([a-zA-Z_][a-zA-Z_0-9]*)=(\d+)', token)
        if not match or match[1] in fields:
            raise ValueError(f'malformed or duplicate field: {token}')
        fields[match[1]] = int(match[2])
    if not REQUIRED <= fields.keys():
        raise ValueError('missing required summary fields')
    if name == 'traj4_ff0':
        if fields != LEGACY:
            raise ValueError(f'legacy baseline changed: {fields}')
    elif any(fields.values()):
        raise ValueError(f'nonzero mismatches: { {k: v for k, v in fields.items() if v} }')
    return fields


def main() -> int:
    # Always build successfully before executing, even when SML2_BIN overrides the binary.
    try:
        build = subprocess.run(['cargo', 'build', '--release'], cwd=ROOT,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        if build.returncode:
            print(build.stdout, end='')
            print('REGRESSION build failed', file=sys.stderr)
            return 1
        binary = os.environ.get('SML2_BIN', './target/release/sml2-runtime')
        failures = 0
        for name in CAPTURES:
            result = subprocess.run([binary, '--level', '19' if name == 'traj19' else '0',
                                     '--replay', f'captures/{name}.json'], cwd=ROOT,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            try:
                validate(name, result.stdout, result.returncode)
                if name == 'traj4_ff0':
                    summary = next(l for l in result.stdout.splitlines() if l.startswith('mismatches:'))
                    print(f'{name:20s}{summary}  notes=0')
            except ValueError as exc:
                failures += 1
                print(f'REGRESSION {name}: {exc}')
                # Include diagnostics for failures; successful replays stay quiet.
                print(result.stdout[-6000:])
        print(f'gate FAILED: {failures} capture(s)' if failures else 'gate done')
        return int(bool(failures))
    except OSError as exc:
        print(f'REGRESSION execution failed: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
