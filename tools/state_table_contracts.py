"""Opt-in reviewed mixed declarations, separate from native helper bindings.

SHA256 ROM witnesses cover every byte in the bounded instruction closure from
0x5902D's audited packet (including conservative removal reward/spawn paths),
plus the home RST body and both data declarations. Source digests hash UTF-8
records ``f'{pc:06X} {source.locations[pc]}\n'`` in address order: every instruction
start for code, every byte for data. Thus both source text and exact file/line
locations are witnesses. Re-deriving a digest requires reviewing the changed
closure; never refresh these hashes merely to make a changed ROM pass.
"""
from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256


@dataclass(frozen=True)
class Footprint:
    start: int
    end: int
    data: bool
    rom_sha256: str
    source_sha256: str

    def addresses(self, rom, instruction_size):
        pc = self.start
        while pc < self.end:
            yield pc
            pc += 1 if self.data else instruction_size(rom[pc])
        if pc != self.end:
            raise ValueError('state-table footprint ends inside an instruction')


# Derived from the exact ranges printed in the audited 0x5902D packet; no
# declaration inference or pointer validity heuristic contributes to these spans.
FOOTPRINTS = (
    Footprint(0x28, 0x34, False,
              '57e82ce6400646450a92cef50410e017392a06a6290729facd21909e51da2483',
              '19eda6ecdb2edcfc26d8f8e67031c86054ce2ba0e8803eaa1f866721c022ec6a'),
    Footprint(0x3D22, 0x3D42, False,
              '9dd50a51c26dfba8c31d54ccc6a588a32129d4ba179e92d642a36626274678a5',
              '1415a5ec7a6db2567f41e6c77eeba5a01cb1dc4c7e20d60c2e12d0cbfc8c695c'),
    Footprint(0x5902D, 0x5907A, False,
              '6f4e728dc34d9a6f190bb7b42cb3b2692f878827ec8db078caaffa887858af50',
              'd5f667b78cdafe2622c3ff00a55d7fd2bbeab003ff632a7f1dbf56ac14005709'),
    Footprint(0x59092, 0x59125, False,
              'a7a70d03d1a3f0abad38de9dae4c28887fe1d8679e601193c441c01ec6d9931e',
              '2dbc48c73fe954e6643793b4794d991f8bb36e778bee75d1bc5ed6f9564b9a79'),
    Footprint(0x59136, 0x5917A, False,
              '79e1b332e4aa9c7958488b85f6b123d3742190f8f7177222e14c74033d05e8e4',
              'f82ff209c84953e4d4b23df01382fdf5e969b6822a92b61a94aa41eba6ffa097'),
    Footprint(0x5B2D7, 0x5B33A, False,
              '5589095560a3a6d8aea78fd5e2280196a52a5eaba465e740222a19f97b52df30',
              '8c3de24d54c60abb82b113cb8104bb4a12bc36fbf45760f68ccbb145d854b58a'),
    Footprint(0x5B346, 0x5B36D, False,
              '0f4f0817166bd9b9faaa2c901a4f48b8841a1200ee6bba76df855d34a5020537',
              'd033ba1e96f5e27897961285e1322d489cc9eb1f713b2900f18ba6b044bac6c5'),
    Footprint(0x5B38C, 0x5B3B9, False,
              'c13001b33818b4b33d958d67b877efefa54fab0f63a7d9ba697507e73f539da9',
              '5fe1aeda188dc2245f09646c506c89550500186cf984d962eec117af269bd877'),
    Footprint(0x5B3C6, 0x5B407, False,
              '3f32b87920fca99c5e9313a95e2315e033e75636f2f5791cf247234051259344',
              '216e99798b2a61635a3afdc67bff1c8353a1bf9efd018819364d10be0d3d396d'),
    Footprint(0x5B41A, 0x5B441, False,
              '6d43a953932e707f3012dd3cbf37f01eb7364e0f151f072b6d96000bb3b2e0f2',
              '8af046aacf947c9caf1db8304dd00d0dfae904c9d46b78a10ce669bf799deaea'),
    Footprint(0x5B511, 0x5B521, False,
              '7c350aa1348824c55166311def271c05fa18f065801d4831d192eed36cf768a0',
              'f2a6470507dfecd478850ff055b7c0ec44c8fb8bfada54eb6c56a63fb6d3fa63'),
    Footprint(0x5B526, 0x5B52D, False,
              '34dd87c6e4ed27a92569a122831a3f8210440f2f3ab788435b0e3128b6f21e48',
              'bf88653f01ded137a2d0318f98c8a262e2114198bde5c73c2b15a7bfba66fa06'),
    Footprint(0x5907A, 0x59092, True,
              'c9a24e0fe119d8844c70b21ae5b9eae58d3e153e21999c643809abb5d76eb2d5',
              'd98b67711a9eade564d6ef5bc341d4dcf1bffeecd86cd8bca7c4b0931043a5fd'),
    Footprint(0x59125, 0x59136, True,
              '6da9c14e4b80db10251359b23c001d01b1a3f73a64c6277759e8a901a318c26c',
              '1096d296955838688e05e684b03df1fde4328b64340ebe5d24151f12fc2db06c'),
)


@dataclass(frozen=True)
class StateTableContract:
    name: str = 'anim53-state0-4'
    entry: int = 0x5902D
    bank: int = 0x16
    dispatch: int = 0x59079
    declaration: tuple[int, int] = (0x5907A, 0x59092)
    pointers: tuple[int, int] = (0x5907A, 0x59082)
    displacements: tuple[tuple[int, int], ...] = ((0x59082, 0x5908A), (0x5908A, 0x59092))
    footprints: tuple[Footprint, ...] = FOOTPRINTS

    def validate(self, rom, source, entry, bank, bindings, instruction_size):
        if entry != self.entry or bank != self.bank:
            raise ValueError(f'{self.name}: entry/selected bank contradicts contract')
        if bindings:
            raise ValueError('state-table contracts cannot be combined with helper bindings')
        start, end = self.declaration
        pstart, pend = self.pointers
        spans = (self.pointers,) + self.displacements
        if (pstart != start or spans[-1][1] != end or (pend - pstart) != 8
                or any(b - a != 8 or a // 0x4000 != self.bank or (b - 1) // 0x4000 != self.bank for a, b in spans)
                or any(left[1] != right[0] for left, right in zip(spans, spans[1:]))
                or self.dispatch + 1 != start or source.tables.get(start) != end):
            raise ValueError('state-table declaration/length/overlap/bank mismatch')
        allowed = set()
        for w in self.footprints:
            # Code records precede data records, so validate overlap pairwise.
            if not 0 <= w.start < w.end <= len(rom):
                raise ValueError('invalid state-table footprint range')
            if w.start // 0x4000 != (w.end - 1) // 0x4000:
                raise ValueError('state-table footprint crosses bank')
            if sha256(rom[w.start:w.end]).hexdigest() != w.rom_sha256:
                raise ValueError(f'{self.name}: ROM closure witness mismatch at 0x{w.start:X}')
            addresses = tuple(w.addresses(rom, instruction_size))
            for pc in addresses:
                if pc not in source.locations:
                    raise ValueError(f'{self.name}: missing exact source at 0x{pc:X}')
            records = ''.join(f'{pc:06X} {source.locations[pc]}\n' for pc in addresses)
            if sha256(records.encode()).hexdigest() != w.source_sha256:
                raise ValueError(f'{self.name}: source closure witness mismatch at 0x{w.start:X}')
            if not w.data:
                allowed.update(addresses)
        ordered = sorted(self.footprints, key=lambda w: w.start)
        if any(a.end > b.start for a, b in zip(ordered, ordered[1:])):
            raise ValueError('overlapping state-table footprints')
        return frozenset(allowed)

    def describe(self, source):
        lines = [
            f'**State-table contract: `{self.name}`. COMPLETE is scoped to entry FFDF in {{0,1,2,3,4}}.**',
            '- Entry ROM 0x5902D, selected bank $16; zero initializes, nonzero dispatch index is FFDF-1.',
            '- Only dispatch ROM 0x59079 uses four pointers [0x5907A,0x59082); source declaration remains [0x5907A,0x59092).',
            '- Y offsets [0x59082,0x5908A), X offsets [0x5908A,0x59092), allocator E&7 and abort guard are byte/source checked.',
            '- State writers: init=3; timer-expired state3=4; facing helper=1/2; removal clears [FFD0,FFF0) including FFDF=0.',
            '- Assumes stable ROM, ordinary caller/return discipline and no external concurrent staging mutation.',
            '- Allocator removal mode2 skips the general removal indirect [de] store and reward/spawn paths; the conservative traversal still includes their checked bytes.',
            '- Excludes arbitrary FFDF>=5, other entry offsets, global initialization, engine selection, interactions and saved-state restoration.',
            '- These are source/state closure witnesses, not native Rust helper verification or implementation dependencies.',
            '',
            'Checked ROM/source SHA256 witnesses (exclusive ends; derivation in tools/state_table_contracts.py):',
        ]
        for w in self.footprints:
            lines.append(f'- ROM [0x{w.start:05X},0x{w.end:05X}) {"data" if w.data else "code"}; '
                         f'{source.locations[w.start]}; ROM `{w.rom_sha256}`, source `{w.source_sha256}`.')
        return '\n'.join(lines)


STATE_TABLE_CONTRACTS = {'anim53-state0-4': StateTableContract()}
