#!/usr/bin/env python3
"""Trajectory capture v7 — after stomping the walker, walk right so record 1
(anim 1, x=584, on a 2-tile platform at y=448) spawns/activates, then watch
it fall/land and pace the platform (ledge turns both ways). Writes traj7.json."""
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

def wx(i=0):
    return int(slots_hex()[i][0:4], 16)

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
        s1 = f['slot_bytes'][1]
        print(f'  [{tag}] n={len(frames)} mx={f["x"]} my={f["y"]} dead={f["dead"]} '
              f'w0x={wx(0)} s1st={int(s1[16:18],16)} s1x={int(s1[0:4],16)} s1y={int(s1[6:10],16)} '
              f's1fst={int(s1[30:32],16)} s1spr={s1[22:24]}')

# --- stomp the walker (same recipe as capture 6) ---
run(['right'], 20)
run(['right', 'b'], 240, 'run')
while frames[-1]['x'] - wx(0) < -56:
    run(['right', 'b'], 1)
run([], 1, 'stop')
for _ in range(40):
    if frames[-1]['hvel'] == 0x80:
        break
    run([], 1)
while frames[-1]['x'] - wx(0) < -28:
    run([], 1)
run(['a'], 30, 'jump')
run([], 60, 'post-stomp')

# --- walk right toward the platform (record 1 at x=584) ---
while frames[-1]['x'] < 536:
    run(['right', 'b'], 2)
run([], 1, 'stop2')
for _ in range(40):
    if frames[-1]['hvel'] == 0x80:
        break
    run([], 1)
f = frames[-1]
print('at', f['x'], 'cam', f['camx'] | ((f['camxh'] & 0xF) << 8), 'record1 status:',
      f['slot_bytes'][1][:40])

# --- watch the pacer for ~350 frames ---
run([], 350, 'watch')
json.dump(frames, open(f'{OUT}/traj7.json', 'w'))
print('frames:', len(frames))
f = frames[-1]
print('final: mx=%d dead=%d s1=%s' % (f['x'], f['dead'], f['slot_bytes'][1]))
print('saved traj7.json')
p.stop(save=False)
