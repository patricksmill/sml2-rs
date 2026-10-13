#!/usr/bin/env python3
"""Trajectory capture v6 — the walker's full life: spawn/activate, walk left
while Mario approaches, a standing jump onto it (stomp → anim 10), the
squashed countdown and removal. Records the raw 32 slot bytes + engine
globals per frame. Writes traj6.json."""
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

def slots_hex():
    return [''.join(f'{rd(0xAD00 + k * 0x20 + j):02X}' for j in range(0x20)) for k in range(16)]

def wx():
    h = slots_hex()[0]
    return int(h[0:4], 16)

def state(keys):
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
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

frames = []
def run(keys, n, tag=''):
    for k in keys:
        p.button_press(k)
    for _ in range(n):
        tick()
        frames.append(state(keys))
    for k in keys:
        p.button_release(k)
    if tag:
        f = frames[-1]
        print(f'  [{tag}] n={len(frames)} mx={f["x"]} my={f["y"]} hvel=${f["hvel"]:02X} '
              f'vvel=${f["vvel"]:02X} wx={wx()} dx={f["x"]-wx()} dead={f["dead"]} '
              f'w0={f["slot_bytes"][0][:24]}')

# --- run right until the walker is ~56px ahead, then stop ---
run(['right'], 20)
run(['right', 'b'], 240, 'run')
while frames[-1]['x'] - wx() < -56:
    run(['right', 'b'], 1)
run([], 1, 'stop')
print('stopped: mx', frames[-1]['x'], 'wx', wx())

# --- let hvel settle to $80 (stationary standing jump) ---
for _ in range(40):
    if frames[-1]['hvel'] == 0x80:
        break
    run([], 1)
print('settled: mx', frames[-1]['x'], 'hvel', hex(frames[-1]['hvel']), 'dx', frames[-1]['x'] - wx())

# --- wait for dx >= -28, then jump (A held 30 frames = no jump cut) ---
while frames[-1]['x'] - wx() < -28:
    run([], 1)
run(['a'], 30, 'jump')
run([], 150, 'post')
json.dump(frames, open(f'{OUT}/traj6.json', 'w'))
print('frames:', len(frames))
f = frames[-1]
print('final: mx=%d dead=%d mb=%02X slot0=%s' % (f['x'], f['dead'], f['mb'], f['slot_bytes'][0]))
print('saved traj6.json')
p.stop(save=False)
