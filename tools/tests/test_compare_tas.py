"""Comparison bridge protocol, button delivery, and logic phase selection."""
import io
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from compare_tas import apply_buttons, logic_phases, read_state


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

    def test_legacy_state(self):
        self.assertEqual(read_state(io.BytesIO(b'STATE 0 34 468 80 448\n')), {
            'level': 0, 'x': 34, 'y': 468, 'cam_x': 80, 'cam_y': 448,
        })

    def test_outside_level(self):
        self.assertIsNone(read_state(io.BytesIO(b'STATE - - - - -\n')))

    def test_invalid_state(self):
        for line in (b'STATE 0 34 468 80\n', b'STATE 0 34 468 80 448 80\n',
                     b'STATE 0 34 468 80 448 zz 18 1 0\n',
                     b'STATE 0 bad 468 80 448\n'):
            with self.subTest(line=line), self.assertRaisesRegex(RuntimeError, 'malformed'):
                read_state(io.BytesIO(line))

    def test_unexpected_output(self):
        with self.assertRaisesRegex(RuntimeError, 'unexpected bridge state'):
            read_state(io.BytesIO(b'diagnostic\n'))

    def test_closed_bridge(self):
        with self.assertRaisesRegex(RuntimeError, 'closed unexpectedly'):
            read_state(io.BytesIO())


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

    def test_pipe_travel_without_a_player_update_still_steps_once(self):
        self.assertEqual(logic_phases(4, 0x10, [1], []), [1])

    def test_in_level_overrun_does_not_step(self):
        self.assertEqual(logic_phases(4, 0x7C, [], []), [])

    def test_outside_a_level_steps_once_per_hardware_frame(self):
        self.assertEqual(logic_phases(0x0C, 0x20, [], []), [1])


if __name__ == '__main__':
    unittest.main()
