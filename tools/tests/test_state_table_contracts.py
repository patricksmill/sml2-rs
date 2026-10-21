"""Mixed state-table contracts fail closed, independently of optional local ROM."""
from contextlib import redirect_stderr
from dataclasses import replace
from hashlib import sha256
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asm_slice import SourceIndex, instruction_size, slice_handler
import dispatch_prompt
import handler_task
from state_table_contracts import Footprint, STATE_TABLE_CONTRACTS

ROOT = Path(__file__).resolve().parents[2]
NAME = 'anim53-state0-4'


class MixedDeclarationTests(unittest.TestCase):
    def test_generic_mixed_declaration_keeps_valid_pointer_tail(self):
        # Even valid-looking offset pairs must never become evidence for a bound.
        rom = bytearray(0x60000)
        rom[0x5902D:0x59033] = bytes.fromhex("F0 DF A7 20 46 C9")
        rom[0x59078:0x5907A] = bytes.fromhex("3D EF")
        rom[0x5907A:0x59092] = bytes.fromhex(
            '92 50 A8 50 BE 50 D8 50 00 51 02 51 04 51 06 51 08 51 0A 51 0C 51 0E 51')
        for pc in (0x59092, 0x590A8, 0x590BE, 0x590D8):
            rom[pc] = 0xC9
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'home.asm').write_text('SECTION "home", ROM0\n')
            lines = ['SECTION "bank16", ROMX, BANK[$16]', 'Entry_0x5902D:',
                     'ldh a, [$FFDF]', 'and a', 'jr nz, $5078', 'ret',
                     'Dispatch_0x59078:', 'dec a', 'rst $28',
                     'db $92,$50,$A8,$50,$BE,$50,$D8,$50',
                     'db $00,$51,$02,$51,$04,$51,$06,$51,$08,$51,$0A,$51,$0C,$51,$0E,$51',
                     'ret']
            for pc in (0x590A8, 0x590BE, 0x590D8):
                lines.extend([f'Arm_0x{pc:X}:', 'ret'])
            (root / 'main.asm').write_text('\n'.join(lines))
            source = SourceIndex.load(root, rom)
            self.assertEqual(source.tables[0x5907A], 0x59092)
            self.assertTrue(source.locations[0x59092].endswith(' ret'))
            result = slice_handler(rom, source, 0x59079)
            self.assertFalse(result.complete)
            self.assertEqual(len(result.tables[0x5907A]), 12)
            # A flag cannot bless a different entry or unreviewed synthetic ROM.
            result = slice_handler(rom, source, 0x59079, state_table_contract=NAME)
            self.assertFalse(result.complete)
            # A fixture-only reviewed manifest exercises the exact dispatch
            # override without depending on the optional copyrighted ROM.
            footprints = []
            ranges = [(0x5902D, 0x59033, False), (0x59078, 0x5907A, False),
                      (0x5907A, 0x59092, True)]
            ranges += [(pc, pc + 1, False) for pc in (0x59092, 0x590A8, 0x590BE, 0x590D8)]
            for start, end, data in ranges:
                addresses = []
                pc = start
                while pc < end:
                    addresses.append(pc)
                    pc += 1 if data else instruction_size(rom[pc])
                records = ''.join(f'{pc:06X} {source.locations[pc]}\n' for pc in addresses)
                footprints.append(Footprint(start, end, data,
                                            sha256(rom[start:end]).hexdigest(),
                                            sha256(records.encode()).hexdigest()))
            fixture_contract = replace(STATE_TABLE_CONTRACTS[NAME], footprints=tuple(footprints))
            with patch.dict(STATE_TABLE_CONTRACTS, {NAME: fixture_contract}):
                checked = slice_handler(rom, source, 0x5902D, state_table_contract=NAME)
            self.assertTrue(checked.complete, checked.unresolved)
            self.assertEqual(checked.tables[0x5907A], [0x5092, 0x50A8, 0x50BE, 0x50D8])
            self.assertFalse(set(range(0x5907A, 0x59092)) & checked.instructions.keys())
            self.assertEqual(source.tables[0x5907A], 0x59092)

    def test_dispatch_flag_forwarding(self):
        with patch.object(dispatch_prompt.subprocess, 'run') as run:
            dispatch_prompt.run_handler_task(0x5902D, state_table_contract=NAME)
        self.assertIn('--state-table-contract', run.call_args.args[0])
        self.assertIn(NAME, run.call_args.args[0])


class CheckedStateTableTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if not (cls.decomp / 'baserom.gb').is_file():
            raise unittest.SkipTest('optional local baserom.gb not found')
        cls.rom = (cls.decomp / 'baserom.gb').read_bytes()
        cls.source = SourceIndex.load(cls.decomp, cls.rom)
        cls.contract = STATE_TABLE_CONTRACTS[NAME]

    def checked(self, rom=None, source=None, **kwargs):
        return slice_handler(self.rom if rom is None else rom,
                             self.source if source is None else source, 0x5902D,
                             state_table_contract=NAME, **kwargs)

    def test_complete_scoped_and_generic_state_bound_agree(self):
        # The generic state bound reaches the same four pointers and closure as the
        # reviewed contract, from the closure's own state writes alone.
        generic = slice_handler(self.rom, self.source, 0x5902D)
        self.assertTrue(generic.complete, generic.unresolved)
        self.assertEqual(generic.tables, {0x5907A: [0x5092, 0x50A8, 0x50BE, 0x50D8]})
        self.assertEqual(generic.inferred_tables, {0x5907A: (0, 1, 2, 3)})
        self.assertEqual(generic.entry_states, (0, 1, 2, 3, 4))
        result = self.checked()
        self.assertEqual(generic.instructions, result.instructions)
        self.assertTrue(result.complete, result.unresolved)
        self.assertEqual(result.tables, {0x5907A: [0x5092, 0x50A8, 0x50BE, 0x50D8]})
        self.assertEqual(len(result.instructions), 356)
        self.assertEqual(result.dependencies, {})
        self.assertEqual(self.source.tables[0x5907A], 0x59092)
        self.assertFalse(set(range(0x5907A, 0x59092)) & result.instructions.keys())
        for target in (0x59092, 0x590A8, 0x590BE, 0x590D8):
            self.assertIn(target, result.instructions)
        self.assertIn('entry FFDF in {0,1,2,3,4}', result.state_table_evidence)
        self.assertIn('saved-state restoration', result.state_table_evidence)
        self.assertIn('not native Rust helper verification', result.state_table_evidence)

    def test_every_closure_byte_and_source_witness_rejects_mutation(self):
        # Includes all writers, indirect stores, new-call/bank-write encodings,
        # guards, allocator mask, displacement readers and all declaration bytes.
        for witness in self.contract.footprints:
            for pc in range(witness.start, witness.end):
                with self.subTest(byte=hex(pc)):
                    bad = bytearray(self.rom)
                    bad[pc] ^= 0xFF
                    result = self.checked(rom=bad)
                    self.assertFalse(result.complete)
                    self.assertTrue(any('ROM closure witness mismatch' in x for x in result.unresolved))
            for pc in witness.addresses(self.rom, instruction_size):
                with self.subTest(source=hex(pc)):
                    changed = SourceIndex(dict(self.source.locations), dict(self.source.tables))
                    changed.locations[pc] += ' changed'
                    self.assertFalse(self.checked(source=changed).complete)
                    del changed.locations[pc]
                    result = self.checked(source=changed)
                    self.assertFalse(result.complete)
                    self.assertTrue(any('missing exact source' in x for x in result.unresolved))

    def test_source_extent_entry_bank_and_binding_rejected(self):
        for end in (0x59080, 0x59082, 0x59094):
            source = SourceIndex(dict(self.source.locations), {**self.source.tables, 0x5907A: end})
            self.assertFalse(self.checked(source=source).complete)
        for entry in (0x59032, 0x59079, 0x59136):
            result = slice_handler(self.rom, self.source, entry, state_table_contract=NAME)
            self.assertFalse(result.complete)
        with self.assertRaises(ValueError):
            self.checked(bank=0x15)
        self.assertFalse(self.checked(bindings={0x59136: 'fake.rs::fake'}).complete)
        with self.assertRaisesRegex(ValueError, 'unknown state-table contract'):
            slice_handler(self.rom, self.source, 0x5902D, state_table_contract='unknown')
        with self.assertRaisesRegex(ValueError, 'cannot be combined'):
            self.checked(entry_contract='ending-text-vblank')
        result = self.checked(budget=1)
        self.assertFalse(result.complete)
        self.assertTrue(any('budget' in x for x in result.unresolved))

    def test_manifest_span_invariants(self):
        for contract in (replace(self.contract, pointers=(0x5907A, 0x59084)),
                         replace(self.contract, displacements=((0x59081, 0x5908A), (0x5908A, 0x59092))),
                         replace(self.contract, dispatch=0x59078),
                         replace(self.contract, displacements=((0x59082, 0x59088), (0x59088, 0x59092))),
                         replace(self.contract, displacements=((0x59082, 0x5908A), (0x5908A, 0x61092)))):
            with self.assertRaises(ValueError):
                contract.validate(self.rom, self.source, 0x5902D, 0x16, {}, instruction_size)

    def test_unexpected_reachable_call_and_state_writer_rejected(self):
        for data in (bytes.fromhex('CD 00 40'), bytes.fromhex('E0 DF'), bytes.fromhex('EA 00 21')):
            rom = bytearray(self.rom)
            rom[0x590F5:0x590F5 + len(data)] = data
            self.assertFalse(self.checked(rom=rom).complete)

    def test_brief_and_cli_scoping(self):
        brief, complete = handler_task.make_brief(ROOT, self.decomp, 0x5902D, state_table_contract=NAME)
        self.assertTrue(complete)
        self.assertIn('**Slice status: COMPLETE** under state-table contract', brief)
        self.assertIn('(entry FFDF in {0,1,2,3,4})', brief)
        self.assertIn('## Checked mixed state-table contract', brief)
        self.assertIn('## Existing helper dependencies supplied by the supervisor\n\n- None.', brief)
        with tempfile.TemporaryDirectory() as d:
            for flags, status in (([], 0), (['--state-table-contract', NAME], 0),
                                  (['--state-table-contract', NAME, '--max-states', '1'], 2)):
                with patch.object(sys, 'argv', ['handler_task', '--addr', '0x5902D', '--out-dir', d] + flags):
                    with redirect_stderr(io.StringIO()):
                        self.assertEqual(handler_task.main(), status)
            for flags, status in (([], 0), (['--state-table-contract', NAME], 0)):
                run = subprocess.run([sys.executable, str(ROOT / 'tools/handler_task.py'),
                                      '--addr', '0x5902D', '--out-dir', d] + flags,
                                     capture_output=True, text=True)
                self.assertEqual(run.returncode, status, run.stderr)


if __name__ == '__main__':
    unittest.main()
