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


def read_state(pipe):
    line = pipe.readline()
    if not line:
        raise RuntimeError('native frame bridge closed unexpectedly')
    text = line.decode().strip()
    if not text.startswith('STATE '):
        raise RuntimeError(f'unexpected bridge state line: {text!r}')
    parts = text.split()[1:]
    if parts == ['-'] * 5:
        return None
    if len(parts) not in (5, 9) and (
            len(parts) < 10 or not all("=" in p for p in parts[9:])):
        raise RuntimeError(f'malformed bridge state line: {text!r}')
    try:
        level, x, y, cam_x, cam_y = (int(value) for value in parts[:5])
        state = {'level': level, 'x': x, 'y': y, 'cam_x': cam_x, 'cam_y': cam_y}
        if len(parts) >= 9:
            state.update(hvel=int(parts[5], 16), vvel=int(parts[6], 16),
                         on_ground=int(parts[7]), phase=int(parts[8]))
            for field in parts[9:]:
                key, value = field.split("=", 1)
                if key not in {"step", "fff0", "update_phase", "standing", "scene", "owns", "pc"}:
                    raise ValueError("unknown diagnostic field")
                state[key] = int(value)
    except ValueError as exc:
        raise RuntimeError(f'malformed bridge state line: {text!r}') from exc
    return state


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


def logic_phases(before_state, before_ff97, st4_ran, f71_phases, scene_phases=()):
    """Port steps for one hardware frame while phase-lock is on.

    `$0F71` is one player update, and the phase argument is `$FF97 & 7` sampled
    at that entry. `$FF97` itself is not a reliable count: a lag frame increments
    it zero times, and a frame that finishes two waits increments it twice, while
    the player update still runs once. A state-`$04` entry that never reaches
    `$0F71` (the screen-transition branch) still advances the camera and scene,
    so it takes one step at the predicted phase. Blocking scenes (`$20A4` to
    `$21EE`) resume on each `$2728` return, bypassing both gameplay hooks.
    Append those steps in execution order, including multiple waits completed
    in one hardware frame. With neither kind of update, do not step again.
    """
    if f71_phases:
        phases = list(f71_phases)
    elif st4_ran or before_state != 4:
        phases = [(before_ff97 + 1) & 7]
    else:
        phases = []
    return phases + list(scene_phases)


def check_divergence(frame, memory, native, threshold, live=False, player_ran=True,
                     native_step=None):
    """In-level ROM position vs the port. `live` reads `$FFC0`/`$FFC2` (the
    position `$181C` just wrote). Otherwise `sMarioX`/`sMarioY`, which the
    state-`$04` handler publishes one frame later.

    The frame that first stores state `$04` has not run the player yet, so the
    port may still be on the last entry-hold step. That is only a missed level
    once `$0F71` or the pipe path has run.
    """
    step = native.get('step', native_step) if native is not None else native_step
    contract = (f'compared level/x/y, position tolerance {threshold}px; '
                f'native update {step if step is not None else "unknown"}')
    if memory[0xFF9B] == 4 and native is None and player_ran:
        return {'frame': frame, 'dx': 0, 'dy': 0,
                'native_step': step,
                'detail': f'ROM level {memory[0xA269]} vs port not in a level; {contract}'}
    if native is None or memory[0xFF9B] != 4:
        return None
    if memory[0xA269] != native['level']:
        return {'frame': frame, 'dx': 0, 'dy': 0,
                'native_step': step,
                'detail': f'ROM level {memory[0xA269]} vs port level {native["level"]}; {contract}'}
    if live:
        x = memory[0xFFC2] | memory[0xFFC3] << 8
        y = memory[0xFFC0] | memory[0xFFC1] << 8
    else:
        x = memory[0xA227] | memory[0xA228] << 8
        y = memory[0xA229] | memory[0xA22A] << 8
        if x == 0 and y == 0:
            return None  # the ROM's first gameplay frame still has the level entry pending
    cam_x = memory[0xFFCA] | memory[0xFFCB] << 8
    cam_y = memory[0xFFC8] | memory[0xFFC9] << 8
    dx, dy = native['x'] - x, native['y'] - y
    # Camera bytes are sampled now, even in the delayed sMario mode. Report
    # their differences, but do not turn that mixed-boundary sample into a gate.
    dcx, dcy = native['cam_x'] - cam_x, native['cam_y'] - cam_y
    if abs(dx) <= threshold and abs(dy) <= threshold:
        return None
    diagnostics = ', '.join(f'{key}={native[key]}' for key in
                            ('fff0', 'update_phase', 'standing', 'scene', 'owns', 'pc')
                            if key in native)
    detail = (f'level {native["level"]} ROM ({x}, {y}) vs port ({native["x"]}, {native["y"]}); '
              f'world camera ROM ({cam_x}, {cam_y}) vs port ({native["cam_x"]}, {native["cam_y"]}) '
              f'(delta {dcx}, {dcy}; diagnostic only); '
              f'{contract}; '
              f'ROM {"live HRAM" if live else "published prior position / current camera"}')
    if diagnostics:
        detail += (f'; native {diagnostics}; ROM current fff0={memory[0xFFF0]} '
                   f'update_phase={memory[0xAF08]} standing={memory[0xA26B]} '
                   f'scene={memory[0xA28E] | memory[0xA28F] << 8}')
    return {'frame': frame, 'native_step': step, 'dx': dx, 'dy': dy,
            'cam_dx': dcx, 'cam_dy': dcy, 'detail': detail}


def comparison_threshold(threshold, exact=False, disabled=False):
    """None disables comparison; exact is explicit so threshold=0 stays disabled."""
    if disabled:
        return None
    if exact:
        return 0
    return threshold if threshold > 0 else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('movie', type=Path, help='JSON from import_bk2.py')
    parser.add_argument('--rom', type=Path, required=True)
    parser.add_argument('--bridge', type=Path, default=Path('target/release/examples/tas_bridge'))
    parser.add_argument('--no-phase-lock', action='store_true',
                        help="do not feed the ROM's $FF97 phase to the port (see --phase-lock)")
    parser.add_argument('--diverge-threshold', type=int, default=4,
                        help='pause both windows when |dx| or |dy| exceeds this many pixels (0 disables)')
    parser.add_argument('--exact-position', action='store_true',
                        help='pause on any world x/y mismatch (overrides threshold 0)')
    parser.add_argument('--debug-state', action='store_true',
                        help='include native clock/contact/scene diagnostics in divergence reports')
    parser.add_argument('--no-diverge', action='store_true',
                        help='keep playing past the first position divergence')
    parser.add_argument('--port-initial-phase', type=int, choices=range(8),
                        help='diagnostic first-level movement phase origin; input timing is unchanged')
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
    if args.diverge_threshold < 0:
        parser.error('--diverge-threshold must be nonnegative')
    if args.frames is not None and args.frames <= 0:
        parser.error('--frames must be positive')
    if args.headless and args.paused:
        parser.error("--paused requires interactive windows")
    from pyboy import PyBoy
    if movie.get('source') == 'manual-pyboy':
        from importlib.metadata import version
        import pyboy as pyboy_package
        boot = args.bootrom or Path(pyboy_package.__file__).parent / 'core' / 'bootrom_dmg.bin'
        if version('pyboy') != movie.get('pyboy_version'):
            parser.error('PyBoy version differs from the manual recording')
        if hashlib.sha1(boot.read_bytes()).hexdigest() != movie.get('bootrom_sha1'):
            parser.error('boot ROM differs from the manual recording')
        if movie.get('sound_emulated') is not False:
            parser.error('unsupported recording audio configuration')
    # A file-like ROM prevents PyBoy from loading the ROM path's existing .ram.
    pyboy = PyBoy(io.BytesIO(rom), window='null', sound_emulated=False,
                  cgb=False, bootrom=str(args.bootrom) if args.bootrom else None)
    pyboy.set_emulation_speed(0)
    bridge = None
    displays = []
    sdl = None
    try:
        bridge_args = [str(args.bridge.resolve())]
        if args.port_initial_phase is not None:
            bridge_args.extend(['--initial-phase', str(args.port_initial_phase)])
        threshold = comparison_threshold(args.diverge_threshold, args.exact_position, args.no_diverge)
        watch = threshold is not None
        state_rows = watch or args.debug_state
        phase_lock = not args.no_phase_lock
        if state_rows:
            bridge_args.append('--state')
        if args.debug_state:
            bridge_args.append('--debug-state')
        if phase_lock:
            bridge_args.append('--phase-lock')
        bridge = subprocess.Popen(bridge_args, stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE)
        native_state = read_state(bridge.stdout) if state_rows else None
        native = read_frame(bridge.stdout)
        native_step = 0
        previous_native_step = None
        if not args.headless:
            import sdl2 as sdl
            if sdl.SDL_Init(sdl.SDL_INIT_VIDEO) != 0:
                raise RuntimeError(sdl.SDL_GetError().decode())
            sdl.SDL_SetHint(sdl.SDL_HINT_RENDER_SCALE_QUALITY, b'0')
            displays.append(Display(sdl, 'PyBoy ROM', 60))
            displays.append(Display(sdl, 'Rust port', 560))
        print(f"Inputs by {movie.get('author', 'unknown')}; same input index in both windows.", flush=True)
        st4_ran = []
        f71_phases = []
        scene_running = [False]
        scene_phases = []
        if phase_lock:
            pyboy.hook_register(0, 0x05D5, lambda _ctx: st4_ran.append(1), None)
            pyboy.hook_register(
                0, 0x0F71,
                lambda _ctx: f71_phases.append(pyboy.memory[0xFF97] & 7), None)
            # `$20A4` also runs with a zero scene id; only a nonzero id enters
            # the blocking script. `$21EE` returns ownership to the main loop.
            def scene_enter(_ctx):
                scene_running[0] = bool(pyboy.memory[0xA28E] | pyboy.memory[0xA28F])

            def scene_exit(_ctx):
                scene_running[0] = False

            pyboy.hook_register(0, 0x20A4, scene_enter, None)
            pyboy.hook_register(0, 0x21EE, scene_exit, None)
            # `$273D` is the RET from `$2728`, after it increments `$FF97`.
            def scene_resume(_ctx):
                if scene_running[0]:
                    scene_phases.append(pyboy.memory[0xFF97] & 7)

            pyboy.hook_register(0, 0x273D, scene_resume, None)
        print(f'Position comparison: {"disabled" if threshold is None else f"level/x/y, world tolerance {threshold}px"}; camera is diagnostic only; pixels are not compared.', flush=True)
        print(f'Phase lock: {"ROM-supplied phases (diagnostic alignment, not native clock parity)" if phase_lock else "off"}.', flush=True)
        print('Space pause/resume BOTH; Right step BOTH; Tab 1x/0.25x; Escape closes BOTH.', flush=True)
        if movie.get('source') == 'manual-pyboy':
            print('Manual PyBoy recording: ROM, boot ROM, core version, and settings match.', flush=True)
        else:
            print('PyBoy uses its built-in boot ROM unless --bootrom is supplied; this BizHawk movie may desync in PyBoy too.', flush=True)
        if args.port_initial_phase is not None:
            print(f'Diagnostic alignment: first native level initial movement phase {args.port_initial_phase}; this is not a shared-clock fix.', flush=True)
        paused, slow, running = args.paused, False, True
        divergence = None
        previous_native = None
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
                previous_divergence = divergence
                mask = frames[index]
                apply_buttons(pyboy, previous, mask)
                if phase_lock:
                    before_ff97 = pyboy.memory[0xFF97]
                    before_state = pyboy.memory[0xFF9B]
                    st4_ran.clear()
                    f71_phases.clear()
                    scene_phases.clear()
                if not pyboy.tick(1, True, False):
                    raise RuntimeError('PyBoy stopped before end of movie')
                if phase_lock:
                    phases = logic_phases(before_state, before_ff97, st4_ran, f71_phases,
                                          scene_phases)
                    for phase in phases:
                        bridge.stdin.write(f'{mask} {phase}\n'.encode())
                        bridge.stdin.flush()
                        if state_rows:
                            native_state = read_state(bridge.stdout)
                        native = read_frame(bridge.stdout)
                        native_step += 1
                    if watch and divergence is None:
                        divergence = check_divergence(
                            index, pyboy.memory, native_state, threshold, live=True,
                            player_ran=bool(f71_phases or st4_ran), native_step=native_step)
                else:
                    bridge.stdin.write(f'{mask}\n'.encode())
                    bridge.stdin.flush()
                    native_state = read_state(bridge.stdout) if state_rows else None
                    native = read_frame(bridge.stdout)
                    native_step += 1
                    # sMarioX/Y are copied from HRAM at the next state-$04 entry,
                    # so this row's ROM state matches the previous row's port state.
                    if watch and divergence is None and previous_native is not None:
                        divergence = check_divergence(
                            index - 1, pyboy.memory, previous_native, threshold,
                            native_step=previous_native_step)
                if divergence is not None and previous_divergence is None:
                    paused = True
                    print(f'FIRST DIVERGENCE at input {divergence["frame"]}: {divergence["detail"]}',
                          flush=True)
                    if args.headless:
                        previous = mask
                        previous_native = native_state
                        index += 1
                        break
                previous = mask
                previous_native = native_state
                previous_native_step = native_step
                index += 1
            if args.headless and index == count:
                break
            held = '+'.join(name for bit, name in BUTTONS if previous & bit) or 'none'
            status = 'end' if index == count else 'paused' if paused else '0.25x' if slow else '1x'
            label = f'input {index - 1 if index else "not started"} | {held} | {status}'
            if divergence:
                label += f' | DIVERGED dx={divergence["dx"]} dy={divergence["dy"]}'
            if displays:
                displays[0].draw(pyboy.screen.ndarray, f'PyBoy ROM | {label}')
                port_name = 'Rust port' if args.port_initial_phase is None else f'Rust port [initial phase {args.port_initial_phase}]'
                displays[1].draw(native, f'{port_name} | {label}')
                time.sleep(max(0, 1 / (15 if slow else 60) - (time.perf_counter() - started)))
        if index == len(frames) and movie.get('final_screen_sha256'):
            actual = hashlib.sha256(pyboy.screen.ndarray.tobytes()).hexdigest()
            if actual != movie['final_screen_sha256']:
                raise RuntimeError('PyBoy final screen does not match the recording')
            print('PyBoy final screen matches the recording exactly.', flush=True)
        if divergence:
            print(f'First sampled level/x/y mismatch beyond tolerance at input {divergence["frame"]} '
                  f'({divergence["detail"]}); playback paused there.', flush=True)
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
