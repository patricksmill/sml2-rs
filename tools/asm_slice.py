"""Bounded, bank-aware SM83 control-flow slicing. Never executes game logic.

ROM bytes determine edges; source locations annotate them. Unresolved computed
transfers and exhausted budgets are reported, never presented as complete slices.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
import re

try:
    from ending_interrupt_contracts import CONTRACTS
    from state_table_contracts import STATE_TABLE_CONTRACTS
except ImportError:
    from tools.ending_interrupt_contracts import CONTRACTS
    from tools.state_table_contracts import STATE_TABLE_CONTRACTS

THREE = {0x01, 0x08, 0x11, 0x21, 0x31, 0xC2, 0xC3, 0xC4, 0xCA, 0xCC,
         0xCD, 0xD2, 0xD4, 0xDA, 0xDC, 0xEA, 0xFA}
TWO = {0x06, 0x0E, 0x10, 0x16, 0x18, 0x1E, 0x20, 0x26, 0x28, 0x2E,
       0x30, 0x36, 0x38, 0x3E, 0xC6, 0xCB, 0xCE, 0xD6, 0xDE, 0xE0, 0xE6,
       0xE8, 0xEE, 0xF0, 0xF6, 0xF8, 0xFE}
ILLEGAL = {0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD}

# `rst $28` words the decomp lists as instructions. Exclusive ROM ends, reviewed
# against baserom: little-endian pointers, then the next real instruction.
# A source `dw` declaration still wins when SourceIndex records one.
REVIEWED_RST28_TABLES = {
    0x58D4E: 0x58D56,  # 0x58D34 anim 45: $4D56, $4D78, $4D9A, $4DBF
    0x9754: 0x9762,  # 0x96FE boss anim 6: $5762, $5443, $5786, $57BD, $57CF, $57E9, $57F6
}

# Instruction spans whose source cursor is shifted by a mislisted pointer table.
# The ROM bytes are the authority; the brief records them without an asm line.
REVIEWED_RAW_SPANS = (
    (0x9762, 0x97FA),  # 0x96FE boss anim 6 state bodies
)


def rom_address(bank: int, cpu: int) -> int:
    if not 0 <= bank < 512 or not 0 <= cpu < 0x8000:
        raise ValueError(f'invalid ROM bank/CPU address: {bank:X}:${cpu:04X}')
    if cpu < 0x4000:
        return cpu
    if bank == 0:
        raise ValueError('banked CPU target requires a nonzero selected bank')
    return bank * 0x4000 + cpu - 0x4000


def bank_cpu(rom: int) -> tuple[int, int]:
    if rom < 0:
        raise ValueError('negative ROM offset')
    bank = rom // 0x4000
    return bank, rom if bank == 0 else 0x4000 + rom % 0x4000


def instruction_size(op: int) -> int:
    if op in ILLEGAL:
        raise ValueError(f'illegal SM83 opcode ${op:02X}')
    return 3 if op in THREE else 2 if op in TWO else 1


@dataclass
class SourceIndex:
    locations: dict[int, str]
    tables: dict[int, int]  # first byte -> exclusive end of a contiguous db/dw declaration

    @classmethod
    def load(cls, root: Path, rom: bytes) -> 'SourceIndex':
        symbols = {}
        sym = root / 'sml2.sym'
        if sym.exists():
            for line in sym.read_text().splitlines():
                m = re.match(r'([\da-fA-F]+):([\da-fA-F]{4}) (\S+)', line)
                if m and int(m[2], 16) < 0x8000:
                    symbols[m[3]] = rom_address(int(m[1], 16), int(m[2], 16))
        sources = {name: (root / name).read_text().splitlines()
                   for name in ('home.asm', 'main.asm')}
        # Forward references with explicit ROM anchors need no symbol build.
        # Their encoded CPU words are checked below before indexing a table.
        for lines in sources.values():
            for raw in lines:
                label = re.match(r'\s*([\w.]+):', raw.split(';', 1)[0])
                anchor = re.search(r'_0x([\da-fA-F]+)$', label[1]) if label else None
                if anchor:
                    symbols.setdefault(label[1], int(anchor[1], 16))
        locations, tables = {}, {}
        for name, lines in sources.items():
            cursor = None
            global_label = ''
            run_start, run_end, run_kind = None, None, None
            for number, raw in enumerate(lines, 1):
                code = raw.split(';', 1)[0].strip()
                if not code:
                    continue
                is_data = re.match(r'^(db|dw)\s+(.+)', code, re.I)
                # Labels delimit declarations; continuation rows may have intervening comments.
                if not is_data and run_start is not None:
                    tables[run_start] = run_end
                    run_start = run_end = run_kind = None
                section = re.match(r'SECTION\b.*ROM([0X])(?:\[\$([\da-fA-F]+)\])?', code, re.I)
                if section:
                    bank_match = re.search(r'BANK\[\$([\da-fA-F]+)\]', code, re.I)
                    bank = int(bank_match[1], 16) if bank_match else 0
                    cpu = int(section[2], 16) if section[2] else (0 if section[1] == '0' else 0x4000)
                    cursor = rom_address(bank, cpu)
                    continue
                label = re.match(r'([\w.]+):', code)
                if label:
                    addr = re.search(r'_0x([\da-fA-F]+)$', label[1])
                    symbol = global_label + label[1] if label[1].startswith('.') else label[1]
                    if not label[1].startswith('.'):
                        global_label = label[1]
                    cursor = int(addr[1], 16) if addr else symbols.get(symbol, cursor)
                    code = code[label.end():].strip()
                    if not code:
                        continue
                    is_data = re.match(r'^(db|dw)\s+(.+)', code, re.I)
                if cursor is None:
                    continue
                if is_data:
                    # Support literal data and ROM-checked symbolic dw words.
                    # Strings, expressions, unresolved names and mismatched
                    # encodings invalidate the whole contiguous declaration.
                    values = [v.strip() for v in is_data[2].split(',')]
                    kind = is_data[1].lower()
                    encoded = bytearray()
                    symbolic = False
                    valid = True
                    for value in values:
                        if re.fullmatch(r'\$[\da-fA-F]+|\d+', value):
                            number_value = int(value[1:], 16) if value.startswith('$') else int(value)
                        elif kind == 'dw' and re.fullmatch(r'[A-Za-z_.][\w.]*', value):
                            symbol = global_label + value if value.startswith('.') else value
                            if symbol not in symbols:
                                valid = False
                                break
                            number_value = bank_cpu(symbols[symbol])[1]
                            symbolic = True
                        else:
                            valid = False
                            break
                        width = 2 if kind == 'dw' else 1
                        if not 0 <= number_value < 1 << (width * 8):
                            valid = False
                            break
                        encoded.extend(number_value.to_bytes(width, 'little'))
                    if (not valid or (symbolic and rom[cursor:cursor + len(encoded)] != encoded)):
                        run_start = run_end = run_kind = None
                        cursor = None
                        continue
                    size = len(values) * (2 if is_data[1].lower() == 'dw' else 1)
                    if run_start is not None and (cursor != run_end or kind != run_kind):
                        tables[run_start] = run_end
                        run_start = run_end = run_kind = None
                    if run_start is None:
                        run_start, run_kind = cursor, kind
                    for addr in range(cursor, cursor + size):
                        locations[addr] = f'{name}:{number} {code}'
                    cursor += size
                    run_end = cursor
                    continue
                # Unknown directives stop inferred address progression until the next anchor.
                if code.lower().startswith(('include', 'incbin', 'def ', 'ds ', 'assert', 'purge')):
                    cursor = None
                    continue
                if not 0 <= cursor < len(rom):
                    cursor = None
                    continue
                try:
                    size = instruction_size(rom[cursor])
                except ValueError:
                    cursor = None
                    continue
                locations[cursor] = f'{name}:{number} {code}'
                cursor += size
            if run_start is not None:
                tables[run_start] = run_end
        return cls(locations, tables)


@dataclass
class Slice:
    instructions: dict[int, str]
    tables: dict[int, list[int]]
    dependencies: dict[int, str]
    unresolved: set[str]
    entry_contract: str | None = None
    contract_evidence: str = ''
    state_table_contract: str | None = None
    state_table_evidence: str = ''
    interrupt_exits: set[int] = field(default_factory=set)
    # rst $28 table start -> table indices reachable from the state byte's values.
    inferred_tables: dict[int, tuple[int, ...]] = field(default_factory=dict)
    entry_states: tuple[int, ...] = ()

    @property
    def complete(self) -> bool:
        return not self.unresolved


# Object state byte (`$FFDF`). An `rst $28` table without a source declaration is
# bounded by the state values that can reach its dispatch, never by pointer shape.
STATE = 0xFFDF
ALL_VALUES = frozenset(range(256))
# Opcodes that leave every flag unchanged (loads, 16-bit inc/dec, stack, control flow).
FLAGS_KEPT = (set(range(0x40, 0x80)) - {0x76}) | {
    0x00, 0x01, 0x02, 0x03, 0x06, 0x08, 0x0A, 0x0B, 0x0E, 0x11, 0x12, 0x13, 0x16, 0x18,
    0x1A, 0x1B, 0x1E, 0x20, 0x21, 0x22, 0x23, 0x26, 0x28, 0x2A, 0x2B, 0x2E, 0x30, 0x31,
    0x32, 0x33, 0x36, 0x38, 0x3A, 0x3B, 0x3E, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC8,
    0xC9, 0xCA, 0xCC, 0xCD, 0xD0, 0xD1, 0xD2, 0xD4, 0xD5, 0xD8, 0xD9, 0xDA, 0xDC, 0xE0,
    0xE1, 0xE2, 0xE5, 0xEA, 0xF0, 0xF2, 0xF3, 0xF5, 0xF9, 0xFA, 0xFB}
# Opcodes that leave A unchanged, beyond the flag-preserving ones that do not load A.
A_KEPT = (FLAGS_KEPT - set(range(0x78, 0x7F)) - {0x0A, 0x1A, 0x2A, 0x3A, 0x3E, 0xF0, 0xF2, 0xFA}) | {
    0x04, 0x05, 0x0C, 0x0D, 0x14, 0x15, 0x1C, 0x1D, 0x24, 0x25, 0x2C, 0x2D, 0x34, 0x35,
    0x37, 0x3F, 0x09, 0x19, 0x29, 0x39, 0xA7, 0xB7, 0xFE, 0xE8, 0xF8} | set(range(0xB8, 0xC0))
# Branch condition -> (flag, taken when set): nz/z/nc/c for jr, jp, call and ret.
CONDITIONS = {0x20: ('z', False), 0x28: ('z', True), 0x30: ('c', False), 0x38: ('c', True),
              0xC2: ('z', False), 0xCA: ('z', True), 0xD2: ('c', False), 0xDA: ('c', True),
              0xC4: ('z', False), 0xCC: ('z', True), 0xD4: ('c', False), 0xDC: ('c', True),
              0xC0: ('z', False), 0xC8: ('z', True), 0xD0: ('c', False), 0xD8: ('c', True)}


def slice_handler(rom: bytes, source: SourceIndex, entry: int, *, bank: int | None = None,
                  bindings: dict[int, str] | None = None, budget: int = 800,
                  entry_contract: str | None = None, state_table_contract: str | None = None,
                  entry_states: frozenset[int] = frozenset({0})) -> Slice:
    """Slice `entry`. Undeclared state tables are bounded by a fixed point over the
    state values written anywhere in the full closure (bound helpers included),
    starting from `entry_states` (the spawn state). Writers outside the closure,
    such as another object's code, are not modelled and are listed in the brief."""
    kwargs = dict(bank=bank, budget=budget, entry_contract=entry_contract,
                  state_table_contract=state_table_contract)
    states = frozenset(entry_states)
    # Untracked first: state tracking splits paths, so it only ever upgrades an
    # incomplete slice and never costs a complete one its budget.
    legacy, _ = _walk(rom, source, entry, bindings=bindings, entry_sv=states, track=False, **kwargs)
    if legacy.complete or entry_contract or state_table_contract:
        return legacy
    result, _ = _walk(rom, source, entry, bindings=bindings, entry_sv=states, **kwargs)
    if not result.inferred_tables:
        return legacy
    for _ in range(32):
        scan, writes = _walk(rom, source, entry, bindings={}, entry_sv=states, **kwargs)
        if writes <= states:
            break
        states |= writes
    else:
        legacy.unresolved.add('state bound: state-table inference did not reach a fixed point')
        return legacy
    result, _ = _walk(rom, source, entry, bindings=bindings, entry_sv=states, **kwargs)
    result.entry_states = tuple(sorted(states))
    for start, reach in sorted(result.inferred_tables.items()):
        if not reach:
            result.unresolved.add(f'0x{start:X}: no state value reaches this rst $28 table')
    for reason in sorted(scan.unresolved):
        result.unresolved.add(f'state-table write scan (bound helpers traversed): {reason}')
    if result.complete:
        return result
    legacy.unresolved.update(f'state bound: {reason}' for reason in result.unresolved)
    return legacy


def _walk(rom: bytes, source: SourceIndex, entry: int, *, bank: int | None,
          bindings: dict[int, str] | None, budget: int, entry_contract: str | None,
          state_table_contract: str | None, entry_sv: frozenset[int],
          track: bool = True) -> tuple[Slice, frozenset[int]]:
    if not 0 <= entry < len(rom) or budget < 1:
        raise ValueError('invalid entry address or instruction budget')
    bindings = bindings or {}
    writes: set[int] = set()
    contract = CONTRACTS[entry_contract] if entry_contract in CONTRACTS else None
    if entry_contract is not None and contract is None:
        raise ValueError(f'unknown entry contract: {entry_contract}')
    state_contract = STATE_TABLE_CONTRACTS.get(state_table_contract)
    if state_table_contract is not None and state_contract is None:
        raise ValueError(f'unknown state-table contract: {state_table_contract}')
    if contract and state_contract:
        raise ValueError('entry and state-table contracts cannot be combined')
    initial_bank = (contract.bank if contract else bank_cpu(entry)[0]) if bank is None else bank
    if not 0 <= initial_bank < (len(rom) + 0x3FFF) // 0x4000:
        raise ValueError('selected bank is outside this ROM')
    if entry >= 0x4000 and initial_bank != bank_cpu(entry)[0]:
        raise ValueError('selected bank contradicts banked entry ROM offset')
    # Each stack word is a known CPU address or an unknown saved register. This
    # resolves immediate HL push/ret and ordinary register save/restore patterns.
    # The last three fields track the state byte (see `_track_state`): A as a
    # function of the state, the state values possible here, and the state
    # values that set Z and C.
    pending = [(entry, initial_bank, None, None, (), None, entry_sv, (None, None))]
    seen = set()
    result = Slice({}, {}, {}, set())
    allowed_state_code = frozenset()
    if state_contract:
        result.state_table_contract = state_contract.name
        try:
            allowed_state_code = state_contract.validate(rom, source, entry, initial_bank, bindings, instruction_size)
            result.state_table_evidence = state_contract.describe(source)
        except ValueError as exc:
            result.unresolved.add(str(exc))
            return result, frozenset()
    if contract:
        result.entry_contract = contract.name
        try:
            contract.validate(rom, source, entry, initial_bank, bindings, instruction_size)
            result.contract_evidence = contract.describe(source)
        except ValueError as exc:
            result.unresolved.add(str(exc))
            return result, frozenset()
    while pending:
        state = pending.pop()
        if state in seen:
            continue
        if len(seen) >= budget:
            result.unresolved.add(f'control-flow budget ({budget} states) exhausted; split prerequisite helpers')
            break
        seen.add(state)
        pc, selected, a, hl, stack, amap, sv, flags = state
        if state_contract and (pc not in allowed_state_code or selected != state_contract.bank):
            result.unresolved.add(f'0x{pc:X}: transfer outside checked state-table closure/bank')
            continue
        if contract and not contract.contains(pc):
            result.unresolved.add(f'0x{pc:X}: transfer outside checked entry-contract body')
            continue
        if contract and pc == contract.terminal:
            try:
                terminal = contract.terminal_instructions(rom, source, stack, selected, instruction_size)
                if len(seen) + len(terminal) - 1 > budget:
                    raise ValueError(f'control-flow budget ({budget} states) exhausted at interrupt terminal')
                result.instructions.update(terminal)
                result.interrupt_exits.add(pc)
            except ValueError as exc:
                result.unresolved.add(f'0x{pc:X}: {exc}')
            continue
        if pc != entry and pc in bindings:
            result.dependencies[pc] = bindings[pc]
            if stack:
                try:
                    # A bound helper may write the state byte; only the closure-wide set is sound.
                    pending.append((rom_address(selected, stack[-1]), selected, None, None, stack[:-1],
                                    None, entry_sv, (None, None)))
                except (ValueError, TypeError):
                    result.unresolved.add(f'0x{pc:X}: unknown return through bound helper')
            continue
        if not 0 <= pc < len(rom):
            result.unresolved.add(f'0x{pc:X}: target outside ROM')
            continue
        try:
            op = rom[pc]
            size = instruction_size(op)
            if pc + size > len(rom) or (pc // 0x4000 != (pc + size - 1) // 0x4000):
                raise ValueError('instruction crosses ROM bank boundary')
            raw = rom[pc:pc + size]
            location = source.locations.get(pc)
            if location is None and any(start <= pc < end for start, end in REVIEWED_RAW_SPANS):
                location = f'baserom.gb:{pc:05X} reviewed raw span'
            if location is None:
                raise ValueError('no exact source location; regenerate symbols or isolate an ASM slice')
            result.instructions[pc] = f'{raw.hex(" ").upper():8s} {location}'
            cpu = bank_cpu(pc)[1]
            next_cpu = cpu + size
            a0, hl0, amap0 = a, hl, amap
            amap, sv, flags = _track_state(op, raw, a0, hl0, amap, sv, flags, writes)
            if not track:
                amap0, amap, sv, flags = None, None, entry_sv, (None, None)
            def emit(target: int, *, bank=selected, av=a, hv=hl, words=stack, k=amap, s=sv, f=flags):
                pending.append((rom_address(bank, target), bank, av, hv, words, k, s, f))
            def split(taken_target: int | None, fall_target: int | None, *, words=stack):
                # A conditional edge narrows the state values when its flag tests the state byte.
                flag, when_set = CONDITIONS[op]
                tested = flags[0] if flag == 'z' else flags[1]
                taken = sv if tested is None else (sv & tested if when_set else sv - tested)
                fall = sv if tested is None else sv - taken
                # Narrowing never drops code: an empty set only means no state reaches it.
                if taken_target is not None:
                    emit(taken_target, words=words, s=taken)
                if fall_target is not None:
                    emit(fall_target, av=a, hv=hl, s=fall)
            if op == 0x3E:
                a = raw[1]
            elif op == 0xAF:
                a = 0
            elif op not in {0xEA, 0xE0, 0xE2, 0xC3, 0x18, 0x20, 0x28, 0x30, 0x38,
                            0xC2, 0xCA, 0xD2, 0xDA, 0xFE, 0xC5, 0xD5, 0xE5, 0xF5}:
                a = None
            # These loads are valid only in a byte-for-byte checked body. No
            # arbitrary memory facts survive stores or unreviewed callees.
            if contract and pc in dict(contract.constant_loads):
                a = dict(contract.constant_loads)[pc]
            if op == 0x21:
                hl = int.from_bytes(raw[1:], 'little')
            elif op in {0x09, 0x19, 0x22, 0x23, 0x24, 0x25, 0x26, 0x29, 0x2A, 0x2B,
                        0x2C, 0x2D, 0x2E, 0x32, 0x39, 0x3A, 0xF8, 0xCB} or 0x60 <= op <= 0x6F:
                hl = None
            if op == 0xEA and 0x2000 <= int.from_bytes(raw[1:], 'little') < 0x4000:
                if a is None or not 1 <= a <= 31:
                    raise ValueError('unresolved ROM bank write')
                selected = a
            if op in {0xC5, 0xD5, 0xE5, 0xF5}:
                if len(stack) >= 24:
                    raise ValueError('stack budget exceeded')
                emit(next_cpu, bank=selected, av=a, hv=hl, words=stack + (hl if op == 0xE5 else None,))
            elif op in {0xC1, 0xD1, 0xE1, 0xF1}:
                if not stack:
                    raise ValueError('pop consumes unknown caller stack')
                emit(next_cpu, bank=selected, av=None, hv=stack[-1] if op == 0xE1 else hl, words=stack[:-1])
            elif op in {0xC9, 0xD9, 0xC0, 0xC8, 0xD0, 0xD8}:
                if op in {0xC9, 0xD9}:
                    if stack:
                        if stack[-1] is None:
                            raise ValueError('ret target is an unknown stack value')
                        emit(stack[-1], bank=selected, av=a, hv=hl, words=stack[:-1])
                else:
                    if stack and stack[-1] is None:
                        raise ValueError('ret target is an unknown stack value')
                    split(stack[-1] if stack else None, next_cpu, words=stack[:-1])
            elif op in {0xCD, 0xC4, 0xCC, 0xD4, 0xDC}:
                if contract and pc in contract.skipped_calls:
                    # Exact LDH/CP/CALL guard and unchanged state19 fact were
                    # validated together with the complete bounded body.
                    emit(next_cpu, bank=selected, av=a, hv=hl)
                    continue
                if len(stack) >= 24:
                    raise ValueError('call-stack budget exceeded')
                target = int.from_bytes(raw[1:], 'little')
                if op == 0xCD:
                    emit(target, bank=selected, words=stack + (next_cpu,))
                else:
                    split(target, next_cpu, words=stack + (next_cpu,))
            elif op in {0xC3, 0xC2, 0xCA, 0xD2, 0xDA}:
                if op == 0xC3:
                    emit(int.from_bytes(raw[1:], 'little'), bank=selected)
                else:
                    split(int.from_bytes(raw[1:], 'little'), next_cpu)
            elif op in {0x18, 0x20, 0x28, 0x30, 0x38}:
                target = next_cpu + int.from_bytes(raw[1:], 'little', signed=True)
                if op == 0x18:
                    emit(target, bank=selected)
                else:
                    split(target, next_cpu)
            elif op == 0xE9:
                if hl is None:
                    raise ValueError('computed jp hl requires a dependency or exact target evidence')
                emit(hl, bank=selected)
            elif op == 0xEF:  # rst $28 consumes its return address as a pointer table
                start = rom_address(selected, next_cpu)
                end = source.tables.get(start)
                if end is None:
                    end = REVIEWED_RST28_TABLES.get(start)
                if state_contract and pc == state_contract.dispatch:
                    if (start != state_contract.pointers[0] or selected != state_contract.bank
                            or end != state_contract.declaration[1]):
                        raise ValueError('checked state-table dispatch mismatch')
                    end = state_contract.pointers[1]
                indices = None if amap0 is None else frozenset(amap0[v] for v in sv)
                declared = None
                if (end is not None and indices is not None and not (contract or state_contract)
                        and any(not _in_bank(selected, int.from_bytes(rom[i:i + 2], 'little'))
                                for i in range(start, min(end, len(rom) - 1), 2))):
                    # A declaration that mixes pointers with data is bounded by state instead.
                    declared, end = end, None
                if end is None and track and not (contract or state_contract):
                    if indices is None:
                        raise ValueError('rst $28 has no source table and its index is not the state byte')
                    reach = indices | frozenset(result.inferred_tables.get(start, ()))
                    result.inferred_tables[start] = tuple(sorted(reach))
                    if not reach:
                        continue  # no state value reaches this dispatch yet
                    if len(entry_sv) == 256:
                        raise ValueError('rst $28 state index is unbounded: the closure writes a non-constant state')
                    if max(reach) >= 0x80:
                        raise ValueError(f'rst $28 state index {max(reach)} wraps; guard the state first')
                    end = start + 2 * (max(reach) + 1)
                    if declared is not None and end > declared:
                        raise ValueError('state-bounded table runs past its source declaration')
                if (end is None or end <= start or end > len(rom)
                        or start // 0x4000 != (end - 1) // 0x4000 or (end - start) % 2):
                    raise ValueError('rst $28 has no complete, even-length source table')
                targets = [int.from_bytes(rom[i:i + 2], 'little') for i in range(start, end, 2)]
                result.tables[start] = targets
                for index, target in enumerate(targets):
                    arm = sv if indices is None else frozenset(v for v in sv if amap0[v] == index)
                    emit(target, bank=selected, av=None, hv=None, k=None, s=arm, f=(None, None))
            elif op & 0xC7 == 0xC7:
                raise ValueError(f'rst ${op & 0x38:02X} requires explicit semantics')
            elif op in {0x10, 0x76}:
                raise ValueError('STOP/HALT requires scheduler/interrupt evidence')
            else:
                emit(next_cpu, bank=selected, av=a, hv=hl)
        except (ValueError, TypeError) as exc:
            result.unresolved.add(f'0x{pc:X}: {exc}')
    return result, frozenset(writes)


def _in_bank(bank: int, cpu: int) -> bool:
    return cpu < 0x4000 or (bank != 0 and cpu < 0x8000)


IDENTITY = tuple(range(256))
# A-register operations as (result, Z predicate, C predicate) over the old A value x.
# A predicate of None keeps that flag; 'clear'/'set' are constant flags.
_UNARY = {
    0x3C: (lambda x: (x + 1) & 0xFF, 'z', None),
    0x3D: (lambda x: (x - 1) & 0xFF, 'z', None),
    0x2F: (lambda x: ~x & 0xFF, None, None),
    0xA7: (lambda x: x, 'z', 'clear'),
    0xB7: (lambda x: x, 'z', 'clear'),
    0x07: (lambda x: ((x << 1) | (x >> 7)) & 0xFF, 'clear', lambda x: x >> 7),
    0x0F: (lambda x: ((x >> 1) | (x << 7)) & 0xFF, 'clear', lambda x: x & 1),
}
_IMMEDIATE = {
    0xC6: (lambda x, n: (x + n) & 0xFF, 'z', lambda x, n: x + n > 0xFF),
    0xD6: (lambda x, n: (x - n) & 0xFF, 'z', lambda x, n: x < n),
    0xE6: (lambda x, n: x & n, 'z', 'clear'),
    0xEE: (lambda x, n: x ^ n, 'z', 'clear'),
    0xF6: (lambda x, n: x | n, 'z', 'clear'),
    0xFE: (lambda x, n: x, lambda x, n: x == n, lambda x, n: x < n),
}
_CB_A = {  # CB-prefixed operations on A (low three bits 7)
    0x07: (lambda x: ((x << 1) | (x >> 7)) & 0xFF, 'z', lambda x: x >> 7),
    0x0F: (lambda x: ((x >> 1) | (x << 7)) & 0xFF, 'z', lambda x: x & 1),
    0x27: (lambda x: (x << 1) & 0xFF, 'z', lambda x: x >> 7),
    0x2F: (lambda x: (x >> 1) | (x & 0x80), 'z', lambda x: x & 1),
    0x37: (lambda x: ((x << 4) | (x >> 4)) & 0xFF, 'z', 'clear'),
    0x3F: (lambda x: x >> 1, 'z', lambda x: x & 1),
}


def _in_bank(bank: int, cpu: int) -> bool:
    return cpu < 0x4000 or (bank != 0 and cpu < 0x8000)


def _track_state(op, raw, a, hl, amap, sv, flags, writes):
    """Advance (amap, sv, flags) over one instruction; record state-byte writes.

    amap[v] is A's value when the state byte holds v (None: A is not a function
    of the state); sv is the state values possible here; flags are the state
    values that set Z and C (None: the flag says nothing about the state)."""
    if amap is None and a is not None:
        amap = (a,) * 256  # a known constant is a (constant) function of the state
    stored = None
    if (op == 0xE0 and raw[1] == STATE & 0xFF) or (op == 0xEA and int.from_bytes(raw[1:], 'little') == STATE) \
            or (op in {0x77, 0x22, 0x32} and hl == STATE):
        # Any other A is e.g. a saved-state restore; a bound that depends on it fails.
        stored = ALL_VALUES if amap is None else frozenset(amap[v] for v in sv)
    elif op == 0x36 and hl == STATE:
        stored = frozenset({raw[1]})
    elif op in {0x34, 0x35} and hl == STATE:
        stored = frozenset((v + (1 if op == 0x34 else -1)) & 0xFF for v in sv)
    if stored is not None:
        writes |= stored
        # After an A store, A is the new state; flags described the old value.
        return (IDENTITY if op in {0xE0, 0xEA, 0x77, 0x22, 0x32} and amap is not None else None), stored, (None, None)
    if op == 0xF0 and raw[1] == STATE & 0xFF:
        return IDENTITY, sv, flags
    if op == 0x3E:
        return (raw[1],) * 256, sv, flags
    if op == 0xAF:
        return (0,) * 256, sv, (ALL_VALUES, frozenset())
    rule = None
    if amap is not None:
        if op in _UNARY:
            rule = _UNARY[op]
        elif op in _IMMEDIATE:
            fn2, z2, c2 = _IMMEDIATE[op]
            n = raw[1]
            rule = (lambda x, f=fn2: f(x, n),
                    z2 if isinstance(z2, str) else (lambda x, f=z2: f(x, n)),
                    c2 if isinstance(c2, str) else (lambda x, f=c2: f(x, n)))
        elif op == 0xCB and raw[1] in _CB_A:
            rule = _CB_A[raw[1]]
        elif op == 0xCB and raw[1] & 7 == 7 and raw[1] >= 0x40:
            bit = 1 << ((raw[1] >> 3) & 7)
            if raw[1] < 0x80:
                rule = (lambda x: x, lambda x: not x & bit, None)
            else:
                rule = ((lambda x: x & ~bit) if raw[1] < 0xC0 else (lambda x: x | bit), None, None)
    if rule is not None:
        fn, z, c = rule
        new = tuple(fn(x) for x in amap)
        def flag(spec, old, result):
            if spec is None:
                return old
            if spec == 'clear':
                return frozenset()
            if spec == 'z':
                return frozenset(v for v in range(256) if result[v] == 0)
            return frozenset(v for v in range(256) if spec(amap[v]))
        return new, sv, (flag(z, flags[0], new), flag(c, flags[1], new))
    new_flags = flags if op in FLAGS_KEPT else (None, None)
    keeps_a = op in A_KEPT or (op == 0xCB and raw[1] & 7 != 7)
    return (amap if keeps_a else None), sv, new_flags
