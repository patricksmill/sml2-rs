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

from compare_tas import Display, apply_buttons


def save_movie(path, movie, pyboy):
    # Publish a complete JSON file atomically, including a replay verification hash.
    movie['final_screen_sha256'] = hashlib.sha256(pyboy.screen.ndarray.tobytes()).hexdigest()
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(movie) + '\n')
    temp.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rom', type=Path, required=True)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--headless', action='store_true', help='record a neutral-input smoke check')
    parser.add_argument('--frames', type=int, help='stop after this many recorded ticks')
    args = parser.parse_args()
    if args.headless and (args.frames is None or args.frames <= 0):
        parser.error('--headless requires --frames N, greater than zero')
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
    # Fresh RAM, identical boot/core settings to compare_tas.py; no existing saves.
    pyboy = PyBoy(io.BytesIO(rom), window='null', sound_emulated=False, cgb=False)
    pyboy.set_emulation_speed(0)
    sdl, display = None, None
    mask = previous = 0
    paused, focused, running = False, True, True
    try:
        if not args.headless:
            import sdl2 as sdl
            if sdl.SDL_Init(sdl.SDL_INIT_VIDEO) != 0:
                raise RuntimeError(sdl.SDL_GetError().decode())
            sdl.SDL_SetHint(sdl.SDL_HINT_RENDER_SCALE_QUALITY, b'0')
            display = Display(sdl, 'PyBoy — RECORDING', 100)
            key_masks = {sdl.SDLK_RIGHT: 1, sdl.SDLK_LEFT: 2, sdl.SDLK_UP: 4,
                         sdl.SDLK_DOWN: 8, sdl.SDLK_z: 16, sdl.SDLK_x: 32,
                         sdl.SDLK_RETURN: 64, sdl.SDLK_BACKSPACE: 128}
        print(f'Recording to {output.resolve()}', flush=True)
        print('Arrows move; Z = A/jump; X = B/run; Enter = Start; Backspace = Select.', flush=True)
        print('P pauses/resumes recording; Escape or close window finishes and saves. Autosave every 300 frames.', flush=True)
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
                display.draw(pyboy.screen.ndarray, f'PyBoy {status} | frame {len(movie["frames"])} | Arrows, Z jump, X run, Enter Start | P pause | Esc save')
                time.sleep(max(0, 1 / 60 - (time.perf_counter() - started)))
            if args.frames and len(movie['frames']) >= args.frames:
                break
    finally:
        save_movie(output, movie, pyboy)
        pyboy.stop(save=False)
        if display:
            display.close()
        if sdl:
            sdl.SDL_Quit()
        print(f'Saved {len(movie["frames"])} input frames to {output.resolve()}', flush=True)


if __name__ == '__main__':
    main()
