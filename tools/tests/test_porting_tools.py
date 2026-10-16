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


class GateTests(unittest.TestCase):
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
    def test_current_queue_files_exist(self):
        for files in dispatch_prompt.QUEUE_FILES.values():
            for file in files:
                self.assertTrue((dispatch_prompt.ROOT / file).is_file(), file)

    def test_incomplete_slices_cannot_be_dispatched(self):
        with patch.object(dispatch_prompt, 'run_handler_task', side_effect=subprocess.CalledProcessError(2, ['slice'])):
            with self.assertRaises(subprocess.CalledProcessError):
                dispatch_prompt.handler_prompt(0x58201)

    def test_prompt_rejects_arbitrary_modules(self):
        with self.assertRaises(ValueError):
            dispatch_prompt.handler_prompt(0x58201, module='crates/sml2-runtime/src/main.rs')


if __name__ == '__main__':
    unittest.main()
