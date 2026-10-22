#!/usr/bin/env python3
"""Record manual PyBoy play as exact held-button frames for native comparison."""
import argparse
import ctypes
from datetime import datetime
import hashlib
from importlib.metadata import version
import io
import json
from pathlib import Path
import time
import zipfile

from compare_tas import Display, apply_buttons


def save_movie(path, movie, pyboy):
    # Publish a complete JSON file atomically, including a replay verification hash.
    movie['final_screen_sha256'] = hashlib.sha256(pyboy.screen.ndarray.tobytes()).hexdigest()
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(movie) + '\n')
    temp.replace(path)


def save_checkpoint(path, movie, pyboy):
    state = io.BytesIO()
    pyboy.save_state(state)
    saved_movie = dict(movie, final_screen_sha256=hashlib.sha256(
        pyboy.screen.ndarray.tobytes()).hexdigest())
    temp = path.with_suffix(path.suffix + '.tmp')
    with zipfile.ZipFile(temp, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('movie.json', json.dumps(saved_movie))
        archive.writestr('pyboy.state', state.getvalue())
    temp.replace(path)


def read_checkpoint(path):
    with zipfile.ZipFile(path) as archive:
        return json.loads(archive.read('movie.json')), archive.read('pyboy.state')


def validate_movie(movie, config):
    if not isinstance(movie, dict) or movie.get('format') != 'sml2-tas-v1' or movie.get('source') != 'manual-pyboy':
        raise ValueError('requires a manual PyBoy recording')
    frames = movie.get('frames')
    if not isinstance(frames, list) or any(type(mask) is not int or not 0 <= mask <= 255
                                          for mask in frames):
        raise ValueError('invalid recording input masks')
    for field in ('rom_sha1', 'pyboy_version', 'bootrom_sha1', 'sound_emulated'):
        if type(movie.get(field)) is not type(config[field]) or movie[field] != config[field]:
            raise ValueError(f'recording {field} differs from the current configuration')
    if not movie.get('final_screen_sha256'):
        raise ValueError('recording has no final screen hash to verify')


def restore_checkpoint(pyboy, movie, state):
    pyboy.load_state(io.BytesIO(state))
    actual = hashlib.sha256(pyboy.screen.ndarray.tobytes()).hexdigest()
    if actual != movie['final_screen_sha256']:
        raise RuntimeError('loaded checkpoint screen differs from recording')
    return movie['frames'][-1] if movie['frames'] else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rom', type=Path, required=True)
    parser.add_argument('--out', type=Path)
    start = parser.add_mutually_exclusive_group()
    start.add_argument('--resume', type=Path, help='replay a manual recording, then continue in a new file')
    start.add_argument('--load-state', type=Path, help='continue from an F5 checkpoint in a new recording')
    parser.add_argument('--headless', action='store_true', help='record a neutral-input smoke check')
    parser.add_argument('--frames', type=int, help='stop after this many new recorded ticks')
    args = parser.parse_args()
    if args.headless and (args.frames is None or args.frames <= 0):
        parser.error('--headless requires --frames N, greater than zero')
    if args.frames is not None and args.frames <= 0:
        parser.error('--frames must be positive')
    output = args.out or Path('recordings') / f'pyboy-{datetime.now():%Y%m%d-%H%M%S-%f}.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        parser.error(f'refusing to overwrite recording: {output}')
    import pyboy as pyboy_package
    from pyboy import PyBoy
    rom = args.rom.read_bytes()
    bootrom = Path(pyboy_package.__file__).parent / 'core' / 'bootrom_dmg.bin'
    movie = {'format': 'sml2-tas-v1', 'author': 'manual player',
             'source': 'manual-pyboy', 'rom_sha1': hashlib.sha1(rom).hexdigest(),
             'pyboy_version': version('pyboy'),
             'bootrom_sha1': hashlib.sha1(bootrom.read_bytes()).hexdigest(),
             'sound_emulated': False, 'frames': []}
    config = movie.copy()
    state = None
    if args.resume or args.load_state:
        if args.load_state:
            resumed, state = read_checkpoint(args.load_state)
        else:
            resumed = json.loads(args.resume.read_text())
        try:
            validate_movie(resumed, config)
        except ValueError as exc:
            parser.error(str(exc))
        movie = resumed
    prefix_length = len(movie['frames'])
    checkpoint_path = output.with_suffix('.state')
    if checkpoint_path == output:
        parser.error('--out must use a different extension from .state')
    if args.load_state and checkpoint_path.resolve() == args.load_state.resolve():
        parser.error('--out would overwrite the loaded checkpoint; choose another output path')
    available_checkpoint = args.load_state
    # Fresh RAM, identical boot/core settings to compare_tas.py; no existing saves.
    pyboy = PyBoy(io.BytesIO(rom), window='null', sound_emulated=False, cgb=False)
    pyboy.set_emulation_speed(0)
    sdl, display = None, None
    mask = previous = 0
    paused, focused, running = False, True, True
    ready = False
    try:
        if args.load_state:
            previous = restore_checkpoint(pyboy, movie, state)
            paused = not args.headless
            print(f'Loaded checkpoint at frame {prefix_length}.', flush=True)
        elif args.resume:
            print(f'Replaying {prefix_length} frames from {args.resume}…', flush=True)
            for previous_index, held in enumerate(movie['frames'], 1):
                apply_buttons(pyboy, previous, held)
                previous = held
                if not pyboy.tick(1, True, False):
                    raise RuntimeError(f'PyBoy stopped at replay frame {previous_index}')
                if previous_index % 3600 == 0:
                    print(f'Replayed {previous_index}/{prefix_length} frames', flush=True)
            actual = hashlib.sha256(pyboy.screen.ndarray.tobytes()).hexdigest()
            if actual != movie['final_screen_sha256']:
                raise RuntimeError('replayed screen differs from recording; continuation was not saved')
            print('Replay verified. Continuing from the last recorded frame.', flush=True)
            paused = not args.headless
        if not args.headless:
            import sdl2 as sdl
            if sdl.SDL_Init(sdl.SDL_INIT_VIDEO) != 0:
                raise RuntimeError(sdl.SDL_GetError().decode())
            sdl.SDL_SetHint(sdl.SDL_HINT_RENDER_SCALE_QUALITY, b'0')
            display = Display(sdl, 'PyBoy — RECORDING', 100)
            key_masks = {sdl.SDLK_RIGHT: 1, sdl.SDLK_LEFT: 2, sdl.SDLK_UP: 4,
                         sdl.SDLK_DOWN: 8, sdl.SDLK_z: 16, sdl.SDLK_x: 32,
                         sdl.SDLK_RETURN: 64, sdl.SDLK_BACKSPACE: 128}
        ready = True
        print(f'Recording to {output.resolve()}', flush=True)
        print('Arrows move; Z = A/jump; X = B/run; Enter = Start; Backspace = Select.', flush=True)
        print('P pauses/resumes recording; Escape or close window finishes and saves. Autosave every 300 frames.', flush=True)
        print(f'F5 saves checkpoint to {checkpoint_path.resolve()}; F9 loads it and pauses.', flush=True)
        if paused:
            print('Paused at the end of the recording. Press P when ready to play.', flush=True)
        while running:
            started = time.perf_counter()
            if sdl:
                event = sdl.SDL_Event()
                while sdl.SDL_PollEvent(ctypes.byref(event)):
                    if event.type == sdl.SDL_QUIT:
                        running = False
                    elif event.type == sdl.SDL_WINDOWEVENT:
                        if event.window.event == sdl.SDL_WINDOWEVENT_CLOSE:
                            running = False
                        elif event.window.event == sdl.SDL_WINDOWEVENT_FOCUS_LOST:
                            focused, mask = False, 0
                        elif event.window.event == sdl.SDL_WINDOWEVENT_FOCUS_GAINED:
                            focused = True
                    elif event.type in (sdl.SDL_KEYDOWN, sdl.SDL_KEYUP):
                        key = event.key.keysym.sym
                        down = event.type == sdl.SDL_KEYDOWN
                        if down and key == sdl.SDLK_ESCAPE:
                            running = False
                        elif down and key == sdl.SDLK_p and not event.key.repeat:
                            paused, mask = not paused, 0
                        elif down and key == sdl.SDLK_F5 and not event.key.repeat:
                            save_checkpoint(checkpoint_path, movie, pyboy)
                            available_checkpoint = checkpoint_path
                            print(f'Checkpoint saved at frame {len(movie["frames"])}.', flush=True)
                        elif down and key == sdl.SDLK_F9 and not event.key.repeat:
                            if available_checkpoint is None:
                                print('No checkpoint yet. Press F5 to save one.', flush=True)
                                continue
                            restored_movie, restored_state = read_checkpoint(available_checkpoint)
                            validate_movie(restored_movie, config)
                            ready = False
                            previous = restore_checkpoint(pyboy, restored_movie, restored_state)
                            movie = restored_movie
                            ready = True
                            paused, mask = True, 0
                            save_movie(output, movie, pyboy)
                            print(f'Checkpoint loaded at frame {len(movie["frames"])}. Press P to retry.', flush=True)
                        elif key in key_masks:
                            mask = mask | key_masks[key] if down else mask & ~key_masks[key]
            if not running:
                break
            if not paused and focused:
                apply_buttons(pyboy, previous, mask)
                previous = mask
                if not pyboy.tick(1, True, False):
                    break
                movie['frames'].append(mask)
                if len(movie['frames']) % 300 == 0:
                    save_movie(output, movie, pyboy)
            if display:
                status = 'PAUSED' if paused or not focused else 'RECORDING'
                display.draw(pyboy.screen.ndarray, f'PyBoy {status} | frame {len(movie["frames"])} | Z jump, X run | F5 save state, F9 load | P pause | Esc save')
                time.sleep(max(0, 1 / 60 - (time.perf_counter() - started)))
            if args.frames and len(movie['frames']) - prefix_length >= args.frames:
                break
    finally:
        if ready:
            save_movie(output, movie, pyboy)
        pyboy.stop(save=False)
        if display:
            display.close()
        if sdl:
            sdl.SDL_Quit()
        if ready:
            print(f'Saved {len(movie["frames"])} input frames to {output.resolve()}', flush=True)


if __name__ == '__main__':
    main()
