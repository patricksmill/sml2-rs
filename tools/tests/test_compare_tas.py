"""Comparison bridge protocol, button delivery, and logic phase selection."""
import io
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from compare_tas import (apply_buttons, logic_phases, read_state,
                         check_divergence, comparison_threshold, rom_keys_mask,
                         step_masks)


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
                          b'step=8102 fff0=174 update_phase=0 standing=1 scene=17 owns=1 pc=24700\n'
                          + pixels + b'STATE - - - - -\n' + pixels)
        state = read_state(pipe)
        self.assertEqual(state['step'], 8102)
        self.assertEqual(state['fff0'], 174)
        self.assertEqual(state['update_phase'], 0)
        self.assertEqual(state['standing'], 1)
        self.assertEqual(state['scene'], 17)
        self.assertEqual(state['owns'], 1)
        self.assertEqual(state['pc'], 24700)
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
        self.assertEqual(logic_phases(4, 0x7C, [1, 1], [5]), [5])

    def test_camera_transition_without_a_player_update_still_steps_once(self):
        self.assertEqual(logic_phases(4, 0x10, [1], []), [1])

    def test_blocking_pipe_scene_advances_without_gameplay_hooks(self):
        self.assertEqual(logic_phases(4, 0x10, [], [], [2]), [2])

    def test_scene_resumes_after_gameplay_in_the_same_hardware_frame(self):
        self.assertEqual(logic_phases(4, 0x10, [1], [5], [6, 7]), [5, 6, 7])

    def test_scene_wait_without_a_return_does_not_advance(self):
        self.assertEqual(logic_phases(4, 7, [], [], []), [])

    def test_scene_return_phase_wraps_without_a_gameplay_step(self):
        self.assertEqual(logic_phases(4, 7, [], [], [0]), [0])

    def test_in_level_overrun_does_not_step(self):
        self.assertEqual(logic_phases(4, 0x7C, [], []), [])

    def test_outside_a_level_steps_once_per_hardware_frame(self):
        self.assertEqual(logic_phases(0x0C, 0x20, [], []), [1])



class StepMaskTests(unittest.TestCase):
    def test_rom_joypad_byte_maps_to_movie_layout(self):
        # hKeysHeld: A=0, B=1, Select=2, Start=3, Right=4, Left=5, Up=6, Down=7.
        self.assertEqual(rom_keys_mask(0b0000_0001), 16)
        self.assertEqual(rom_keys_mask(0b0010_0010), 2 | 32)
        self.assertEqual(rom_keys_mask(0b1100_1100), 8 | 4 | 64 | 128)

    def test_player_update_uses_the_keys_the_rom_read(self):
        # The row adds Left, but the overrunning frame read the previous row (B).
        self.assertEqual(step_masks(2 | 32, [1], [0b0000_0010]), [32])

    def test_steps_without_a_read_use_the_movie_row(self):
        self.assertEqual(step_masks(34, [1], [None]), [34])
        self.assertEqual(step_masks(34, [5, 6, 7], [0b0000_0010]), [32, 34, 34])
        self.assertEqual(step_masks(34, [1], []), [34])


if __name__ == '__main__':
    unittest.main()
