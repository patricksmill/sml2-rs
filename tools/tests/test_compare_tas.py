"""Comparison bridge protocol, button delivery, and logic phase selection."""
import argparse
from contextlib import redirect_stderr, redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from compare_tas import (apply_buttons, logic_phases, read_state,
                         check_divergence, comparison_threshold, rom_keys_mask,
                         step_masks, completed_sample, frame_read, frame_timing)
from compare_tas import boss_div_field, BOSS_DIV_READ_HOOKS
import compare_tas


class TraceTests(unittest.TestCase):
    def test_inclusive_ranges_require_two_ordered_nonnegative_endpoints(self):
        self.assertEqual(compare_tas.trace_range('0:0'), (0, 0))
        self.assertEqual(compare_tas.trace_range('12:34'), (12, 34))
        for value in ('', '5', '1:2:3', 'x:y', ':4', '4:', '-1:4', '9:1'):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                compare_tas.trace_range(value)

    def test_invalid_ranges_exit_through_argparse_before_reading_files(self):
        for value in ('5', '1:2:3', 'x:y', '-1:4', '9:1'):
            error = io.StringIO()
            with self.subTest(value=value), patch.object(sys, 'argv', [
                'compare_tas.py', 'unused.json', '--rom', 'unused.gb', f'--trace={value}'
            ]), patch.object(Path, 'read_text') as read, redirect_stderr(error):
                with self.assertRaises(SystemExit) as exited:
                    compare_tas.main()
                self.assertEqual(exited.exception.code, 2)
                read.assert_not_called()
                self.assertIn('error: argument --trace:', error.getvalue())

    def test_trace_uses_completed_snapshot_velocities_and_optional_native_state(self):
        memory = dict.fromkeys(compare_tas.SNAPSHOT_ADDRS, 0)
        memory.update({0xFF9B: 4, 0xFFC2: 34, 0xFFC0: 212, 0xFFC1: 1,
                       0xA200: 0x80, 0xA201: 0x18, 0xFFF0: 3, 0xAF08: 1})
        values = {address: memory[address] for address in compare_tas.SNAPSHOT_ADDRS}
        memory.update({0xA200: 0xFF, 0xA201: 0x5F})
        port = dict(x=34, y=468, hvel=0x80, vvel=0x18, fff0=3, update_phase=1)
        sample, (paired, _) = completed_sample(
            memory, [('d', values), ('u', None)], [(port, 5), (None, 6)])
        self.assertEqual(compare_tas.trace_line(12, 33, sample, paired),
                         '12 right+b    ROM st=04 x=34 y=468 hv=80 vv=18 fff0=3 up=1'
                         ' | port x=34 y=468 hv=80 vv=18 fff0=3 up=1')
        self.assertTrue(compare_tas.trace_line(12, 0, sample, None).endswith(' | port -'))

    def test_trace_without_phase_lock_or_divergence_requests_state_and_prints_range(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rom = root / 'test.gb'
            rom.write_bytes(b'trace protocol fixture')
            movie = root / 'movie.json'
            movie.write_text(json.dumps(dict(format='sml2-tas-v1', frames=[0, 33, 0],
                                             rom_sha1=hashlib.sha1(rom.read_bytes()).hexdigest())))
            memory = bytearray(65536)
            memory[0xFF9B] = 4
            memory[0xFFC0:0xFFC2] = (468).to_bytes(2, 'little')
            memory[0xFFC2] = 34
            memory[0xA200:0xA202] = bytes([0x80, 0x18])
            emulator = Mock(memory=memory)

            def tick(*_):
                memory[0xFFC2] += 1
                return True

            emulator.tick.side_effect = tick
            bridge = Mock(stdin=io.BytesIO(), stdout=io.BytesIO())
            states = [None] + [dict(x=x, y=468, hvel=0x80, vvel=0x18) for x in (35, 36, 37)]
            output = io.StringIO()
            with patch.object(sys, 'argv', [
                'compare_tas.py', str(movie), '--rom', str(rom), '--headless',
                '--no-phase-lock', '--no-diverge', '--trace', '1:1'
            ]), patch.dict(sys.modules, {'pyboy': SimpleNamespace(PyBoy=Mock(return_value=emulator))}), \
                    patch.object(compare_tas.subprocess, 'Popen', return_value=bridge) as popen, \
                    patch.object(compare_tas, 'read_state', side_effect=states), \
                    patch.object(compare_tas, 'read_frame'), redirect_stdout(output):
                compare_tas.main()
            self.assertIn('--state', popen.call_args.args[0])
            rows = [line for line in output.getvalue().splitlines() if line[:1].isdigit()]
            self.assertEqual(len(rows), 1)
            self.assertTrue(rows[0].startswith('1 right+b '))
            self.assertIn('ROM st=04 x=36 y=468 hv=80 vv=18', rows[0])
            self.assertIn('| port x=36 y=468 hv=80 vv=18', rows[0])
            emulator.stop.assert_called_once_with(save=False)


class BossDividerTests(unittest.TestCase):
    def test_decisions_stay_with_their_gameplay_update(self):
        reads = [[], [0], [255]]
        self.assertEqual(boss_div_field(reads, 0), '')
        self.assertEqual(boss_div_field(reads, 1), ' boss_div=0')
        self.assertEqual(boss_div_field(reads, 2), ' boss_div=255')
        self.assertEqual(boss_div_field(reads, 3), '')
        with self.assertRaises(RuntimeError):
            boss_div_field([[1, 2]], 0)

    def test_hooks_follow_the_rom_divider_read_instructions(self):
        rom = Path(__file__).resolve().parents[2] / '../sml2decomp/baserom.gb'
        data = rom.read_bytes()
        for bank, cpu in BOSS_DIV_READ_HOOKS:
            address = bank * 0x4000 + cpu - 0x4000
            self.assertEqual(data[address - 3:address], bytes.fromhex('FA 04 FF'))

    def test_tatanga_volley_divider_reads_are_all_hooked(self):
        rom = Path(__file__).resolve().parents[2] / '../sml2decomp/baserom.gb'
        data = rom.read_bytes()
        # Boss anim 16 ($A0B9..$A27D) reads DIV at the end of each sweep.
        # Discover both read instructions independently of the hook list.
        start, end = 0xA0B9, 0xA27E
        reads = [address for address in range(start, end - 2)
                 if data[address:address + 3] == bytes.fromhex('FA 04 FF')]
        self.assertEqual(len(reads), 2)
        for address in reads:
            bank = address // 0x4000
            after_read = 0x4000 + address % 0x4000 + 3
            self.assertIn((bank, after_read), BOSS_DIV_READ_HOOKS)


class BridgeStateTests(unittest.TestCase):
    def test_extended_state_leaves_pixels_and_next_frame_aligned(self):
        pixels = bytes(range(256)) * 90  # 160 * 144 grayscale pixels
        pipe = io.BytesIO(b'STATE 0 34 468 80 448 80 18 1 0\n' + pixels
                          + b'STATE 1 35 469 81 449 ff 0a 0 7\n')
        self.assertEqual(read_state(pipe), {
            'level': 0, 'x': 34, 'y': 468, 'cam_x': 80, 'cam_y': 448,
            'hvel': 0x80, 'vvel': 0x18, 'on_ground': 1, 'phase': 0,
        })
        self.assertEqual(pipe.read(len(pixels)), pixels)
        self.assertEqual(read_state(pipe), {
            'level': 1, 'x': 35, 'y': 469, 'cam_x': 81, 'cam_y': 449,
            'hvel': 0xFF, 'vvel': 0x0A, 'on_ground': 0, 'phase': 7,
        })

    def test_debug_fields_share_the_state_line_and_preserve_pixel_boundary(self):
        pixels = bytes(range(256)) * 90
        pipe = io.BytesIO(b'STATE 10 498 164 499 160 00 00 1 2 '
                          b'step=8102 fff0=174 update_phase=0 standing=1 scene=17 owns=1 pc=24700 '
                          b'boss_x=24 boss_state=4 door_phase=3 door_parity=1 '
                          b'platform_sprite=19 tile_anim=0 writes=12\n'
                          + pixels + b'STATE - - - - -\n' + pixels)
        state = read_state(pipe)
        self.assertEqual(state['step'], 8102)
        self.assertEqual(state['fff0'], 174)
        self.assertEqual(state['update_phase'], 0)
        self.assertEqual(state['standing'], 1)
        self.assertEqual(state['scene'], 17)
        self.assertEqual(state['owns'], 1)
        self.assertEqual(state['pc'], 24700)
        self.assertEqual(state['boss_x'], 24)
        self.assertEqual(state['door_phase'], 3)
        self.assertEqual(state['platform_sprite'], 19)
        self.assertEqual(state['tile_anim'], 0)
        self.assertEqual(state['writes'], 12)
        self.assertEqual(pipe.read(len(pixels)), pixels)
        self.assertIsNone(read_state(pipe))
        self.assertEqual(pipe.read(len(pixels)), pixels)

    def test_legacy_state(self):
        self.assertEqual(read_state(io.BytesIO(b'STATE 0 34 468 80 448\n')), {
            'level': 0, 'x': 34, 'y': 468, 'cam_x': 80, 'cam_y': 448,
        })

    def test_outside_level(self):
        self.assertIsNone(read_state(io.BytesIO(b'STATE - - - - -\n')))

    def test_invalid_state(self):
        for line in (b'STATE 0 34 468 80\n', b'STATE 0 34 468 80 448 80\n',
                     b'STATE 0 34 468 80 448 zz 18 1 0\n',
                     b'STATE 0 bad 468 80 448\n',
                     b'STATE 0 34 468 80 448 80 18 1 0 step=bad\n',
                     b'STATE 0 34 468 80 448 80 18 1 0 typo=1\n'):
            with self.subTest(line=line), self.assertRaisesRegex(RuntimeError, 'malformed'):
                read_state(io.BytesIO(line))

    def test_unexpected_output(self):
        with self.assertRaisesRegex(RuntimeError, 'unexpected bridge state'):
            read_state(io.BytesIO(b'diagnostic\n'))

    def test_closed_bridge(self):
        with self.assertRaisesRegex(RuntimeError, 'closed unexpectedly'):
            read_state(io.BytesIO())


class PositionComparisonTests(unittest.TestCase):
    def fixture(self, x=402, y=343, cam_x=400, cam_y=300):
        memory = bytearray(65536)
        memory[0xFF9B] = 4
        memory[0xA269] = 10
        for address, value in ((0xFFC2, x), (0xFFC0, y),
                               (0xA227, x), (0xA229, y),
                               (0xFFCA, cam_x), (0xFFC8, cam_y)):
            memory[address] = value & 255
            memory[address + 1] = value >> 8
        native = dict(level=10, x=x, y=y, cam_x=cam_x, cam_y=cam_y)
        return memory, native

    def test_threshold_zero_still_disables_and_exact_is_explicit(self):
        self.assertIsNone(comparison_threshold(0))
        self.assertEqual(comparison_threshold(0, exact=True), 0)
        self.assertEqual(comparison_threshold(4, exact=True), 0)
        self.assertIsNone(comparison_threshold(4, exact=True, disabled=True))

    def test_one_pixel_mismatch_is_exact_only_and_reports_separate_indices(self):
        memory, native = self.fixture()
        native.update(x=401, step=7500)
        self.assertIsNone(check_divergence(7602, memory, native, 4, live=True))
        mismatch = check_divergence(7602, memory, native, 0, live=True)
        self.assertEqual((mismatch['frame'], mismatch['native_step']), (7602, 7500))
        self.assertEqual(mismatch['dx'], -1)
        self.assertIn('position tolerance 0px', mismatch['detail'])
        self.assertIn('compared level/x/y', mismatch['detail'])
        self.assertIn('ROM live HRAM', mismatch['detail'])

    def test_threshold_boundary_and_legacy_native_step_fallback(self):
        memory, native = self.fixture()
        native['x'] -= 4
        self.assertIsNone(check_divergence(5, memory, native, 4))
        native['x'] -= 1
        mismatch = check_divergence(5, memory, native, 4, native_step=7)
        self.assertEqual(mismatch['native_step'], 7)
        self.assertIn('published prior position / current camera', mismatch['detail'])

    def test_equal_modulo_projection_does_not_hide_page_change(self):
        memory, native = self.fixture(x=499, y=676, cam_x=499, cam_y=672)
        native.update(x=498, y=164, cam_y=160)
        self.assertEqual((676 - 672) & 255, (native['y'] - native['cam_y']) & 255)
        self.assertEqual(676 & 255, native['y'] & 255)
        mismatch = check_divergence(8134, memory, native, 4, live=True, native_step=8000)
        self.assertEqual((mismatch['dy'], mismatch['cam_dy']), (-512, -512))
        self.assertIn('world camera ROM (499, 672) vs port (499, 160)', mismatch['detail'])

    def test_camera_is_diagnostic_and_does_not_gate_mixed_boundary_state(self):
        memory, native = self.fixture()
        native['cam_y'] += 512
        self.assertIsNone(check_divergence(1, memory, native, 0))

    def test_live_reads_hram_and_published_mode_reads_saved_position(self):
        memory, native = self.fixture()
        memory[0xFFC2] += 1
        self.assertIsNone(check_divergence(2, memory, native, 0))
        self.assertEqual(check_divergence(2, memory, native, 0, live=True)['dx'], -1)

    def test_entry_without_a_player_update_can_still_be_outside_a_level(self):
        memory, _ = self.fixture()
        self.assertIsNone(check_divergence(1, memory, None, 0, live=True, player_ran=False))
        mismatch = check_divergence(1, memory, None, 0, live=True, native_step=4)
        self.assertEqual(mismatch['native_step'], 4)
        self.assertIn('position tolerance 0px; native update 4', mismatch['detail'])

    def test_level_mismatch_reports_tolerance_and_native_update(self):
        memory, native = self.fixture()
        native.update(level=9, step=99)
        mismatch = check_divergence(103, memory, native, 4, live=True)
        self.assertEqual(mismatch['native_step'], 99)
        self.assertIn('ROM level 10 vs port level 9', mismatch['detail'])
        self.assertIn('position tolerance 4px; native update 99', mismatch['detail'])


class Controller:
    def __init__(self):
        self.held = set()
        self.events = []

    def button_press(self, name):
        self.held.add(name)
        self.events.append(('press', name))

    def button_release(self, name):
        self.held.remove(name)
        self.events.append(('release', name))


class ButtonDeliveryTests(unittest.TestCase):
    def test_alternating_directions_release_previous_direction_and_keep_run(self):
        controller = Controller()
        apply_buttons(controller, 0, 33)
        self.assertEqual(controller.held, {'right', 'b'})
        controller.events.clear()
        apply_buttons(controller, 33, 34)
        self.assertEqual(controller.held, {'left', 'b'})
        self.assertEqual(controller.events, [('release', 'right'), ('press', 'left')])
        controller.events.clear()
        apply_buttons(controller, 34, 34)
        self.assertEqual(controller.events, [])
        apply_buttons(controller, 34, 0)
        self.assertEqual(controller.held, set())

    def test_actual_simultaneous_directions_are_preserved(self):
        controller = Controller()
        apply_buttons(controller, 0, 3)
        self.assertEqual(controller.held, {'right', 'left'})


class LogicPhaseTests(unittest.TestCase):
    def test_player_update_uses_the_phase_sampled_at_entry(self):
        self.assertEqual(logic_phases(4, 0x7C, [5]), [5])

    def test_camera_transition_without_a_player_update_still_steps_once(self):
        self.assertEqual(logic_phases(4, 0x10, [1]), [1])

    def test_every_state4_entry_steps_even_without_the_player_routine(self):
        # The hit (reaches `$0F71`), then the first death-arc entry (does not).
        self.assertEqual(logic_phases(4, 5, [6, 7]), [6, 7])

    def test_blocking_pipe_scene_advances_without_gameplay_hooks(self):
        self.assertEqual(logic_phases(4, 0x10, [], [2]), [2])

    def test_scene_resumes_after_gameplay_in_the_same_hardware_frame(self):
        self.assertEqual(logic_phases(4, 0x10, [5], [6, 7]), [5, 6, 7])

    def test_scene_wait_without_a_return_does_not_advance(self):
        self.assertEqual(logic_phases(4, 7, [], []), [])

    def test_scene_return_phase_wraps_without_a_gameplay_step(self):
        self.assertEqual(logic_phases(4, 7, [], [0]), [0])

    def test_in_level_overrun_does_not_step(self):
        self.assertEqual(logic_phases(4, 0x7C, [], []), [])

    def test_outside_a_level_steps_once_per_hardware_frame(self):
        self.assertEqual(logic_phases(0x0C, 0x20, [], []), [1])

    def test_finished_native_loader_waits_for_the_first_gameplay_dispatch(self):
        # $04F1 stores state $04 and returns without calling $05D5. If the
        # native loader finished on the previous row, this is still no game
        # update: $AF08 and $FFF0 must remain at their loader values.
        for state in (2, 3):
            self.assertEqual(logic_phases(state, 6, [], native_in_level=False), [7])
            self.assertEqual(logic_phases(state, 6, [], native_in_level=True), [])
            self.assertEqual(logic_phases(state, 6, [7], native_in_level=True), [7])
        self.assertEqual(logic_phases(4, 7, [0], native_in_level=True), [0])

    def test_loader_alignment_preserves_scene_resumes_and_pause_updates(self):
        self.assertEqual(logic_phases(3, 6, [], [7], native_in_level=True), [7])
        self.assertEqual(logic_phases(8, 6, [], native_in_level=True), [7])



class StepMaskTests(unittest.TestCase):
    def test_rom_joypad_byte_maps_to_movie_layout(self):
        # hKeysHeld: A=0, B=1, Select=2, Start=3, Right=4, Left=5, Up=6, Down=7.
        self.assertEqual(rom_keys_mask(0b0000_0001), 16)
        self.assertEqual(rom_keys_mask(0b0010_0010), 2 | 32)
        self.assertEqual(rom_keys_mask(0b1100_1100), 8 | 4 | 64 | 128)

    def test_player_update_uses_the_keys_the_rom_read(self):
        # The row adds Left, but the overrunning frame read the previous row (B).
        self.assertEqual(step_masks(2 | 32, [1], [0b0000_0010]), [32])

    def test_a_read_at_the_end_of_a_tick_belongs_to_the_next_frame(self):
        # Level 10 map entry: A read at LY 149, acted on the next frame.
        self.assertEqual(frame_read(0b0000_0001, [], 0b0000_0001), 0b0000_0001)
        # First entry: A read at LY 0 of the frame that acts on it.
        self.assertEqual(frame_read(None, [0b0000_0001], 0), 0b0000_0001)
        self.assertEqual(frame_read(0, [0b0000_0001], 0b0000_0001), 0b0000_0001)
        self.assertEqual(frame_read(None, [], 0b0010_0000), 0b0010_0000)

    def test_frame_step_outside_a_level_uses_the_frame_read(self):
        # Map: the row adds A, but this frame's `ReadJoypad` still saw nothing.
        self.assertEqual(step_masks(16, [3], [], 0), [0])
        self.assertEqual(step_masks(16, [3, 4], [], 0b0000_0001), [16, 16])

    def test_steps_without_a_read_use_the_movie_row(self):
        self.assertEqual(step_masks(34, [1], [None]), [34])
        self.assertEqual(step_masks(34, [5, 6, 7], [0b0000_0010]), [32, 34, 34])
        self.assertEqual(step_masks(34, [1], []), [34])



class CompletedSampleTests(unittest.TestCase):
    def test_without_a_completion_the_live_memory_and_last_state_are_used(self):
        live = {0xFFC0: 1}
        self.assertEqual(completed_sample(live, [('u', None)], [('a', 0), ('b', 1)]),
                         (live, ('b', 1)))

    def test_clean_frame_pairs_the_completion_with_the_last_step(self):
        sample, state = completed_sample(
            {}, [('u', None), ('d', {0xFFC0: 7})], [('a', 0), ('b', 1)])
        self.assertEqual((sample[0xFFC0], state), (7, ('b', 1)))

    def test_straddling_update_pairs_with_the_step_before_it(self):
        # Death arc: update k finishes, update k+1 starts before the tick ends.
        live = {0xFFC0: 0xD1, 0xFF9B: 4}
        sample, state = completed_sample(
            live, [('d', {0xFFC0: 0xD4}), ('u', None)], [('a', 0), ('b', 1)])
        self.assertEqual((sample[0xFFC0], sample[0xFF9B], state), (0xD4, 4, ('a', 0)))

    def test_two_updates_in_one_frame_pair_the_last_completion(self):
        _, state = completed_sample(
            {}, [('u', None), ('d', {}), ('u', None), ('d', {})],
            [('a', 0), ('b', 1), ('c', 2)])
        self.assertEqual(state, ('c', 2))



class FrameTimingTests(unittest.TestCase):
    def test_iteration_reads_go_to_the_frame_they_belong_to(self):
        iterations = [{'frame': 9, 'ff97': 7, 'reads': [1]},
                      {'frame': 10, 'ff97': 8, 'reads': [2, 3]}]
        self.assertEqual(frame_timing(iterations, 10), (8, [2, 3]))
        self.assertIsNone(frame_timing(iterations, 11))

    def test_two_iterations_in_one_frame_keep_every_read(self):
        iterations = [{'frame': 4, 'ff97': 1, 'reads': [5]},
                      {'frame': 4, 'ff97': 2, 'reads': []}]
        self.assertEqual(frame_timing(iterations, 4), (2, [5]))



try:
    import numpy  # noqa: F401  (the comparison venv has it; plain python3 may not)
    HAVE_NUMPY = True
except ImportError:
    HAVE_NUMPY = False


class PixelReportTests(unittest.TestCase):
    @unittest.skipUnless(HAVE_NUMPY, 'numpy is in the PyBoy comparison environment')
    def test_pixel_row_compares_dmg_shades_not_display_greys(self):
        import numpy as np
        memory = {0xFF9B: 4, 0xA269: 3, 0xA68B: 0x2B}
        rom = np.zeros((144, 160, 4), dtype=np.uint8)
        rom[:, :, :3] = 153  # PyBoy's light grey is shade 1
        port = np.zeros((144, 160, 4), dtype=np.uint8)
        port[:, :, :3] = 170  # the port's light grey
        port[0, :3, :3] = 85
        row = compare_tas.pixel_row(7, memory, rom, port, None)
        self.assertEqual(row, {'row': 7, 'state': 4, 'level': 3, 'map': 0x2B, 'diff_same': 3})
        # The ROM's frame shows the state the port drew one frame earlier.
        self.assertEqual(compare_tas.pixel_row(7, memory, rom, rom * 0 + 255, port)['diff'], 3)

    @unittest.skipUnless(HAVE_NUMPY, 'numpy is in the PyBoy comparison environment')
    def test_unknown_greys_are_rejected(self):
        import numpy as np
        with self.assertRaises(RuntimeError):
            compare_tas.shade_indices(np.full((2, 2), 128, dtype=np.uint8), compare_tas.ROM_SHADES)

    def test_out_of_level_rows_keep_clock_diagnostics(self):
        self.addCleanup(compare_tas.OUT_OF_LEVEL.clear)
        state = read_state(io.BytesIO(b'STATE - - - - - tile_timer=5 tile_phase=2\n'))
        self.assertIsNone(state)
        self.assertEqual(compare_tas.OUT_OF_LEVEL, {'tile_timer': 5, 'tile_phase': 2})


if __name__ == '__main__':
    unittest.main()
