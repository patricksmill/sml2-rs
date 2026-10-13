#!/usr/bin/env python3
"""Trajectory capture v5 — like v4 but stops short of level 0's first pipe
(x~441): run right 150 frames (spawns record 0 at x=472 + activates it),
idle, jump in place, then retreat left. Writes traj5.json."""
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

def objstate():
    slots = []
    for k in range(16):
        b = 0xAD00 + k * 0x20
        slots.append(dict(state=rd(b + 8),
                          x=(rd(b) << 8) | rd(b + 1),
                          y=(rd(b + 3) << 8) | rd(b + 4),
                          def_=rd(b + 5), sprite=rd(b + 0x0B),
                          rec=(rd(b + 6) << 8) | rd(b + 7)))
    return dict(
        slots=slots,
        sptr=(rd(0xAF1E) << 8) | rd(0xAF1F), spx=(rd(0xAF00) << 8) | rd(0xAF01),
        sdir=rd(0xAF22), standby=rd(0xAF40), active=rd(0xAF41),
        easy=rd(0xA2E4), odd=rd(0xFFF0) & 1)

def state(keys):
    s = dict(
        x=rd(0xFFC2) | (rd(0xFFC3) << 8), y=rd(0xFFC0) | (rd(0xFFC1) << 8),
        hvel=rd(0xA200), vvel=rd(0xA201), ground=rd(0xA214) != 0,
        dir_right=rd(0xA22B) == 1, camx=rd(0xFFCA), camxh=rd(0xFFCB) & 0x0F,
        camy=rd(0xFFC8), camyh=rd(0xFFC9) & 0x0F, phase=rd(0xFF97) & 7,
        timer=rd(0xA222), frame=rd(0xFFC6), edge=rd(0xFF98),
        limit=rd(0xA2D9) != 0, autoscroll=rd(0xA2C8),
        en_r=rd(0xA232) != 0, en_l=rd(0xA233) != 0,
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)
    s.update(objstate())
    return s

frames = []
def run(keys, n):
    for k in keys:
        p.button_press(k)
    for _ in range(n):
        tick()
        frames.append(state(keys))
    for k in keys:
        p.button_release(k)

seq = [
    ([], 10), (['right', 'b'], 150), ([], 40),
    (['a'], 6), ([], 60), (['left'], 70), ([], 46),
]
for keys, n in seq:
    run(keys, n)

print('frames:', len(frames))
for i, s in enumerate(frames[::15]):
    occ = [(k, sl['state'], sl['x'], sl['def_']) for k, sl in enumerate(s['slots']) if sl['state']]
    print(f"  f{i*15:3d} x={s['x']:5d} xw={s['x']:5d} vvel=${s['vvel']:02X} g={int(s['ground'])} "
          f"sptr=${s['sptr']:04X} sb={s['standby']} ac={s['active']} | {occ}")
json.dump(frames, open(f'{OUT}/traj5.json', 'w'))
print('saved traj5.json')
p.stop(save=False)
