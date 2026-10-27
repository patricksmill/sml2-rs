"""Deterministic tool contracts; no emulator, ROM fixture, or new captures needed."""
from __future__ import annotations

from contextlib import redirect_stdout, redirect_stderr
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asm_slice import SourceIndex, bank_cpu, rom_address, slice_handler
import handler_coverage as coverage
import handler_task
import dispatch_prompt
import replay_gate
try:
    from ending_interrupt_contracts import CONTRACTS
except ImportError:
    from tools.ending_interrupt_contracts import CONTRACTS

ROOT = Path(__file__).resolve().parents[2]


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.rom = bytearray(0x4000)
        self.rom[0x200] = 0xC9
        self.objects = {'anim_profiles_a': [{'handler': 0x100}],
                        'anim_profiles_b': [{'handler': 0x200}],
                        'boss_anim_handlers': [0x300, 0x301]}
        self.write('extracted/objects.json', self.objects)
        self.write('extracted/levels/index.json', {'levels': [{'level_id': i} for i in range(32)]})
        for i in range(32):
            self.write(f'extracted/levels/level{i+1:02d}.manifest.json',
                       {'spawns': {'records': [{'anim_index': 0}]}})
        self.source = self.root / 'crates/sml2-core/src/objects'
        self.source.mkdir(parents=True)
        (self.source / 'dispatch.rs').write_text('''match handler {
0x00100 => self.walk(st),
0x00200 => {}
0x00300 => self.boss(st),
// 0x00301 => self.boss(st),
h => self.note_unported(h),
}''')
        (self.source / 'family.rs').write_text('fn walk() {}\nfn boss() {}')

    def write(self, path, data):
        p = self.root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(data))

    def test_mode_boundary_and_separate_boss_accounting(self):
        report = coverage.audit(self.root, self.rom)
        self.assertEqual([coverage.sprite_map_index(i) for i in (9, 10, 11)], [0, 1, 1])
        self.assertEqual(report['by_level']['level10'], [0x100])
        self.assertEqual(report['by_level']['level11'], [0x200])
        self.assertEqual(report['by_level']['level12'], [0x200])
        self.assertEqual(report['ported_spawn_reachable'], [0x100, 0x200])
        self.assertEqual(report['boss_table']['unported'], [0x301])
        self.assertEqual(report['level_tables']['rom_no_ops'], [0x200])
        self.assertEqual(report['ported_match_arms'], 3)

    def test_family_subfolder_methods_count_as_implemented(self):
        family = self.source / 'wario'
        family.mkdir()
        (self.source / 'family.rs').replace(family / 'form2.rs')
        report = coverage.audit(self.root, self.rom)
        self.assertEqual(report['ported_spawn_reachable'], [0x100, 0x200])
        self.assertEqual(report['boss_table']['implemented_bodies'], [0x300])

    def test_bad_profile_and_negative_index_fail(self):
        for anim in (-1, 1):
            with self.subTest(anim=anim):
                self.write('extracted/levels/level01.manifest.json',
                           {'spawns': {'records': [{'anim_index': anim}]}})
                with self.assertRaisesRegex(ValueError, 'invalid anim'):
                    coverage.audit(self.root, self.rom)

    def test_missing_profile_is_not_skipped(self):
        self.objects['anim_profiles_b'] = [{}]
        self.write('extracted/objects.json', self.objects)
        with self.assertRaises(KeyError):
            coverage.audit(self.root, self.rom)

    def test_missing_source_and_missing_assets_fail(self):
        (self.source / 'dispatch.rs').unlink()
        with self.assertRaises(OSError):
            coverage.audit(self.root, self.rom)
        (self.root / 'extracted/objects.json').unlink()
        with self.assertRaises(OSError):
            coverage.audit(self.root, self.rom)

    def test_no_op_requires_rom_ret_and_body_requires_method(self):
        with self.assertRaisesRegex(ValueError, 'not a ROM ret'):
            coverage.dispatch_entries('0x00100 => {}\nh => self.note_unported(h),', set(), self.rom)
        with self.assertRaisesRegex(ValueError, 'missing implementation'):
            coverage.dispatch_entries('0x00100 => self.absent(st),\nh => self.note_unported(h),', set(), self.rom)

    def test_missing_level_is_an_error(self):
        self.write('extracted/levels/index.json', {'levels': [{'level_id': 0}]})
        with self.assertRaisesRegex(ValueError, 'all 32'):
            coverage.audit(self.root, self.rom)

    def test_cli_distinguishes_pending_from_invalid_and_preserves_report(self):
        rom_path = self.root / 'baserom.gb'
        rom_path.write_bytes(self.rom)
        with patch.object(sys, 'argv', ['coverage', '--root', str(self.root), '--rom', str(rom_path)]):
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                self.assertEqual(coverage.main(), 1)
                report = (self.root / 'handler_coverage.json').read_text()
                (self.source / 'dispatch.rs').unlink()
                self.assertEqual(coverage.main(), 2)
                self.assertEqual((self.root / 'handler_coverage.json').read_text(), report)


class SliceTests(unittest.TestCase):
    def setUp(self):
        self.rom = bytearray(0x80000)
        self.source = SourceIndex({}, {})

    def code(self, addr, data):
        self.rom[addr:addr+len(data)] = bytes(data)
        # Test annotations are independent of traversal: map each supplied byte.
        for pc in range(addr, addr+len(data)):
            self.source.locations[pc] = f'fixture.asm:{pc} instruction'

    def test_address_mapping_uses_hex_bank_16(self):
        self.assertEqual(bank_cpu(0x58201), (0x16, 0x4201))
        self.assertEqual(rom_address(0x16, 0x4201), 0x58201)
        self.assertEqual(rom_address(0x16, 0x0100), 0x0100)
        with self.assertRaises(ValueError):
            rom_address(0, 0x4201)

    def test_all_jump_table_arms_and_conditional_paths(self):
        self.code(0x58000, [0xEF])
        self.rom[0x58001:0x58005] = bytes([0, 0x42, 0, 0x43])
        self.source.tables[0x58001] = 0x58005
        self.code(0x58200, [0x20, 1, 0xC9, 0xC9])
        self.code(0x58300, [0xC9])
        result = slice_handler(self.rom, self.source, 0x58000)
        self.assertTrue(result.complete, result.unresolved)
        self.assertEqual(set(result.instructions), {0x58000, 0x58200, 0x58202, 0x58203, 0x58300})
        self.assertEqual(result.tables[0x58001], [0x4200, 0x4300])

    def test_cross_bank_trampoline_restores_bank_before_return(self):
        self.code(0x58000, [0xCD, 0, 1, 0xC9])
        self.code(0x100, [0x3E, 2, 0xEA, 0, 0x21, 0xCD, 0, 0x40,
                          0x3E, 0x16, 0xEA, 0, 0x21, 0xC9])
        self.code(0x8000, [0xC9])
        result = slice_handler(self.rom, self.source, 0x58000)
        self.assertTrue(result.complete, result.unresolved)
        self.assertIn(0x8000, result.instructions)
        self.assertIn(0x58003, result.instructions)

    def test_push_hl_ret_target_is_resolved(self):
        self.code(0x58000, [0x21, 0, 0x42, 0xE5, 0xC9])
        self.code(0x58200, [0xC9])
        result = slice_handler(self.rom, self.source, 0x58000)
        self.assertTrue(result.complete, result.unresolved)
        self.assertIn(0x58200, result.instructions)

    def test_computed_jump_and_budget_are_explicitly_incomplete(self):
        self.code(0x58000, [0xE9])
        self.assertFalse(slice_handler(self.rom, self.source, 0x58000).complete)
        self.code(0x58000, [0, 0, 0xC9])
        result = slice_handler(self.rom, self.source, 0x58000, budget=1)
        self.assertFalse(result.complete)
        self.assertTrue(any('budget' in reason for reason in result.unresolved))

    def test_unresolved_table_and_unknown_bank_write_fail(self):
        for data in ([0xEF], [0xEA, 0, 0x21]):
            self.code(0x58000, data)
            self.assertFalse(slice_handler(self.rom, self.source, 0x58000).complete)

    def state_machine(self, arm2):
        # ldh a,[$FFDF] / and a / jr nz dispatch; state 0 initializes to 1.
        self.code(0x58000, [0xF0, 0xDF, 0xA7, 0x20, 0x05, 0x3E, 0x01, 0xE0, 0xDF, 0xC9,
                            0xF0, 0xDF, 0x3D, 0xEF])
        self.rom[0x5800E:0x58014] = bytes([0x00, 0x41, 0x00, 0x42, 0xFF, 0xFF])
        self.code(0x58100, [0x3E, 0x02, 0xE0, 0xDF, 0xC9])  # state 1 -> 2
        self.code(0x58200, arm2)
        return slice_handler(self.rom, self.source, 0x58000)

    def test_undeclared_state_table_is_bounded_by_written_states(self):
        result = self.state_machine([0xF0, 0xDF, 0xEE, 0x03, 0xE0, 0xDF, 0xC9])  # 2 -> 2^3
        self.assertTrue(result.complete, result.unresolved)
        self.assertEqual(result.tables, {0x5800E: [0x4100, 0x4200]})
        self.assertEqual(result.inferred_tables, {0x5800E: (0, 1)})
        self.assertEqual(result.entry_states, (0, 1, 2))

    def test_non_constant_state_write_leaves_the_bound_unresolved(self):
        result = self.state_machine([0xFA, 0x00, 0xC0, 0xE0, 0xDF, 0xC9])  # ld a,[$C000]
        self.assertFalse(result.complete)
        self.assertTrue(any('unbounded' in r for r in result.unresolved), result.unresolved)

    def test_unguarded_state_zero_wraps_and_stays_unresolved(self):
        self.code(0x58000, [0xF0, 0xDF, 0x3D, 0xEF])
        self.rom[0x58004:0x58006] = bytes([0x00, 0x41])
        self.code(0x58100, [0xC9])
        result = slice_handler(self.rom, self.source, 0x58000)
        self.assertFalse(result.complete)
        self.assertTrue(any('wraps' in r for r in result.unresolved), result.unresolved)

    def test_state_bound_never_replaces_a_complete_declared_slice(self):
        self.code(0x58000, [0xF0, 0xDF, 0xEF])
        self.rom[0x58003:0x58007] = bytes([0x00, 0x41, 0x00, 0x42])
        self.source.tables[0x58003] = 0x58007
        self.code(0x58100, [0xC9])
        self.code(0x58200, [0xC9])
        result = slice_handler(self.rom, self.source, 0x58000)
        self.assertTrue(result.complete, result.unresolved)
        self.assertEqual(result.tables, {0x58003: [0x4100, 0x4200]})  # state 0 alone would give one arm
        self.assertEqual(result.inferred_tables, {})

    def test_reviewed_helper_contract_bounds_slice(self):
        self.code(0x58000, [0xCD, 0, 0x42, 0xC9])
        result = slice_handler(self.rom, self.source, 0x58000,
                               bindings={0x58200: 'helpers.rs::already_ported'})
        self.assertTrue(result.complete, result.unresolved)
        self.assertEqual(result.dependencies, {0x58200: 'helpers.rs::already_ported'})

    def test_source_index_finds_unlabelled_entry_exactly(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'home.asm').write_text('SECTION "bank00", ROM0\n')
            (root / 'main.asm').write_text('''SECTION "bank16", ROMX, BANK[$16]
UnknownCall_0x58000:
    ret
    call UnknownCall_0x58010
    ret
UnknownCall_0x58010:
    ret
''')
            self.code(0x58000, [0xC9, 0xCD, 0x10, 0x40, 0xC9])
            self.code(0x58010, [0xC9])
            source = SourceIndex.load(root, self.rom)
            result = slice_handler(self.rom, source, 0x58001)
            self.assertTrue(result.complete, result.unresolved)
            self.assertIn('main.asm:4 call', result.instructions[0x58001])
            self.assertNotIn(0x58000, result.instructions)

    def test_boss_anim_lookup_and_range(self):
        self.assertEqual(handler_task.handler_for_anim({'boss_anim_handlers': [0x9322]}, 0, 'boss'), 0x9322)
        with self.assertRaises(ValueError):
            handler_task.handler_for_anim({'boss_anim_handlers': [0x9322]}, -1, 'boss')

    def symbolic_table_fixture(self, root, declaration):
        (root / 'home.asm').write_text('SECTION "bank00", ROM0\n')
        (root / 'main.asm').write_text('''SECTION "bank02", ROMX, BANK[$02]
UnknownCall_0x8000:
    rst $28
UnknownData_0x8001:
''' + declaration + '''
UnknownJump_0x8010:
    ret
UnknownJump_0x8020:
    ret
''')
        self.code(0x8000, [0xEF, 0x10, 0x40, 0x20, 0x40])
        self.code(0x8010, [0xC9])
        self.code(0x8020, [0xC9])

    def test_symbolic_dw_forward_reference_keeps_exact_table_and_following_code(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self.symbolic_table_fixture(root, '    dw $4010, UnknownJump_0x8020')
            main = root / 'main.asm'
            main.write_text(main.read_text().replace('\nUnknownJump_0x8010:', '\n    ret\nUnknownJump_0x8010:'))
            self.code(0x8005, [0xC9])
            source = SourceIndex.load(root, self.rom)
            self.assertEqual(source.tables[0x8001], 0x8005)
            self.assertIn('dw $4010, UnknownJump_0x8020', source.locations[0x8003])
            self.assertIn('ret', source.locations[0x8005])
            result = slice_handler(self.rom, source, 0x8000)
            self.assertTrue(result.complete, result.unresolved)
            self.assertEqual(result.tables[0x8001], [0x4010, 0x4020])
            self.assertEqual(set(result.instructions), {0x8000, 0x8010, 0x8020})

    def test_symbolic_dw_mismatched_rom_or_unknown_reference_rejects_whole_table(self):
        for bad in ['UnknownJump_0x8030', 'missing', 'UnknownJump_0x8020 + 1']:
            with self.subTest(bad=bad), tempfile.TemporaryDirectory() as d:
                root = Path(d)
                self.symbolic_table_fixture(root, '    dw $4010\n    dw ' + bad)
                source = SourceIndex.load(root, self.rom)
                self.assertNotIn(0x8001, source.tables)
                self.assertFalse(slice_handler(self.rom, source, 0x8000).complete)
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self.symbolic_table_fixture(root, '    dw $4010, UnknownJump_0x8020')
            self.rom[0x8003] = 0x21
            source = SourceIndex.load(root, self.rom)
            self.assertNotIn(0x8001, source.tables)
            self.assertFalse(slice_handler(self.rom, source, 0x8000).complete)

    def test_symbolic_dw_supports_symbol_file_addresses(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self.symbolic_table_fixture(root, '    dw $4010, NamedTarget')
            (root / 'sml2.sym').write_text('02:4020 NamedTarget\n')
            source = SourceIndex.load(root, self.rom)
            self.assertEqual(source.tables[0x8001], 0x8005)
            self.assertTrue(slice_handler(self.rom, source, 0x8000).complete)

    def test_source_local_label_does_not_consume_instruction_bytes(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'home.asm').write_text('SECTION "bank00", ROM0\n')
            (root / 'main.asm').write_text('''SECTION "bank16", ROMX, BANK[$16]
UnknownCall_0x58000:
    nop
.next:
    ret
''')
            self.code(0x58000, [0, 0xC9])
            source = SourceIndex.load(root, self.rom)
            self.assertIn('main.asm:5 ret', source.locations[0x58001])
            self.assertTrue(slice_handler(self.rom, source, 0x58000).complete)

    def test_default_ending_slices_remain_incomplete(self):
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if (decomp / 'baserom.gb').is_file():
            rom = (decomp / 'baserom.gb').read_bytes()
            source = SourceIndex.load(decomp, rom)
            res_text = slice_handler(rom, source, 0x68572)
            self.assertFalse(res_text.complete)
            res_transfer = slice_handler(rom, source, 0x26D7)
            self.assertFalse(res_transfer.complete)

            res_budget = slice_handler(rom, source, 0x68572, entry_contract='ending-text-vblank', budget=1)
            self.assertFalse(res_budget.complete)
            self.assertTrue(any('budget' in reason for reason in res_budget.unresolved))

    def test_positive_ending_text_vblank_contract(self):
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if not (decomp / 'baserom.gb').is_file():
            self.skipTest('baserom.gb not found')
        rom = (decomp / 'baserom.gb').read_bytes()
        source = SourceIndex.load(decomp, rom)
        res = slice_handler(rom, source, 0x68572, entry_contract='ending-text-vblank')
        self.assertTrue(res.complete, res.unresolved)
        self.assertEqual(res.dependencies, {})
        self.assertEqual(res.entry_contract, 'ending-text-vblank')
        self.assertEqual(res.interrupt_exits, {0x685D5})
        for loc in (0x685D5, 0x685D8, 0x685DB, 0x685DD, 0x685E0, 0x685E1, 0x685E2, 0x685E3, 0x685E4):
            self.assertIn(loc, res.instructions)
        self.assertIn(0x68591, res.instructions)
        self.assertIn(0x68593, res.instructions)
        self.assertIn(0x685A5, res.instructions)
        self.assertIn(0x685C9, res.instructions)
        self.assertIn(0x685CB, res.instructions)

    def test_positive_ending_transfer_vblank_contract(self):
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if not (decomp / 'baserom.gb').is_file():
            self.skipTest('baserom.gb not found')
        rom = (decomp / 'baserom.gb').read_bytes()
        source = SourceIndex.load(decomp, rom)
        res = slice_handler(rom, source, 0x26D7, entry_contract='ending-transfer-vblank')
        self.assertTrue(res.complete, res.unresolved)
        self.assertEqual(res.dependencies, {})
        self.assertEqual(res.entry_contract, 'ending-transfer-vblank')
        self.assertEqual(res.interrupt_exits, {0x2719})
        self.assertIn(0x26D7, res.instructions)
        self.assertIn(0x26DA, res.instructions)
        self.assertIn(0x2716, res.instructions)
        self.assertNotIn(0x2089, res.instructions)
        self.assertIn(0x26F6, res.instructions)
        self.assertIn(0x26EF, res.instructions)
        self.assertIn(0x26F8, res.instructions)
        self.assertIn(0x270C, res.instructions)
        self.assertIn(0x270E, res.instructions)
        self.assertIn(0x2712, res.instructions)
        for loc in (0x2719, 0x271B, 0x271D, 0x2720, 0x2723, 0x2724, 0x2725, 0x2726, 0x2727):
            self.assertIn(loc, res.instructions)

    def test_ending_contract_witness_tampering_and_invariants(self):
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if not (decomp / 'baserom.gb').is_file():
            self.skipTest('baserom.gb not found')
        clean_rom = (decomp / 'baserom.gb').read_bytes()
        source = SourceIndex.load(decomp, clean_rom)

        tamper_cases = [
            (0x154, 'wrong save order at VBlank DI/save'),
            (0x2CA3, 'wrong wrapper bank/store'),
            (0x6806E, 'changed request bank'),
            (0x2712, 'changed state comparison'),
            (0x2716, 'changed call instruction/condition'),
            (0x685D5, 'wrong terminal restore operand'),
            (0x685DD, 'missing frame-ready write'),
            (0x685E0, 'wrong POP order/count'),
            (0x685E4, 'RET instead of RETI'),
        ]
        transfer_addrs = {0x6806E, 0x2712, 0x2716}
        for addr, desc in tamper_cases:
            with self.subTest(tamper=desc, addr=hex(addr)):
                bad_rom = bytearray(clean_rom)
                bad_rom[addr] ^= 0xFF
                contract_name = 'ending-transfer-vblank' if addr in transfer_addrs else 'ending-text-vblank'
                entry = 0x26D7 if contract_name == 'ending-transfer-vblank' else 0x68572
                res = slice_handler(bad_rom, source, entry, entry_contract=contract_name)
                self.assertFalse(res.complete, f'tamper at {hex(addr)} ({desc}) should have failed')

    def test_ending_contract_stack_and_bank_invariants(self):
        contract = CONTRACTS['ending-text-vblank']
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if not (decomp / 'baserom.gb').is_file():
            self.skipTest('baserom.gb not found')
        rom = (decomp / 'baserom.gb').read_bytes()
        source = SourceIndex.load(decomp, rom)
        from asm_slice import instruction_size
        with self.assertRaisesRegex(ValueError, 'unbalanced local push/call stack'):
            contract.terminal_instructions(rom, source, stack=(0x1234,), selected=0x1A, instruction_size=instruction_size)
        with self.assertRaisesRegex(ValueError, 'selected bank contradicts caller-bank fact'):
            contract.terminal_instructions(rom, source, stack=(), selected=0x01, instruction_size=instruction_size)

        with self.assertRaisesRegex(ValueError, 'cannot be combined with helper bindings'):
            contract.validate(rom, source, 0x68572, 0x1A, bindings={0x1234: 'foo'}, instruction_size=instruction_size)

        with self.assertRaisesRegex(ValueError, 'contradicts contract'):
            contract.validate(rom, source, 0x68572, 0x01, bindings={}, instruction_size=instruction_size)
        with self.assertRaisesRegex(ValueError, 'contradicts contract'):
            contract.validate(rom, source, 0x26D7, 0x1A, bindings={}, instruction_size=instruction_size)

        with self.assertRaisesRegex(ValueError, 'unknown entry contract'):
            slice_handler(rom, source, 0x68572, entry_contract='invalid-contract')

    def test_ending_contract_source_locations_checked(self):
        decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
        if not (decomp / 'baserom.gb').is_file():
            self.skipTest('baserom.gb not found')
        rom = (decomp / 'baserom.gb').read_bytes()
        full_source = SourceIndex.load(decomp, rom)
        incomplete_source = SourceIndex(dict(full_source.locations), dict(full_source.tables))
        del incomplete_source.locations[0x68572]
        res = slice_handler(rom, incomplete_source, 0x68572, entry_contract='ending-text-vblank')
        self.assertFalse(res.complete)
        self.assertTrue(any('missing exact source' in r for r in res.unresolved))

    def test_handler_task_entry_contract_output_and_cli(self):
        with tempfile.TemporaryDirectory() as d:
            out_dir = Path(d)
            decomp = Path(os.environ.get('SML2_DECOMP_ROOT', ROOT.parent / 'sml2decomp'))
            if not (decomp / 'baserom.gb').is_file():
                self.skipTest('baserom.gb not found')
            brief, complete = handler_task.make_brief(
                ROOT, decomp, 0x68572, entry_contract='ending-text-vblank')
            self.assertTrue(complete)
            self.assertIn('**Slice status: COMPLETE** under entry contract `ending-text-vblank`', brief)
            self.assertIn('## Checked entry and interrupt-return contract', brief)
            self.assertIn('Structurally checked interrupt terminal entries: 0x685D5', brief)
            self.assertIn('## Existing helper dependencies supplied by the supervisor\n\n- None.', brief)

            brief_t, complete_t = handler_task.make_brief(
                ROOT, decomp, 0x26D7, entry_contract='ending-transfer-vblank')
            self.assertTrue(complete_t)
            self.assertIn('**Slice status: COMPLETE** under entry contract `ending-transfer-vblank`', brief_t)
            self.assertIn('Structurally checked interrupt terminal entries: 0x02719', brief_t)

            with patch.object(sys, 'argv', ['handler_task', '--addr', '0x68572',
                                            '--entry-contract', 'ending-text-vblank',
                                            '--out-dir', str(out_dir)]):
                with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                    self.assertEqual(handler_task.main(), 0)

            with patch.object(sys, 'argv', ['handler_task', '--addr', '0x68572',
                                            '--out-dir', str(out_dir)]):
                with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                    self.assertEqual(handler_task.main(), 2)


class GateTests(unittest.TestCase):
    def test_every_retained_capture_is_gated_once(self):
        captures = {path.stem for path in (ROOT / 'captures').glob('*.json')}
        self.assertEqual(set(replay_gate.CAPTURES), captures)
        self.assertEqual(len(replay_gate.CAPTURES), len(captures))

    @staticmethod
    def output(fields=None):
        if fields is None:
            fields = {k: 0 for k in replay_gate.LEGACY}
        return 'mismatches: ' + ' '.join(f'{k}={v}' for k, v in fields.items()) + '\n'

    def test_regular_zero_and_exact_legacy_pass(self):
        replay_gate.validate('traj5', self.output(), 0)
        replay_gate.validate('traj4_ff0', self.output(replay_gate.LEGACY), 0)

    def test_regression_in_middle_of_summary_fails(self):
        values = {k: 0 for k in replay_gate.LEGACY}
        values['camx'] = 1
        with self.assertRaises(ValueError):
            replay_gate.validate('traj5', self.output(values), 0)

    def test_legacy_counts_fields_and_notes_cannot_change(self):
        for field, value in [('y', 54), ('x', 1), ('extra', 0)]:
            values = dict(replay_gate.LEGACY, **{field: value})
            with self.assertRaises(ValueError):
                replay_gate.validate('traj4_ff0', self.output(values), 0)
        with self.assertRaises(ValueError):
            replay_gate.validate('traj4_ff0', self.output(replay_gate.LEGACY) + 'note: bad\n', 0)

    def test_failed_runtime_missing_malformed_and_duplicate_summaries_fail(self):
        for output, status in [(self.output(), 1), ('', 0), ('mismatches: x=0\n', 0),
                               (self.output() + 'note: gap\n', 0),
                               (self.output() + self.output(), 0),
                               (self.output().rstrip() + ' x=0\n', 0),
                               (self.output().rstrip() + ' y=-1\n', 0)]:
            with self.subTest(output=output, status=status), self.assertRaises(ValueError):
                replay_gate.validate('traj5', output, status)

    def test_build_failure_stops_before_any_replays(self):
        with patch.object(replay_gate.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, 'build failed\n')) as run:
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                self.assertEqual(replay_gate.main(), 1)
            self.assertEqual(run.call_count, 1)

    def test_main_runtime_failure_is_nonzero_and_level19_is_selected(self):
        build = subprocess.CompletedProcess([], 0, '')
        runtime = subprocess.CompletedProcess([], 7, self.output())
        with patch.object(replay_gate, 'CAPTURES', ('traj19',)), patch.object(replay_gate.subprocess, 'run', side_effect=[build, runtime]) as run:
            with redirect_stdout(io.StringIO()):
                self.assertEqual(replay_gate.main(), 1)
            self.assertEqual(run.call_args.args[0][1:3], ['--level', '19'])


class PromptTests(unittest.TestCase):
    def test_handler_prompt_requires_native_logic_and_bounded_adapter(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'tasks/handlers').mkdir(parents=True)
            (root / 'tasks/handlers/0x58201.md').write_text('**Slice status: COMPLETE**')
            with patch.object(dispatch_prompt, 'ROOT', root), patch.object(dispatch_prompt, 'run_handler_task'):
                prompt = dispatch_prompt.handler_prompt(0x58201, module='crates/sml2-core/src/objects/enemies/walker.rs')
            self.assertIn('docs/idiomatic-rust-porting.md', prompt)
            self.assertIn('crates/sml2-core/src/actor.rs', prompt)
            self.assertIn('objects/enemies/walker.rs', prompt)
            self.assertIn('objects/compat/enemies/walker.rs', prompt)
            self.assertIn('objects/enemies/tests/walker.rs', prompt)
            self.assertIn('objects/mod.rs (module registrations only', prompt)
            self.assertIn('No slot offsets, register names or HRAM scratch', prompt)
            self.assertIn('Actor::from_raw_slot / Actor::to_raw_slot', prompt)
            self.assertNotIn('objects/helpers.rs', prompt)

    def test_handler_prompt_follows_registered_grouped_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folder = root / 'crates/sml2-core/src/objects'
            folder.mkdir(parents=True)
            (folder / 'mod.rs').write_text(
                '#[path = "enemies/hopping.rs"]\nmod handler_0x58848;\n')
            owner = folder / 'enemies'
            owner.mkdir(parents=True)
            (owner / 'hopping.rs').write_text('')
            (folder / 'compat/enemies').mkdir(parents=True)
            (folder / 'compat/enemies/hopping.rs').write_text(
                '#[cfg(test)]\n#[path = "../../enemies/tests/hopping_adapter.rs"]\nmod tests;\n')
            (root / 'tasks/handlers').mkdir(parents=True)
            (root / 'tasks/handlers/0x58848.md').write_text('**Slice status: COMPLETE**')
            with patch.object(dispatch_prompt, 'ROOT', root), patch.object(dispatch_prompt, 'run_handler_task'):
                prompt = dispatch_prompt.handler_prompt(0x58848)
            self.assertIn('objects/enemies/tests/hopping_adapter.rs', prompt)
            self.assertNotIn('behavior_adapter.rs', prompt)

    def test_handler_prompt_follows_named_native_module(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folder = root / 'crates/sml2-core/src/objects'
            folder.mkdir(parents=True)
            (folder / 'mod.rs').write_text(
                '#[path = "enemies/hopping.rs"]\nmod hopping;\n#[path = "compat/enemies/hopping.rs"]\nmod handler_0x58848_adapter;\n')
            owner = folder / 'enemies'
            owner.mkdir(parents=True)
            (owner / 'hopping.rs').write_text('')
            (folder / 'compat/enemies').mkdir(parents=True)
            (folder / 'compat/enemies/hopping.rs').write_text(
                '#[cfg(test)]\n#[path = "../../enemies/tests/hopping_adapter.rs"]\nmod tests;\n')
            (root / 'tasks/handlers').mkdir(parents=True)
            (root / 'tasks/handlers/0x58848.md').write_text('**Slice status: COMPLETE**')
            with patch.object(dispatch_prompt, 'ROOT', root), patch.object(dispatch_prompt, 'run_handler_task'):
                prompt = dispatch_prompt.handler_prompt(0x58848)
            self.assertIn('objects/enemies/tests/hopping_adapter.rs', prompt)
            self.assertNotIn('behavior_adapter.rs', prompt)

    def test_family_prompt_registers_owning_module_and_rejects_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'tasks/handlers').mkdir(parents=True)
            (root / 'tasks/handlers/0x0AD93.md').write_text('**Slice status: COMPLETE**')
            with patch.object(dispatch_prompt, 'ROOT', root), patch.object(dispatch_prompt, 'run_handler_task'):
                prompt = dispatch_prompt.handler_prompt(
                    0xAD93, module='crates/sml2-core/src/objects/bosses/wario/form3.rs')
                self.assertIn('objects/bosses/wario/form3_adapter.rs', prompt)
                self.assertIn('objects/bosses/wario/mod.rs (module registrations only', prompt)
                self.assertNotIn('objects/mod.rs (module registrations only', prompt)
                for module in ['crates/sml2-core/src/objects/../escape.rs',
                               'crates/sml2-core/src/objects/bosses/wario/mod.rs']:
                    with self.assertRaises(ValueError):
                        dispatch_prompt.handler_prompt(0xAD93, module=module)

    def test_new_handler_requires_a_descriptive_module(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(dispatch_prompt, 'ROOT', Path(directory)):
                with self.assertRaisesRegex(ValueError, 'descriptive native file'):
                    dispatch_prompt.handler_prompt(0x58848)

    def test_queue_prompt_uses_explicit_scope_for_current_items(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'PORT_QUEUE.json').write_text(json.dumps({'queue': [
                {'id': 'current', 'title': 'Current work', 'asm_refs': ['home:$1234']}
            ]}))
            (root / 'behavior.rs').write_text('// native behavior')
            with patch.object(dispatch_prompt, 'ROOT', root):
                output = io.StringIO()
                with patch.object(sys, 'argv', ['dispatch_prompt', '--queue', 'current',
                                                '--file', 'behavior.rs']), redirect_stdout(output):
                    dispatch_prompt.main()
                self.assertIn('Current work', output.getvalue())
                self.assertIn('home:$1234', output.getvalue())
                self.assertIn('Touch ONLY:\n  - behavior.rs', output.getvalue())
                with self.assertRaisesRegex(ValueError, 'explicit scope'):
                    dispatch_prompt.queue_prompt('current')
                with self.assertRaisesRegex(SystemExit, 'non-pending'):
                    dispatch_prompt.queue_prompt('completed', ['behavior.rs'])

    def test_queue_scope_rejects_missing_and_outside_files(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as outside:
            root = Path(directory)
            (root / 'PORT_QUEUE.json').write_text(json.dumps({'queue': [
                {'id': 'current', 'title': 'Current work'}
            ]}))
            external = Path(outside) / 'outside.rs'
            external.write_text('// outside the repository')
            (root / 'linked.rs').symlink_to(external)
            with patch.object(dispatch_prompt, 'ROOT', root):
                for file in [str(external), '../outside.rs', 'linked.rs']:
                    with self.subTest(file=file), self.assertRaisesRegex(ValueError, 'inside the repository'):
                        dispatch_prompt.queue_prompt('current', [file])
                with self.assertRaisesRegex(ValueError, 'missing task file'):
                    dispatch_prompt.queue_prompt('current', ['missing.rs'])

    def test_object_queue_scope_keeps_engine_integration_tests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'PORT_QUEUE.json').write_text(json.dumps({'queue': [
                {'id': 'current', 'title': 'Current work'}
            ]}))
            behavior = 'crates/sml2-core/src/objects/items/coin.rs'
            engine = 'crates/sml2-core/src/objects/tests/engine.rs'
            for file in [behavior, engine]:
                (root / file).parent.mkdir(parents=True, exist_ok=True)
                (root / file).touch()
            with patch.object(dispatch_prompt, 'ROOT', root):
                prompt = dispatch_prompt.queue_prompt('current', [behavior])
            self.assertIn(engine, prompt)

    def test_queue_file_option_cannot_silently_change_handler_scope(self):
        with patch.object(sys, 'argv', ['dispatch_prompt', '--addr', '0x58201',
                                        '--file', 'behavior.rs']), redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as result:
                dispatch_prompt.main()
        self.assertEqual(result.exception.code, 2)

    def test_incomplete_slices_cannot_be_dispatched(self):
        with patch.object(dispatch_prompt, 'run_handler_task', side_effect=subprocess.CalledProcessError(2, ['slice'])):
            with self.assertRaises(subprocess.CalledProcessError):
                dispatch_prompt.handler_prompt(0x58201, module='crates/sml2-core/src/objects/enemies/future_enemy.rs')

    def test_prompt_rejects_arbitrary_modules(self):
        with self.assertRaises(ValueError):
            dispatch_prompt.handler_prompt(0x58201, module='crates/sml2-runtime/src/main.rs')


if __name__ == '__main__':
    unittest.main()
