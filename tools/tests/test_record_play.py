"""Recording continuation preserves inputs and rejects unverified replay state."""
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
import record_play


class ResumeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.rom = self.root / 'rom.gb'
        self.rom.write_bytes(b'test ROM')
        (self.root / 'core').mkdir()
        (self.root / 'core/bootrom_dmg.bin').write_bytes(b'test boot ROM')
        self.source = self.root / 'original.json'
        self.output = self.root / 'continued.json'
        self.movie = dict(format='sml2-tas-v1', source='manual-pyboy',
                          author='manual player', frames=[16, 17],
                          rom_sha1=hashlib.sha1(b'test ROM').hexdigest(),
                          bootrom_sha1=hashlib.sha1(b'test boot ROM').hexdigest(),
                          pyboy_version='test', sound_emulated=False,
                          final_screen_sha256=hashlib.sha256(b'2').hexdigest())
        self.emulator = Mock()
        self.emulator.tick.return_value = True
        self.emulator.screen.ndarray.tobytes.side_effect = lambda: str(
            self.emulator.tick.call_count).encode()
        self.factory = Mock(return_value=self.emulator)

    def run_resume(self, extra=()):
        self.source.write_text(json.dumps(self.movie))
        package = SimpleNamespace(__file__=str(self.root / '__init__.py'), PyBoy=self.factory)
        with patch.object(sys, 'argv', [
            'record_play.py', '--rom', str(self.rom), '--resume', str(self.source),
            '--out', str(self.output), '--headless', '--frames', '2', *extra
        ]), patch.dict(sys.modules, {'pyboy': package}), \
                patch.object(record_play, 'version', return_value='test'), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            record_play.main()

    def test_resume_keeps_prefix_releases_held_buttons_and_counts_only_new_frames(self):
        self.run_resume()
        self.assertEqual(json.loads(self.output.read_text())['frames'], [16, 17, 0, 0])
        self.assertEqual(json.loads(self.source.read_text()), self.movie)
        self.emulator.button_release.assert_any_call('right')
        self.emulator.button_release.assert_any_call('a')
        self.assertEqual(self.emulator.tick.call_count, 4)
        self.assertEqual(json.loads(self.output.read_text())['final_screen_sha256'],
                         hashlib.sha256(b'4').hexdigest())
        self.emulator.stop.assert_called_once_with(save=False)

    def test_bad_replay_hash_never_publishes_continuation(self):
        self.movie['final_screen_sha256'] = 'wrong'
        with self.assertRaisesRegex(RuntimeError, 'replayed screen differs'):
            self.run_resume()
        self.assertFalse(self.output.exists())
        self.assertEqual(json.loads(self.source.read_text()), self.movie)
        self.emulator.stop.assert_called_once_with(save=False)

    def test_stopped_replay_never_publishes_continuation(self):
        self.emulator.tick.return_value = False
        with self.assertRaisesRegex(RuntimeError, 'stopped at replay frame 1'):
            self.run_resume()
        self.assertFalse(self.output.exists())
        self.emulator.stop.assert_called_once_with(save=False)

    def test_configuration_and_input_errors_are_rejected_before_emulation(self):
        for field, bad in [('rom_sha1', 'wrong'), ('bootrom_sha1', 'wrong'),
                           ('pyboy_version', 'old'), ('sound_emulated', True),
                           ('sound_emulated', 0), ('frames', [256]),
                           ('frames', [True]), ('frames', None),
                           ('source', 'bizhawk'), ('final_screen_sha256', '')]:
            original = self.movie[field]
            with self.subTest(field=field, value=bad):
                self.movie[field] = bad
                with self.assertRaises(SystemExit) as exited:
                    self.run_resume()
                self.assertEqual(exited.exception.code, 2)
                self.factory.assert_not_called()
                self.assertFalse(self.output.exists())
            self.movie[field] = original

    def test_original_cannot_be_used_as_output(self):
        with self.assertRaises(SystemExit):
            self.run_resume(['--out', str(self.source)])
        self.assertEqual(json.loads(self.source.read_text()), self.movie)
        self.factory.assert_not_called()


if __name__ == '__main__':
    unittest.main()
