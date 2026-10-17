#!/usr/bin/env python3
"""Write one bank-aware handler brief. Exit 2 for incomplete or invalid slices.

--dependency ROM=FILE::METHOD names an explicitly reviewed, bank-preserving
Rust helper contract. Such dependencies are shown in the brief, not silently
considered translated. --max-states bounds traversal; exhausted budgets produce INCOMPLETE briefs.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys

from asm_slice import SourceIndex, bank_cpu, slice_handler
from state_table_contracts import STATE_TABLE_CONTRACTS
try:
    from ending_interrupt_contracts import CONTRACTS
except ImportError:
    from tools.ending_interrupt_contracts import CONTRACTS

ROOT = Path(__file__).resolve().parents[1]


def handler_for_anim(data: dict, anim: int, mode: str) -> int:
    table = data['boss_anim_handlers'] if mode == 'boss' else data[f'anim_profiles_{mode}']
    if not 0 <= anim < len(table):
        raise ValueError(f'anim {anim} out of range for mode {mode}')
    return table[anim] if mode == 'boss' else table[anim]['handler']


def bindings_for(values: list[str], root: Path) -> dict[int, str]:
    bindings = {}
    for value in values:
        address, contract = value.split('=', 1)
        file, method = contract.rsplit('::', 1)
        path = (root / file).resolve()
        if not path.is_relative_to(root.resolve()):
            raise ValueError('dependency file must be inside repository')
        text = path.read_text()
        if not re.search(rf'\bfn\s+{re.escape(method)}\s*\(', text):
            raise ValueError(f'missing Rust helper: {contract}')
        addr = int(address, 0)
        if addr in bindings:
            raise ValueError(f'duplicate dependency: {address}')
        bindings[addr] = contract
    return bindings


def make_brief(root: Path, decomp: Path, addr: int, *, bank=None, budget=800, dependencies=None,
               entry_contract=None, state_table_contract=None) -> tuple[str, bool]:
    data = json.loads((root / 'extracted/objects.json').read_text())
    rom = (decomp / 'baserom.gb').read_bytes()
    source = SourceIndex.load(decomp, rom)
    result = slice_handler(rom, source, addr, bank=bank, budget=budget, bindings=dependencies,
                           entry_contract=entry_contract, state_table_contract=state_table_contract)
    bank_id, cpu = bank_cpu(addr)
    anims = []
    for mode in ('a', 'b', 'boss'):
        table = data['boss_anim_handlers'] if mode == 'boss' else data[f'anim_profiles_{mode}']
        for i in range(len(table)):
            if handler_for_anim(data, i, mode) == addr:
                anims.append(f'{mode}:{i}')
    instructions = '\n'.join(f'0x{pc:05X}  {line}' for pc, line in sorted(result.instructions.items()))
    tables = '\n'.join(f'- ROM 0x{pc:05X}: ' + ', '.join(f'${p:04X}' for p in targets)
                       for pc, targets in sorted(result.tables.items())) or '- None.'
    contracts = '\n'.join(f'- ROM 0x{pc:05X}: `{contract}` (reviewed bank-preserving contract)'
                          for pc, contract in sorted(result.dependencies.items())) or '- None.'
    unresolved = '\n'.join(f'- {reason}' for reason in sorted(result.unresolved)) or '- None.'
    status = 'COMPLETE' if result.complete else 'INCOMPLETE'
    selected_bank = bank if bank is not None else (CONTRACTS[entry_contract].bank if entry_contract else bank_id)
    scope = f' under entry contract `{entry_contract}`' if entry_contract else ''
    if state_table_contract:
        scope += f' under state-table contract `{state_table_contract}` (entry FFDF in {{0,1,2,3,4}})'
    state_evidence = result.state_table_evidence or ('Contract validation failed; see unresolved work.' if state_table_contract else '- None.')
    entry_evidence = result.contract_evidence or ('Contract validation failed; see unresolved work.' if entry_contract else '- None.')
    exits = ', '.join(f'0x{pc:05X}' for pc in sorted(result.interrupt_exits)) or 'None'
    return f'''# Handler 0x{addr:05X}

**Slice status: {status}**{scope} — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x{addr:05X}`, bank `${bank_id:02X}` ({bank_id} decimal), CPU `${cpu:04X}`.
**Selected bank at entry:** `${selected_bank:02X}`.
**Anims:** {', '.join(anims) or '(helper / no table entry)'}.
**Policy:** [PORTING.md](../../PORTING.md).

Implement in a focused sibling module under `crates/sml2-core/src/objects/`;
register it in its owning `mod.rs` if new, and wire `objects/dispatch.rs`.
Wario modules belong in `objects/wario/`; shared boss helpers stay in `objects/`.
Apply `docs/idiomatic-rust-porting.md`: use typed `Actor` state from `actor.rs`.
Keep slot conversion and legacy world input/effect mapping in an explicit adapter;
do not put raw offsets, register names or HRAM scratch in domain handler logic.
Resolve typed shared-helper prerequisites first. New tests belong beside the
implementation; shared integration fixtures are currently in `objects/tests.rs`.

## Exact ROM-addressed instructions and ASM locations

Addresses and edges derive from ROM bytes. Source labels are not assumed to
mark every entry. Review table bounds and dependency contracts before porting.

```text
{instructions}
```

## Checked entry and interrupt-return contract

{entry_evidence}

Structurally checked interrupt terminal entries: {exits}.

## Checked mixed state-table contract

{state_evidence}

## Recovered rst $28 state tables (CPU targets)

{tables}

## Existing helper dependencies supplied by the supervisor

{contracts}

## Unresolved work

{unresolved}

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
''', result.complete


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    which = ap.add_mutually_exclusive_group(required=True)
    which.add_argument('--addr', type=lambda s: int(s, 0))
    which.add_argument('--anim', type=int)
    ap.add_argument('--mode', choices=('a', 'b', 'boss'), default='a')
    ap.add_argument('--bank', type=lambda s: int(s, 0), help='selected bank for a home-bank entry')
    ap.add_argument('--max-states', type=int, default=800)
    ap.add_argument('--dependency', action='append', default=[])
    ap.add_argument('--entry-contract', choices=sorted(CONTRACTS),
                    help='closed, byte-checked ending VBlank caller profile (scopes COMPLETE)')
    ap.add_argument('--state-table-contract', choices=sorted(STATE_TABLE_CONTRACTS),
                    help='checked mixed declaration and entry state predicate (scopes COMPLETE)')
    ap.add_argument('--out-dir', type=Path, default=ROOT / 'tasks/handlers')
    args = ap.parse_args()
    try:
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT / '../sml2decomp')).resolve()
        data = json.loads((ROOT / 'extracted/objects.json').read_text())
        addr = args.addr if args.addr is not None else handler_for_anim(data, args.anim, args.mode)
        body, complete = make_brief(ROOT, decomp, addr, bank=args.bank, budget=args.max_states,
                                    dependencies=bindings_for(args.dependency, ROOT),
                                    entry_contract=args.entry_contract, state_table_contract=args.state_table_contract)
        args.out_dir.mkdir(parents=True, exist_ok=True)
        out = args.out_dir / f'0x{addr:05X}.md'
        out.write_text(body)
        print(f'wrote {out}: {"complete" if complete else "INCOMPLETE"}', file=sys.stderr)
        return 0 if complete else 2
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f'handler task error: {exc}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
