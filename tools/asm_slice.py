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

    @property
    def complete(self) -> bool:
        return not self.unresolved


def slice_handler(rom: bytes, source: SourceIndex, entry: int, *, bank: int | None = None,
                  bindings: dict[int, str] | None = None, budget: int = 800,
                  entry_contract: str | None = None, state_table_contract: str | None = None) -> Slice:
    if not 0 <= entry < len(rom) or budget < 1:
        raise ValueError('invalid entry address or instruction budget')
    bindings = bindings or {}
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
    pending = [(entry, initial_bank, None, None, ())]
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
            return result
    if contract:
        result.entry_contract = contract.name
        try:
            contract.validate(rom, source, entry, initial_bank, bindings, instruction_size)
            result.contract_evidence = contract.describe(source)
        except ValueError as exc:
            result.unresolved.add(str(exc))
            return result
    while pending:
        state = pending.pop()
        if state in seen:
            continue
        if len(seen) >= budget:
            result.unresolved.add(f'control-flow budget ({budget} states) exhausted; split prerequisite helpers')
            break
        seen.add(state)
        pc, selected, a, hl, stack = state
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
                    pending.append((rom_address(selected, stack[-1]), selected, None, None, stack[:-1]))
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
            def emit(target: int, *, bank=selected, av=a, hv=hl, words=stack):
                pending.append((rom_address(bank, target), bank, av, hv, words))
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
                if stack:
                    if stack[-1] is None:
                        raise ValueError('ret target is an unknown stack value')
                    emit(stack[-1], bank=selected, av=a, hv=hl, words=stack[:-1])
                if op not in {0xC9, 0xD9}:
                    emit(next_cpu, bank=selected, av=a, hv=hl)
            elif op in {0xCD, 0xC4, 0xCC, 0xD4, 0xDC}:
                if contract and pc in contract.skipped_calls:
                    # Exact LDH/CP/CALL guard and unchanged state19 fact were
                    # validated together with the complete bounded body.
                    emit(next_cpu, bank=selected, av=a, hv=hl)
                    continue
                if len(stack) >= 24:
                    raise ValueError('call-stack budget exceeded')
                emit(int.from_bytes(raw[1:], 'little'), bank=selected, words=stack + (next_cpu,))
                if op != 0xCD:
                    emit(next_cpu, bank=selected, av=a, hv=hl)
            elif op in {0xC3, 0xC2, 0xCA, 0xD2, 0xDA}:
                emit(int.from_bytes(raw[1:], 'little'), bank=selected)
                if op != 0xC3:
                    emit(next_cpu, bank=selected)
            elif op in {0x18, 0x20, 0x28, 0x30, 0x38}:
                emit(next_cpu + int.from_bytes(raw[1:], 'little', signed=True), bank=selected)
                if op != 0x18:
                    emit(next_cpu, bank=selected)
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
                if (end is None or end <= start or end > len(rom)
                        or start // 0x4000 != (end - 1) // 0x4000 or (end - start) % 2):
                    raise ValueError('rst $28 has no complete, even-length source table')
                targets = [int.from_bytes(rom[i:i + 2], 'little') for i in range(start, end, 2)]
                result.tables[start] = targets
                for target in targets:
                    emit(target, bank=selected)
            elif op & 0xC7 == 0xC7:
                raise ValueError(f'rst ${op & 0x38:02X} requires explicit semantics')
            elif op in {0x10, 0x76}:
                raise ValueError('STOP/HALT requires scheduler/interrupt evidence')
            else:
                emit(next_cpu, bank=selected, av=a, hv=hl)
        except (ValueError, TypeError) as exc:
            result.unresolved.add(f'0x{pc:X}: {exc}')
    return result
