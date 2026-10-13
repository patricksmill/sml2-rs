#!/usr/bin/env python3
"""Walker behavior trace — dumps slot 0 raw bytes + staging $FFD0-$FFEF +
object flags per frame around level 0's first walker spawn/activation.
Mirrors trajectory_capture4/5 navigation."""
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

frames = []
prev = None
log = open(f'{OUT}/walker_trace1.log', 'w')

def snap():
    slot = [rd(0xAD00 + i) for i in range(0x20)]
    slot1 = [rd(0xAD20 + i) for i in range(0x20)]
    stage = [rd(0xFFD0 + i) for i in range(0x20)]
    return dict(
        mx=rd(0xFFC2) | (rd(0xFFC3) << 8), my=rd(0xFFC0) | (rd(0xFFC1) << 8),
        camx=rd(0xFFCA) | ((rd(0xFFCB) & 0x0F) << 8),
        hvel=rd(0xA200), vvel=rd(0xA201),
        slot=slot, slot1=slot1, stage=stage,
        mf=rd(0xAF2F), fac=rd(0xAF2C), up=rd(0xAF08), ff0=rd(0xFFF0),
        done=rd(0xAF3E), sb=rd(0xAF40), ac=rd(0xAF41),
        hit=rd(0xAF2E), win=rd(0xAF2A))

def fmt_changes(off_base, a, b, names=None):
    out = []
    for i in range(32):
        if a[i] != b[i]:
            nm = names.get(i, '') if names else ''
            out.append(f'{off_base}+{i:02X}{nm}:{a[i]:02X}->{b[i]:02X}')
    return out

slotnames = {0: 'Xhi', 1: 'Xlo', 2: 'mk', 3: 'Yhi', 4: 'Ylo', 5: 'def', 6: 'rchi', 7: 'rclo',
             8: 'st', 9: 'scrY', 10: 'scrX', 11: 'spr', 12: 'defeat', 13: 'flip', 14: 'flash',
             15: 'fst', 16: 'ph', 17: 'p17', 18: 'type', 19: 'hbT', 20: 'hbB', 21: 'hbL', 22: 'hbR',
             23: 'ifl', 24: 't24', 25: 't25', 26: 'afr', 27: 'hitr', 28: 't28', 29: 'p29', 30: 'p30', 31: 'mask'}

def run(keys, n, dump_from, tag):
    global prev
    for k in keys:
        p.button_press(k)
    for _ in range(n):
        tick()
        idx = len(frames)
        s = snap()
        frames.append(s)
        if idx >= dump_from:
            if prev is None:
                log.write(f'f{idx:3d} {tag} mx={s["mx"]:5d} s0: ' +
                          ' '.join(f'{slotnames.get(i,"")}={s["slot"][i]:02X}' for i in range(32)) + '\n')
            else:
                ch = fmt_changes('s0', prev['slot'], s['slot'], slotnames)
                ch1 = fmt_changes('s1', prev['slot1'], s['slot1'], slotnames)
                chst = fmt_changes('stg', prev['stage'], s['stage'], slotnames)
                extra = []
                if s['mf'] != prev['mf']: extra.append(f'mf:{prev["mf"]:02X}->{s["mf"]:02X}')
                if s['fac'] != prev['fac']: extra.append(f'fac:{prev["fac"]:02X}->{s["fac"]:02X}')
                if s['up'] != prev['up']: extra.append(f'up:{prev["up"]:02X}->{s["up"]:02X}')
                if s['ff0'] != prev['ff0']: extra.append(f'ff0:{prev["ff0"]:02X}->{s["ff0"]:02X}')
                if s['done'] != prev['done']: extra.append(f'done:{prev["done"]:02X}->{s["done"]:02X}')
                if s['hit'] != prev['hit']: extra.append(f'hit:{prev["hit"]:02X}->{s["hit"]:02X}')
                if s['win'] != prev['win']: extra.append(f'win:{prev["win"]:02X}->{s["win"]:02X}')
                if s['sb'] != prev['sb'] or s['ac'] != prev['ac']:
                    extra.append(f'sb={s["sb"]} ac={s["ac"]}')
                log.write(f'f{idx:3d} {tag} mx={s["mx"]:5d} x={s["camx"]:5d} ' +
                          ' '.join(ch + ch1 + chst + extra) + '\n')
        prev = s
    for k in keys:
        p.button_release(k)

# Phase 1: run right until walker spawns+activates (~frame 300 like capture4)
run(['right'], 20, 9999, 'r')
run(['right', 'b'], 400, 200, 'rb')
run([], 30, 0, 'idle')
run(['a', 'right', 'b'], 6, 0, 'jump')
run(['right', 'b'], 50, 0, 'rb2')
run([], 60, 0, 'idle2')
log.close()
print('frames:', len(frames))
json.dump(frames, open(f'{OUT}/walker_trace1.json', 'w'))
print('saved walker_trace1.json / walker_trace1.log')
p.stop(save=False)
