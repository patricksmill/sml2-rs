"""A direction change must release the old key while preserving held buttons."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from compare_tas import apply_buttons


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


if __name__ == '__main__':
    unittest.main()
