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
                if key not in {"step", "fff0", "update_phase", "standing", "scene", "owns", "pc",
                               "boss_x", "boss_state", "door_phase", "door_parity",
                               "platform_sprite", "tile_anim", "writes"}:
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


def logic_phases(before_state, before_ff97, st4_phases, scene_phases=(),
                 native_in_level=False):
    """Port steps for one hardware frame while phase-lock is on.

    Every state-`$04` entry (`$05D5`) is one gameplay update, whether it reaches
    the player routine `$0F71` (its only caller) or not: the screen-transition
    branch and the death arc at `$0640` skip it. The phase argument is
    `$FF97 & 7` sampled at that entry; `$FF97` only changes in the main loop's
    halt wait, so `$0F71` would see the same value. `$FF97` itself is not a
    reliable count: a lag frame increments it zero times, and a frame that
    finishes two waits increments it twice, with one update after each.
    Blocking scenes (`$20A4` to `$21EE`) resume on each `$2728` return,
    bypassing `$05D5`. Append those steps in execution order, including
    multiple waits completed in one hardware frame. Outside a level, step once
    per hardware frame; in a level with neither kind of update, do not step.
    If the native loader has finished before ROM states $02/$03 return, hold
    it until $05D5: a loader tick must not become a native gameplay update.
    """
    if st4_phases:
        phases = list(st4_phases)
    elif native_in_level and before_state in (2, 3):
        phases = []
    elif before_state != 4:
        phases = [(before_ff97 + 1) & 7]
    else:
        phases = []
    return phases + list(scene_phases)


# `ReadJoypad` ($1FD2) packs hKeysHeld as A, B, Select, Start, Right, Left, Up, Down.
ROM_KEY_BITS = (16, 32, 128, 64, 1, 2, 4, 8)


def rom_keys_mask(held):
    """`hKeysHeld` in the movie's button-mask layout."""
    return sum(bit for k, bit in enumerate(ROM_KEY_BITS) if held >> k & 1)


def frame_read(carried, early_reads, latch):
    """`hKeysHeld` for the once-per-frame step outside a level.

    The main loop reads the joypad just after its VBlank wait, and PyBoy's tick
    ends at the start of the frame, so the read lands either at the end of a
    tick (LY >= 144) or at the start of the next (LY 0). Either way the state
    handler runs in the frame after the boundary. A read is therefore this
    frame's when it happened here before line 144 (`early_reads`), or at the
    end of the previous tick (`carried`). The last one wins; without one, the
    latest latch stands.
    """
    reads = ([] if carried is None else [carried]) + list(early_reads)
    return reads[-1] if reads else latch


# The `rDIV` reads the port takes in order (`MapSim::div_reads`), hooked on
# the instruction after each read so A holds the value: `$3C13E`'s two
# (`ld c, 4 / ld a, [$FF00+c]`), the Space blink `$3DB40`, the Turtle patch
# `$3DE25`, and bell room B's two rung picks `$60481`/`$604C1`.
DIV_READ_HOOKS = ((0x0F, 0x4141), (0x0F, 0x4160), (0x0F, 0x5B50), (0x0F, 0x5E2F),
                  (0x18, 0x4484), (0x18, 0x44C4))

# Witch appearance ($97FE) and fire trajectory ($979D), after `ld a,[$FF04]`.
BOSS_DIV_READ_HOOKS = ((2, 0x5801), (2, 0x57A0))


def boss_div_field(reads, step):
    """One witch decision per gameplay update, paired with that update only."""
    if step >= len(reads) or not reads[step]:
        return ''
    if len(reads[step]) != 1:
        raise RuntimeError('multiple witch divider reads in one gameplay update')
    return f' boss_div={reads[step][0]}'


def frame_timing(iterations, frame):
    """`(ff97, div_reads)` for the main-loop iterations belonging to `frame`.

    Each iteration records the frame it belongs to by `frame_read`'s rule (a
    dispatch at LY >= 144 belongs to the next frame), its `$FF97`, and the
    `rDIV` reads made until the next dispatch. Several iterations in one frame
    keep the last `$FF97` and all reads in order; none gives `None`.
    """
    mine = [it for it in iterations if it['frame'] == frame]
    if not mine:
        return None
    return mine[-1]['ff97'], [value for it in mine for value in it['reads']]


def step_masks(mask, phases, st4_keys, frame_held=None):
    """Buttons for each port step from `logic_phases`.

    The ROM acts on `hKeysHeld` from its last `ReadJoypad` (`$38B9` runs right
    after the read), which can trail the movie row by a frame: the read can
    miss a row just applied, and once a frame overruns the hardware tick the
    next read happens before the next row. So a gameplay update uses the keys
    latched at its `$05D5` entry, and the once-per-frame step outside a level
    uses `frame_held`, from `frame_read`. Scene steps, and steps before any
    read, use the movie row.
    """
    keys = [mask if held is None else rom_keys_mask(held) for held in st4_keys]
    if not st4_keys and frame_held is not None and len(phases) > 0:
        keys = [rom_keys_mask(frame_held)]
    return keys + [mask] * (len(phases) - len(keys))


# Every address `check_divergence` reads.
SNAPSHOT_ADDRS = (0xFF9B, 0xA269, 0xFFC0, 0xFFC1, 0xFFC2, 0xFFC3, 0xFFC8, 0xFFC9,
                  0xFFCA, 0xFFCB, 0xFFF0, 0xAF08, 0xA26B, 0xA28E, 0xA28F)


class Snapshot(dict):
    """Memory captured at one ROM point; other addresses read live."""
    def __init__(self, memory, values):
        super().__init__(values)
        self.memory = memory

    def __missing__(self, address):
        return self.memory[address]


def completed_sample(memory, events, states):
    """Pair the last completed ROM update with the port state after its step.

    `events` are this hardware frame's hooks in execution order: `u` ($05D5)
    and `d` with the memory captured at `$028D`, the main loop's
    point after the state handler returns. A lagging ROM frame can straddle the
    tick boundary: the next update has started (and the port has stepped for
    it), but its `$028D` lands in the next frame, so live HRAM can hold half an
    update. Compare the last `d` instead, against the port state from before
    any step still in progress. `states` holds `(state, step)` from before this
    frame and after each of its steps. Without a `d`, sample live memory.
    """
    done = [k for k, (kind, _) in enumerate(events) if kind == 'd']
    if not done:
        return memory, states[-1]
    pending = sum(kind == 'u' for kind, _ in events[done[-1] + 1:])
    return (Snapshot(memory, events[done[-1]][1]),
            states[max(0, len(states) - 1 - pending)])


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
        phase_lock = not args.no_phase_lock
        state_rows = watch or args.debug_state or phase_lock
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
        st4_phases = []
        st4_keys = []
        st4_divs = []
        scene_running = [False]
        scene_phases = []
        joypad_latch = [None]
        late_read = [None]
        main_loop = [False]
        iterations = []
        tick = [0]
        early_reads = []
        events = []
        if phase_lock:
            def state4_entry(_ctx):
                st4_phases.append(pyboy.memory[0xFF97] & 7)
                st4_keys.append(joypad_latch[0])
                st4_divs.append([])
                events.append(('u', None))

            pyboy.hook_register(0, 0x05D5, state4_entry, None)

            def boss_div_read(_ctx):
                if st4_divs:
                    st4_divs[-1].append(pyboy.register_file.A)

            for bank, address in BOSS_DIV_READ_HOOKS:
                pyboy.hook_register(bank, address, boss_div_read, None)

            def joypad_read(_ctx):
                held = pyboy.memory[0xFF80]
                joypad_latch[0] = held
                if pyboy.memory[0xFF44] >= 144:
                    late_read[0] = held
                else:
                    early_reads.append(held)

            pyboy.hook_register(0, 0x38B9, joypad_read, None)

            # The port starts on the title loader (state `$00`); the boot ROM and
            # `Init` run first, so hold the port until the main loop dispatches.
            def dispatch(_ctx):
                main_loop[0] = True
                ly = pyboy.memory[0xFF44]
                iterations.append({'frame': tick[0] + (ly >= 144),
                                   'ff97': pyboy.memory[0xFF97], 'reads': []})
                del iterations[:-4]

            def div_read(_ctx):
                if iterations:
                    iterations[-1]['reads'].append(pyboy.register_file.A)

            for bank, address in DIV_READ_HOOKS:
                pyboy.hook_register(bank, address, div_read, None)

            pyboy.hook_register(0, 0x0279, dispatch, None)

            def post_state(_ctx):
                events.append(('d', {a: pyboy.memory[a] for a in SNAPSHOT_ADDRS}))

            pyboy.hook_register(0, 0x028D, post_state, None)
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
                    st4_phases.clear()
                    st4_keys.clear()
                    st4_divs.clear()
                    scene_phases.clear()
                    events.clear()
                    carried, late_read[0] = late_read[0], None
                    early_reads.clear()
                    states = [(native_state, native_step)]
                    tick[0] = index
                if not pyboy.tick(1, True, False):
                    raise RuntimeError('PyBoy stopped before end of movie')
                if phase_lock:
                    phases = (logic_phases(before_state, before_ff97, st4_phases, scene_phases,
                                           native_in_level=native_state is not None)
                              if main_loop[0] else [])
                    has_frame_step = not st4_phases and len(phases) > len(scene_phases)
                    predicted = (frame_read(carried, early_reads, joypad_latch[0])
                                 if len(phases) > len(scene_phases) else None)
                    timing = frame_timing(iterations, index) if has_frame_step else None
                    for k, (phase, keys) in enumerate(
                            zip(phases, step_masks(mask, phases, st4_keys, predicted))):
                        extra = ''
                        if k == 0 and timing is not None:
                            extra = f' f97={timing[0]} div={",".join(map(str, timing[1]))}'
                        extra += boss_div_field(st4_divs, k)
                        bridge.stdin.write(f'{keys} {phase}{extra}\n'.encode())
                        bridge.stdin.flush()
                        if state_rows:
                            native_state = read_state(bridge.stdout)
                        native = read_frame(bridge.stdout)
                        native_step += 1
                        states.append((native_state, native_step))
                    if watch and divergence is None:
                        sample, (paired, paired_step) = completed_sample(
                            pyboy.memory, events, states)
                        divergence = check_divergence(
                            index, sample, paired, threshold, live=True,
                            player_ran=bool(st4_phases), native_step=paired_step)
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
