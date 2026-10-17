"""Closed, ROM-checked source contracts; never native game implementations.

Full body witnesses deliberately bound the accepted instruction graphs. A byte
change requires source review, not a broader computed-bank/stack exception.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class SavedRegister(Enum):
    AF = 'af'
    BC = 'bc'
    DE = 'de'
    HL = 'hl'


@dataclass(frozen=True)
class InterruptFrame:
    saved: tuple[SavedRegister, ...] = tuple(SavedRegister)
    continuation: str = 'opaque interrupted PC; consumed by RETI, never followed'


@dataclass(frozen=True)
class Witness:
    address: int
    hex_bytes: str
    purpose: str

    @property
    def data(self):
        return bytes.fromhex(self.hex_bytes)


COMMON = (
    Witness(0x154, 'f3 f5 c5 d5 e5', 'VBlank DI/save AF,BC,DE,HL'),
    Witness(0x18B,
            'fa c9 a2 a7 c2 4d 2a f0 9b fe 0c ca 18 2c fa 48 a2 a7 c2 d7 26 '
            'f0 9b fe 13 ca a3 2c', 'transfer dispatch precedes state19 text dispatch'),
    Witness(0x2CA3, '3e 1a ea 4e a2 ea 00 21 c3 28 45',
            'ending VBlank wrapper selects/stores bank1A'),
)

TEXT_BODY = Witness(0x68572,
    'fa d6 a2 ee ff c6 05 cb 27 cb 37 6f 26 00 11 20 9a 19 5d 54 '
    'fa 73 a2 6f fa 74 a2 67 7e fe ff 28 12 06 14 2a d6 30 fe f0 '
    '20 02 3e ff 12 13 05 20 f2 18 0a 23 06 14 3e ff 12 13 05 20 '
    'f9 7d ea 73 a2 7c ea 74 a2 7e fe fe 20 05 3e ff ea d8 a2 '
    'fa d6 a2 3d ea d6 a2 a7 20 0a 3e 01 ea 67 a2 3e 00 ea 66 a2 '
    'fa 4e a2 ea 00 21 3e 01 ea 82 ff e1 d1 c1 f1 d9',
    'complete reviewed text graph; no writes to caller-bank fact')

TRANSFER_BODY = Witness(0x26D7,
    'fa 65 a2 ea 00 21 f0 b5 4f f0 b6 47 f0 b1 6f f0 b2 67 f0 b3 '
    '5f f0 b4 57 2a 12 13 0b 79 e6 3f 20 f7 79 e0 b5 78 e0 b6 '
    '7d e0 b1 7c e0 b2 7b e0 b3 7a e0 b4 78 b1 20 04 af ea 48 a2 '
    'f0 9b fe 13 c4 89 20 3e 01 e0 82 fa 4e a2 ea 00 21 e1 d1 c1 f1 d9',
    'complete reviewed transfer graph; state19/caller-bank/source-bank facts unchanged')


@dataclass(frozen=True)
class EndingInterruptContract:
    name: str
    entry: int
    bank: int
    witnesses: tuple[Witness, ...]
    terminal: int
    constant_loads: tuple[tuple[int, int], ...]
    skipped_calls: tuple[int, ...] = ()
    frame: InterruptFrame = InterruptFrame()

    @property
    def body(self):
        return next(w for w in self.witnesses if w.address == self.entry)

    def validate(self, rom, source, entry, bank, bindings, instruction_size):
        if entry != self.entry or bank != self.bank:
            raise ValueError(f'{self.name}: entry/selected bank contradicts contract')
        if bindings:
            raise ValueError('ending entry contracts cannot be combined with helper bindings')
        for witness in self.witnesses:
            start, data = witness.address, witness.data
            if rom[start:start + len(data)] != data:
                raise ValueError(f'{self.name}: ROM witness mismatch at 0x{start:X} ({witness.purpose})')
            pc = start
            while pc < start + len(data):
                if pc not in source.locations:
                    raise ValueError(f'{self.name}: missing exact source at 0x{pc:X}')
                pc += instruction_size(rom[pc])
            if pc != start + len(data):
                raise ValueError('contract witness ends within an instruction')

    def contains(self, pc):
        return self.entry <= pc < self.entry + len(self.body.data)

    def terminal_instructions(self, rom, source, stack, selected, instruction_size):
        if stack:
            raise ValueError('interrupt terminal has unbalanced local push/call stack')
        if selected != self.bank:
            raise ValueError('interrupt terminal selected bank contradicts caller-bank fact')
        pops = {SavedRegister.HL: 0xE1, SavedRegister.DE: 0xD1,
                SavedRegister.BC: 0xC1, SavedRegister.AF: 0xF1}
        expected = bytes(pops[r] for r in reversed(self.frame.saved)) + bytes([0xD9])
        end = self.entry + len(self.body.data)
        if rom[end - len(expected):end] != expected:
            raise ValueError('interrupt terminal does not consume typed save frame with RETI')
        instructions = {}
        pc = self.terminal
        while pc < end:
            size = instruction_size(rom[pc])
            raw = rom[pc:pc + size]
            instructions[pc] = f'{raw.hex(" ").upper():8s} {source.locations[pc]}'
            pc += size
        return instructions

    def describe(self, source):
        lines = [
            f'**Entry contract: `{self.name}`. COMPLETE is scoped to these preconditions.**',
            '- Entry carries the VBlank saved AF,BC,DE,HL frame above an opaque interrupted PC.',
            '- State byte $FF9B=$13 and caller-bank byte $A24E=$1A are caller preconditions.',
            '- These are source-checked caller witnesses, not a proof of arbitrary caller reachability.',
            '- Exact reviewed body bytes preserve these facts; changed code fails closed.',
            '- Exit sets frame-ready $FF82=1, restores bank1A, pops HL,DE,BC,AF, then RETI.',
            '- The interrupted program is outside this source packet; no native CPU/stack implementation is claimed.',
        ]
        if self.skipped_calls:
            lines += [
                '- Source-bank byte $A265=$1A: only the two checked bank1A request producers are covered.',
                '- At $2716, CALL NZ,$2089 taken edge is impossible: exact $2712 LDH $9B / CP $13 and unchanged state19 fact.',
                '- Generic/non19 sound dispatch and the unproven bank10 ending body are excluded; no sound helper is bound.',
            ]
        else:
            lines.append('- Text entry is after the caller-owned HRAM DMA seam; this packet does not implement that seam.')
        lines.append('\nChecked ROM/source witnesses (all bytes must match):')
        for w in self.witnesses:
            lines.append(f'- ROM 0x{w.address:05X}..0x{w.address + len(w.data):05X} (exclusive): '
                         f'{w.purpose}; {source.locations[w.address]}; `{w.hex_bytes.upper()}`')
        return '\n'.join(lines)


CONTRACTS = {
    'ending-text-vblank': EndingInterruptContract(
        'ending-text-vblank', 0x68572, 0x1A,
        COMMON + (Witness(0x68528, 'cd a0 ff 3e 00 ea 43 ff ea 43 ff fa d6 a2 a7 c2 72 45',
                          'phase-nonzero text dispatch after caller-owned HRAM DMA'), TEXT_BODY),
        0x685D5, ((0x685D5, 0x1A),)),
    'ending-transfer-vblank': EndingInterruptContract(
        'ending-transfer-vblank', 0x26D7, 0x1A,
        COMMON + (
            Witness(0x2CB9, '3e 1a ea 4e a2 ea 00 21 c3 0f 40', 'ending main wrapper stores caller bank1A'),
            Witness(0x6806E,
                    '3e 1a ea 65 a2 3e 56 ea b1 ff 3e 46 ea b2 ff 3e 00 ea b3 ff '
                    '3e 80 ea b4 ff 3e 00 ea b5 ff 3e 08 ea b6 ff 3e ff ea 48 a2',
                    'bank1A request $4656 -> $8000, length $0800'),
            Witness(0x683D0,
                    '3e 1a ea 65 a2 3e 96 ea b1 ff 3e 50 ea b2 ff 3e 00 ea b3 ff '
                    '3e 98 ea b4 ff 3e 00 ea b5 ff 3e 04 ea b6 ff 3e ff ea 48 a2',
                    'bank1A request $5096 -> $9800, length $0400'),
            TRANSFER_BODY),
        0x2719, ((0x26D7, 0x1A), (0x271D, 0x1A)), (0x2716,)),
}
