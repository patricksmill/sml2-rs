#!/usr/bin/env python3
"""Show PyBoy ROM and native port windows driven by the same movie frame clock."""
import argparse
import ctypes
import hashlib
import io
import json
from pathlib import Path
import subprocess
import time


BUTTONS = [(1, 'right'), (2, 'left'), (4, 'up'), (8, 'down'),
           (16, 'a'), (32, 'b'), (64, 'start'), (128, 'select')]


def read_frame(pipe):
    import numpy as np
    data = bytearray()
    while len(data) < 160 * 144:
        chunk = pipe.read(160 * 144 - len(data))
        if not chunk:
            raise RuntimeError('native frame bridge closed unexpectedly')
        data.extend(chunk)
    gray = np.frombuffer(data, dtype=np.uint8).reshape(144, 160)
    rgba = np.empty((144, 160, 4), dtype=np.uint8)
    rgba[:, :, :3] = gray[:, :, None]
    rgba[:, :, 3] = 255
    return rgba


class Display:
    def __init__(self, sdl, title, x):
        self.sdl = sdl
        self.window = sdl.SDL_CreateWindow(title.encode(), x, 100, 480, 432,
                                          sdl.SDL_WINDOW_SHOWN | sdl.SDL_WINDOW_RESIZABLE)
        if not self.window:
            raise RuntimeError(sdl.SDL_GetError().decode())
        self.renderer = sdl.SDL_CreateRenderer(self.window, -1, 0)
        if not self.renderer:
            raise RuntimeError(sdl.SDL_GetError().decode())
        self.texture = sdl.SDL_CreateTexture(self.renderer, sdl.SDL_PIXELFORMAT_RGBA32,
                                            sdl.SDL_TEXTUREACCESS_STREAMING, 160, 144)
        if not self.texture:
            raise RuntimeError(sdl.SDL_GetError().decode())
        sdl.SDL_RenderSetLogicalSize(self.renderer, 160, 144)

    def draw(self, rgba, title):
        import numpy as np
        sdl = self.sdl
        pixels = np.ascontiguousarray(rgba)
        sdl.SDL_SetWindowTitle(self.window, title.encode())
        if sdl.SDL_UpdateTexture(self.texture, None, pixels.ctypes.data_as(ctypes.c_void_p), 160 * 4):
            raise RuntimeError(sdl.SDL_GetError().decode())
        sdl.SDL_RenderClear(self.renderer)
        sdl.SDL_RenderCopy(self.renderer, self.texture, None, None)
        sdl.SDL_RenderPresent(self.renderer)

    def close(self):
        self.sdl.SDL_DestroyTexture(self.texture)
        self.sdl.SDL_DestroyRenderer(self.renderer)
        self.sdl.SDL_DestroyWindow(self.window)


def apply_buttons(pyboy, previous, mask):
    for bit, name in BUTTONS:
        if (mask ^ previous) & bit:
            (pyboy.button_press if mask & bit else pyboy.button_release)(name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('movie', type=Path, help='JSON from import_bk2.py')
    parser.add_argument('--rom', type=Path, required=True)
    parser.add_argument('--bridge', type=Path, default=Path('target/release/examples/tas_bridge'))
    parser.add_argument('--bootrom', type=Path, help='optional local DMG boot ROM; otherwise PyBoy built-in')
    parser.add_argument('--paused', action='store_true')
    parser.add_argument('--headless', action='store_true', help='check protocol without opening windows')
    parser.add_argument('--frames', type=int)
    args = parser.parse_args()
    movie = json.loads(args.movie.read_text())
    if movie.get('format') != 'sml2-tas-v1' or not movie.get('frames'):
        parser.error('unsupported or empty movie')
    rom = args.rom.read_bytes()
    if hashlib.sha1(rom).hexdigest() != movie['rom_sha1']:
        parser.error('ROM SHA1 does not match movie')
    frames = movie['frames']
    if any(type(mask) is not int or not 0 <= mask <= 255 for mask in frames):
        parser.error('invalid movie input mask')
    if args.frames is not None and args.frames <= 0:
        parser.error('--frames must be positive')
    if args.headless and args.paused:
        parser.error("--paused requires interactive windows")
    from pyboy import PyBoy
    # A file-like ROM prevents PyBoy from loading the ROM path's existing .ram.
    pyboy = PyBoy(io.BytesIO(rom), window='null', sound_emulated=False,
                  cgb=False, bootrom=str(args.bootrom) if args.bootrom else None)
    pyboy.set_emulation_speed(0)
    bridge = None
    displays = []
    sdl = None
    try:
        bridge = subprocess.Popen([str(args.bridge.resolve())], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE)
        native = read_frame(bridge.stdout)
        if not args.headless:
            import sdl2 as sdl
            if sdl.SDL_Init(sdl.SDL_INIT_VIDEO) != 0:
                raise RuntimeError(sdl.SDL_GetError().decode())
            sdl.SDL_SetHint(sdl.SDL_HINT_RENDER_SCALE_QUALITY, b'0')
            displays.append(Display(sdl, 'PyBoy ROM', 60))
            displays.append(Display(sdl, 'Rust port', 560))
        print(f"Inputs by {movie.get('author', 'unknown')}; same input index in both windows.", flush=True)
        print('Space pause/resume BOTH; Right step BOTH; Tab 1x/0.25x; Escape closes BOTH.', flush=True)
        print('PyBoy uses its built-in boot ROM unless --bootrom is supplied; this BizHawk movie may desync in PyBoy too.', flush=True)
        paused, slow, running = args.paused, False, True
        index, previous = 0, 0
        count = min(len(frames), args.frames or len(frames))
        while running:
            step_once = False
            started = time.perf_counter()
            if sdl:
                event = sdl.SDL_Event()
                while sdl.SDL_PollEvent(ctypes.byref(event)):
                    if event.type == sdl.SDL_QUIT or (event.type == sdl.SDL_WINDOWEVENT and event.window.event == sdl.SDL_WINDOWEVENT_CLOSE):
                        running = False
                    elif event.type == sdl.SDL_KEYDOWN and not event.key.repeat:
                        key = event.key.keysym.sym
                        if key == sdl.SDLK_ESCAPE:
                            running = False
                        elif key == sdl.SDLK_SPACE:
                            paused = not paused
                        elif key == sdl.SDLK_RIGHT:
                            step_once = True
                        elif key == sdl.SDLK_TAB:
                            slow = not slow
            if not running:
                break
            if index < count and (not paused or step_once):
                mask = frames[index]
                apply_buttons(pyboy, previous, mask)
                bridge.stdin.write(f'{mask}\n'.encode())
                bridge.stdin.flush()
                if not pyboy.tick(1, True, False):
                    raise RuntimeError('PyBoy stopped before end of movie')
                native = read_frame(bridge.stdout)
                previous = mask
                index += 1
            if args.headless and index == count:
                break
            held = '+'.join(name for bit, name in BUTTONS if previous & bit) or 'none'
            status = 'end' if index == count else 'paused' if paused else '0.25x' if slow else '1x'
            label = f'input {index - 1 if index else "not started"} | {held} | {status}'
            if displays:
                displays[0].draw(pyboy.screen.ndarray, f'PyBoy ROM | {label}')
                displays[1].draw(native, f'Rust port | {label}')
                time.sleep(max(0, 1 / (15 if slow else 60) - (time.perf_counter() - started)))
        print(f'Stopped after {index} shared input rows.', flush=True)
    finally:
        if bridge:
            bridge.stdin.close()
            bridge.stdout.close()
            try:
                bridge.wait(timeout=2)
            except subprocess.TimeoutExpired:
                bridge.terminate()
                bridge.wait(timeout=2)
        pyboy.stop(save=False)
        for display in reversed(displays):
            display.close()
        if sdl:
            sdl.SDL_Quit()


if __name__ == '__main__':
    main()
