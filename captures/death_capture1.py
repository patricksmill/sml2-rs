#!/usr/bin/env python3
"""Trajectory capture v10 — the death sequence, sampled at the game's LOGIC
frame boundary.

Why a hook: during the dead branch (`$0640`) the per-frame work overruns the
VBlank budget, so the game runs 2 logic frames in one VBlank every ~8 frames.
A tick-based capture (traj4's method) merges those pairs and can't be replayed
1:1. Hooking `$05E4` (the in-level frame loop entry) samples exactly one frame
per logic iteration, which is what the sim reproduces.

Drive: run right into level 1's walker (harmful contact) → the death arc →
the fade (`$FF9B = 9`). Trimmed at the level reload.
"""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'sml2decomp')))
OUT = '/home/pmill/.hermes/cache/scratch/wp7_extract'
p = PyBoy(f'{ROOT}/baserom.gb', window='null')
p.set_emulation_speed(0)

def rd(a): return p.memory[a]
def tick(n=1):
    for _ in range(n): p.tick()
def tap(btn, hold=6, gap=4):
    p.button_press(btn); tick(hold); p.button_release(btn); tick(gap)

held = {'right': False, 'left': False, 'up': False, 'down': False, 'a': False, 'b': False}

def slots_hex():
    return [''.join(f'{rd(0xAD00 + k * 0x20 + j):02X}' for j in range(0x20)) for k in range(16)]

def state():
    slots = []
    for k in range(16):
        b = 0xAD00 + k * 0x20
        slots.append(dict(state=rd(b + 8), x=(rd(b) << 8) | rd(b + 1),
                          y=(rd(b + 3) << 8) | rd(b + 4), def_=rd(b + 5),
                          sprite=rd(b + 0x0B), rec=(rd(b + 6) << 8) | rd(b + 7)))
    return dict(
        x=rd(0xFFC2) | (rd(0xFFC3) << 8), y=rd(0xFFC0) | (rd(0xFFC1) << 8),
        hvel=rd(0xA200), vvel=rd(0xA201), ground=rd(0xA214) != 0,
        dir_right=rd(0xA22B) == 1, camx=rd(0xFFCA), camxh=rd(0xFFCB) & 0x0F,
        camy=rd(0xFFC8), camyh=rd(0xFFC9) & 0x0F, phase=rd(0xFF97) & 7,
        timer=rd(0xA222), frame=rd(0xFFC6), edge=rd(0xFF98),
        limit=rd(0xA2D9) != 0, autoscroll=rd(0xA2C8),
        en_r=rd(0xA232) != 0, en_l=rd(0xA233) != 0,
        sptr=rd(0xAF1E) << 8 | rd(0xAF1F), spx=rd(0xAF00) << 8 | rd(0xAF01),
        sdir=rd(0xAF22), standby=rd(0xAF40), active=rd(0xAF41),
        easy=rd(0xA2E4), odd=rd(0xFFF0) & 1,
        ff0=rd(0xFFF0), uphase=rd(0xAF08), mf=rd(0xAF2F), fac=rd(0xAF2C),
        dead=rd(0xA224), sbt=rd(0xA27A), scc=rd(0xA279), mb=rd(0xA28D),
        pan=rd(0xA2B3), slot_bytes=slots_hex(), slots=slots,
        pipe=rd(0xA221), tr=rd(0xA20E),
        pcx=(rd(0xAF24) << 8) | rd(0xAF25),
        right=held['right'], left=held['left'], up=held['up'],
        down=held['down'], a=held['a'], b=held['b'])

frames = []
def frame_hook(_ctx):
    frames.append(state())

p.hook_register(0, 0x05E4, frame_hook, None)

# --- nav to level 0 (title -> file select -> hub -> level) ---
for f in range(3000):
    tick()
    if f > 300 and rd(0xFF9B) == 1: break
tap('start')
for _ in range(900):
    tick()
    if rd(0xA277) == 1 and rd(0xA278) == 0: break
p.memory[0xA000 + 67] = 5; p.memory[0xA000 + 69] = 5
tap('start')
for _ in range(1500):
    tick()
    if rd(0xFF9B) == 0x0C: break
for _ in range(200):
    if rd(0xA277) == 1 and rd(0xA278) == 0: break
    tick()
tap('a')
for f in range(1500):
    tick()
    if rd(0xFF9B) == 4: break
    if f and f % 90 == 0: tap('a', hold=4, gap=2)
tick(140)
print('level start: logic frames so far =', len(frames), 'mx=', rd(0xFFC2) | (rd(0xFFC3) << 8))

# --- run right into the walker ---
held['right'] = True; held['b'] = True
p.button_press('right'); p.button_press('b')
for i in range(400):
    tick()
    if rd(0xA224) != 0:
        print('hit: dead=', rd(0xA224), 'at logic frame', len(frames))
        break
held['right'] = False; held['b'] = False
p.button_release('right'); p.button_release('b')

# --- let the death arc + fade play out ---
for i in range(330):
    tick()

print('total logic frames:', len(frames))
f = frames[-1]
print('final: x=%d y=%d dead=%02X frame=%d' % (f['x'], f['y'], f['dead'], f['frame']))

# trim at the level reload (x back at the spawn AFTER the death)
seen_dead = False
trim = len(frames)
for i, fr in enumerate(frames):
    if fr['dead'] != 0:
        seen_dead = True
    elif seen_dead and fr['x'] < 100:
        trim = i
        print('reload detected at logic frame', i)
        break
# Save RAW (sampled at each $05E4 entry = the state at the START of a logic
# frame). Pairing happens offline: the sim's frame k replays the console's
# logic frame k-1, so it needs that frame's keys with the record-k state.
json.dump(frames[:trim], open(f'{OUT}/death1_raw.json', 'w'))
print('saved death1_raw.json (%d frames)' % trim)
p.stop(save=False)
