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
        self.clock = 0

        def tick(*_):
            self.clock += 1
            return True

        def load_state(buffer):
            self.clock = int(buffer.read())

        self.emulator.tick.side_effect = tick
        self.emulator.save_state.side_effect = lambda buffer: buffer.write(str(self.clock).encode())
        self.emulator.load_state.side_effect = load_state
        self.emulator.screen.ndarray.tobytes.side_effect = lambda: str(self.clock).encode()
        self.factory = Mock(return_value=self.emulator)

    def run_resume(self, extra=(), interactive=False, load_state=None):
        self.source.write_text(json.dumps(self.movie))
        package = SimpleNamespace(__file__=str(self.root / '__init__.py'), PyBoy=self.factory)
        with patch.object(sys, 'argv', [
            'record_play.py', '--rom', str(self.rom),
            *(['--load-state', str(load_state)] if load_state else ['--resume', str(self.source)]),
            '--out', str(self.output),
            *([] if interactive else ['--headless', '--frames', '2']), *extra
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
        self.emulator.tick.side_effect = None
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

    def test_checkpoint_retains_its_own_history_and_restores_held_button(self):
        path = self.root / 'retry.state'
        self.clock = 2
        record_play.save_checkpoint(path, self.movie, self.emulator)
        self.movie['frames'].append(0)
        self.clock = 3
        movie, state = record_play.read_checkpoint(path)
        self.assertEqual(movie['frames'], [16, 17])
        previous = record_play.restore_checkpoint(self.emulator, movie, state)
        self.assertEqual(previous, 17)
        self.assertEqual(self.clock, 2)

    def test_checkpoint_screen_mismatch_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'checkpoint screen differs'):
            record_play.restore_checkpoint(self.emulator, self.movie, b'3')

    def test_load_state_starts_new_recording_without_replaying_prefix(self):
        path = self.root / 'retry.state'
        self.clock = 2
        record_play.save_checkpoint(path, self.movie, self.emulator)
        original = path.read_bytes()
        self.clock = 100
        self.run_resume(load_state=path)
        saved = json.loads(self.output.read_text())
        self.assertEqual(saved['frames'], [16, 17, 0, 0])
        self.assertEqual(self.emulator.tick.call_count, 2)
        self.assertEqual(saved['final_screen_sha256'], hashlib.sha256(b'4').hexdigest())
        self.assertEqual(path.read_bytes(), original)

    def test_output_cannot_replace_loaded_checkpoint(self):
        path = self.output.with_suffix('.state')
        self.clock = 2
        record_play.save_checkpoint(path, self.movie, self.emulator)
        original = path.read_bytes()
        with self.assertRaises(SystemExit):
            self.run_resume(load_state=path)
        self.factory.assert_not_called()
        self.assertEqual(path.read_bytes(), original)

    def test_f5_f9_rewinds_inputs_and_pauses_before_retry(self):
        sdl = SimpleNamespace()
        for value, name in enumerate(('SDL_QUIT', 'SDL_WINDOWEVENT', 'SDL_KEYDOWN',
                                     'SDL_KEYUP', 'SDLK_RIGHT', 'SDLK_LEFT', 'SDLK_UP',
                                     'SDLK_DOWN', 'SDLK_z', 'SDLK_x', 'SDLK_RETURN',
                                     'SDLK_BACKSPACE', 'SDLK_ESCAPE', 'SDLK_p',
                                     'SDLK_F5', 'SDLK_F9'), 1):
            setattr(sdl, name, value)
        # Resume pauses at frame 2. Save it, play three failed frames, load,
        # stay paused for two loops, then retry for two frames and finish.
        script = iter(((sdl.SDLK_F5,), (sdl.SDLK_p,), (), (),
                       (sdl.SDLK_F9,), (), (), (sdl.SDLK_p,), (), (sdl.SDLK_ESCAPE,)))
        pending = []
        event = SimpleNamespace(key=SimpleNamespace(keysym=SimpleNamespace(), repeat=False))

        def poll(target):
            if not pending:
                pending.extend(next(script))
                pending.append(None)
            key = pending.pop(0)
            if key is None:
                return 0
            target.type = sdl.SDL_KEYDOWN
            target.key.keysym.sym = key
            return 1

        sdl.SDL_Init = Mock(return_value=0)
        sdl.SDL_INIT_VIDEO = 0
        sdl.SDL_HINT_RENDER_SCALE_QUALITY = b''
        sdl.SDL_SetHint = Mock()
        sdl.SDL_Quit = Mock()
        sdl.SDL_Event = Mock(return_value=event)
        sdl.SDL_PollEvent = poll
        with patch.dict(sys.modules, {'sdl2': sdl}), \
                patch.object(record_play.ctypes, 'byref', side_effect=lambda value: value), \
                patch.object(record_play, 'Display'), patch.object(record_play.time, 'sleep'):
            self.run_resume(interactive=True)
        saved = json.loads(self.output.read_text())
        self.assertEqual(saved['frames'], [16, 17, 0, 0])
        self.assertEqual(self.emulator.tick.call_count, 7)
        self.assertEqual(self.clock, 4)
        self.assertEqual(saved['final_screen_sha256'], hashlib.sha256(b'4').hexdigest())
        checkpoint, _ = record_play.read_checkpoint(self.output.with_suffix('.state'))
        self.assertEqual(checkpoint['frames'], [16, 17])


if __name__ == '__main__':
    unittest.main()
