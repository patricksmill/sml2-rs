#!/usr/bin/env python3
"""Coin-touch probe: what does the console do when Mario's side probes read a
`$48` coin cell?

`$1D75` (home.asm:3770) is called by the `$1E7C`/`$1E8D` wall probes for every
probed tile at or above `$38`; `sCollisionTile & $FE == $48` requests sfx 7,
runs `$1DA5` (writes the level map at the probe's own cell: `$48` -> `$60`,
`$49` -> `$7F`, both via `$2C0D` -> bank-1 `$5CF2`), and credits the BCD
counter (`inc a`/`daa` + `adc 0`, `$0999`-clamped). Level 1's only `$48` cells
are (24,231) and (24,232) (x 3688-3719, y 384-415) — in the air above the
496/480/464 tower, so the tower climb of `trajectory_capture12.py` is the one
route in the suite that reads them. The port collects them at sim frames
**6490** and **6496** (coins `00` -> `01` -> `02`, level-map cells (24,231)
and (24,232) in `layout_writes`).

This probe reproduces `trajectory_capture12.py`'s route verbatim (same prefix,
same inputs) and logs, per tick: `sCoinLow`/`sCoinHigh` ($A262/$A263),
`sCollisionTile` ($A20D), and the level-map bytes at the four cells around the
coins (row 24, cols 230-233; read from the SVBK-selected WRAM bank — `$FF70`
is logged too). At the end it prints every frame where the coin counter or a
cell changed, so the port's frames/count/write can be compared 1:1 against the
console. It writes `$OUT/probe_coin_touch.json` (the per-tick log) and never
writes a capture for the suite; `trajectory_capture16.py` (`traj16`, this same
route recorded per logic frame with `coins`/`coinshi` checks) is the gate
capture that grew out of it.
"""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'sml2decomp')))
OUT = '/home/pmill/.hermes/cache/scratch'
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

def slot(i):
    h = slots_hex()[i]
    return dict(state=int(h[16:18], 16), x=int(h[0:4], 16), y=int(h[6:10], 16),
                def_=int(h[10:12], 16), sprite=int(h[22:24], 16),
                fst=int(h[30:32], 16), ty=int(h[36:38], 16))

def walker_slot(minx=0):
    for k in range(16):
        s = slot(k)
        if s['state'] and s['def_'] == 9 and s['x'] > minx:
            return k, s
    return None, None

def state(keys):
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
        pan=rd(0xA2B3), slots=[],
        inv=rd(0xA217), sonobj=rd(0xA26B), lvl9b=rd(0xFF9B), star_hi=rd(0xA25E),
        pipe=rd(0xA221), tr=rd(0xA20E),
        # ---- the coin-touch observables this probe exists for -------------
        coinlo=rd(0xA262), coinho=rd(0xA263), ftile=rd(0xA20D),
        svbk=rd(0xFF70),
        cell_r24c230=rd(0xB000 + 24 * 256 + 230),
        cell_r24c231=rd(0xB000 + 24 * 256 + 231),
        cell_r24c232=rd(0xB000 + 24 * 256 + 232),
        cell_r24c233=rd(0xB000 + 24 * 256 + 233),
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

frames = []
DBG = {'next': 0}
def run(keys, n, tag=''):
    for k in keys:
        p.button_press(k)
    for _ in range(n):
        tick()
        frames.append(state(keys))
        if len(frames) >= DBG['next']:
            DBG['next'] = len(frames) + 240
            f = frames[-1]
            print(f'    ..f{len(frames)} mx={f["x"]} my={f["y"]} dead={f["dead"]} '
                  f'coin={f["coinho"]:02X}{f["coinlo"]:02X} pipe={f["pipe"]:02X}')
    for k in keys:
        p.button_release(k)
    if tag:
        f = frames[-1]
        print(f'  [{tag}] n={len(frames)} mx={f["x"]} my={f["y"]} hvel={f["hvel"]:02X} '
              f'dead={f["dead"]} pipe={f["pipe"]:02X} tr={f["tr"]:02X} '
              f'coin={f["coinho"]:02X}{f["coinlo"]:02X} '
              f'cells={f["cell_r24c230"]:02X},{f["cell_r24c231"]:02X},'
              f'{f["cell_r24c232"]:02X},{f["cell_r24c233"]:02X}')

def abort(msg):
    print('  ABORT:', msg)
    json.dump(frames, open(f'{OUT}/probe_coin_touch_fail.json', 'w'))
    raise SystemExit(1)

def check_dead():
    if frames[-1]['dead']:
        abort('Mario died')

def dump_slots(tag):
    print(f'  == slots @ {tag} (frames {len(frames)})')
    for k in range(16):
        s = slot(k)
        if s['state']:
            print(f'     s{k}: st={s["state"]} x={s["x"]:>4} y={s["y"]:>3} anim={s["def_"]:>2} '
                  f'spr={s["sprite"]:>3} fst={s["fst"]:>2} ty={s["ty"]:02X}')
    f = frames[-1]
    print(f'     mario mx={f["x"]} my={f["y"]} ground={f["ground"]}')

# =====================================================================
# capture9 prefix: stomp both walkers, staircase, raised floor, pipe descent
# =====================================================================
def stomp(slot_minx, label):
    run(['right', 'b'], 240)
    k, s = walker_slot(slot_minx)
    while k is None:
        run(['right', 'b'], 4)
        check_dead()
        k, s = walker_slot(slot_minx)
    print(f'  {label}: walker slot {k} at x={s["x"]} mx={frames[-1]["x"]}')
    while True:
        k, s = walker_slot(slot_minx)
        if k is None or frames[-1]['x'] - s['x'] >= -56:
            break
        run(['right', 'b'], 1)
        check_dead()
    run([], 1)
    for _ in range(40):
        if frames[-1]['hvel'] == 0x80:
            break
        run([], 1)
    while True:
        k, s = walker_slot(slot_minx)
        if k is None or frames[-1]['x'] - s['x'] >= -28:
            break
        run([], 1)
        check_dead()
    f = frames[-1]
    k, s = walker_slot(slot_minx)
    print(f'  {label}: jumping at dx={f["x"] - s["x"]} mx={f["x"]} wx={s["x"]}')
    run(['a'], 30)
    run([], 100, label + '-done')
    check_dead()

def hop(tag, rise_hold=9):
    run(['a'], rise_hold)
    run(['a', 'right'], 26)
    run(['a'], 10)
    run([], 25, tag)

def walk_until_x(target, tag, limit=120):
    for _ in range(limit):
        if frames[-1]['x'] >= target:
            break
        run(['right'], 1)
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]}')
    check_dead()

def walk_blocked(tag, limit=200):
    prev = -1
    for _ in range(limit):
        run(['right', 'b'], 2)
        check_dead()
        if frames[-1]['x'] == prev and frames[-1]['hvel'] == 0x80:
            break
        prev = frames[-1]['x']
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]}')

stomp(400, 'w1')
walk_blocked('wallA')
hop('hopA')
walk_until_x(870, 'walkA')
walk_until_x(895, 'dropA')
run([], 20)
walk_blocked('gapB')
hop('hopB')
walk_until_x(950, 'walkB')
run(['right'], 25)
run([], 20)
walk_blocked('wallFloor')
hop('hopFloor', 4)
run([], 1)
k, s = walker_slot(950)
print('  w2: walker slot', k, 'x=', s['x'] if s else None, 'mx=', frames[-1]['x'])
for _ in range(40):
    if frames[-1]['hvel'] == 0x80:
        break
    run([], 1)
while True:
    k, s = walker_slot(950)
    if k is None or frames[-1]['x'] - s['x'] >= -28:
        break
    run([], 1)
    check_dead()
f = frames[-1]
k, s = walker_slot(950)
dx = f['x'] - s['x'] if s else None
print(f'  w2: at dx={dx} mx={f["x"]} wx={s["x"] if s else None}')
if s and dx < -28:
    print(f'  w2: jumping at dx={dx}')
    run(['a'], 30)
    run([], 110, 'w2-done')
    check_dead()
else:
    print('  w2: walker is behind/past — no stomp needed')

walk_blocked('pipeWall')
run(['a'], 4)
run(['a', 'right'], 20)
run(['a', 'left'], 7)
run(['a'], 10)
run([], 30, 'hopPipe')
f = frames[-1]
print('after pipe hop: mx=%d my=%d ground=%d pipe=%02X' % (f['x'], f['y'], f['ground'], f['pipe']))
for _ in range(120):
    x = frames[-1]['x']
    lo = x & 0x1F
    if 0x15 <= lo <= 0x1B:
        break
    if lo < 0x15:
        run(['right'], 1)
    else:
        run(['left'], 1)
run([], 4, 'aligned')
f = frames[-1]
print('aligned: mx=%d (x&1F=%02X) my=%d pipe=%02X' % (f['x'], f['x'] & 0x1F, f['y'], f['pipe']))
run(['down'], 260, 'travel')
dump_slots('room landing')

# =====================================================================
# the coin room: exit via the second pipe (jump + UP under it)
# =====================================================================
run(['right'], 200, 'room walk')
f = frames[-1]
print('room: mx=%d my=%d ground=%d pipe=%02X' % (f['x'], f['y'], f['ground'], f['pipe']))

for _ in range(200):
    x = frames[-1]['x']
    if 1455 <= x <= 1462:
        break
    run(['right' if x < 1455 else 'left'], 1)
run([], 3, 'room aligned')
f = frames[-1]
print('room aligned: mx=%d (x&1F=%02X) my=%d tile=%02X pipe=%02X' % (
    f['x'], f['x'] & 0x1F, f['y'], rd(0xA20D), f['pipe']))

p.button_press('up')
p.button_press('a')
travel_started = False
for i in range(140):
    tick()
    frames.append(state(('up', 'a')))
    d = frames[-1]['pipe']
    if d == 0x20 and not travel_started:
        travel_started = True
        print(f'  travel up started f{len(frames)} x={frames[-1]["x"]} y={frames[-1]["y"]}')
    if travel_started and d != 0x20:
        print(f'  travel up ended f{len(frames)} x={frames[-1]["x"]} y={frames[-1]["y"]}')
        break
p.button_release('a')
p.button_release('up')
if not travel_started:
    abort('up travel never started')
run([], 20, 'surfaced')
f = frames[-1]
print('surface: mx=%d my=%d ground=%d pipe=%02X' % (f['x'], f['y'], f['ground'], f['pipe']))
dump_slots('after pipe exit')

# =====================================================================
# surface walk right through the objects
# =====================================================================
def walker_ahead(min_dx=-8):
    mx = frames[-1]['x']
    for k in range(16):
        s = slot(k)
        if s['state'] and s['def_'] == 9 and s['x'] > mx + min_dx:
            return k, s
    return None, None

def fight_walker(tag):
    ledge = frames[-1]['x'] > 2700
    while True:
        f = frames[-1]
        k, s = walker_ahead()
        if k is None or f['x'] - s['x'] >= (-45 if ledge else -56):
            break
        run(['right'], 1)
        check_dead()
    run([], 1)
    for _ in range(40):
        if frames[-1]['hvel'] == 0x80:
            break
        run([], 1)
    while True:
        k, s = walker_ahead()
        lim = -40 if ledge else -28
        if k is None or frames[-1]['x'] - s['x'] >= lim:
            break
        run([], 1)
        check_dead()
    f = frames[-1]
    k, s = walker_ahead()
    print(f'  [{tag}] jumping at dx={f["x"] - s["x"] if s else None} mx={f["x"]} ledge={ledge}')
    if ledge:
        run(['a', 'right'], 24)
        run(['a'], 20)
        run([], 120, tag + '-done')
    else:
        run(['a'], 30)
        run([], 110, tag + '-done')
    check_dead()

def hop_right(tag):
    run(['a'], 9)
    run(['a', 'right'], 26)
    run(['a'], 10)
    run([], 25, tag)

def drive_right(target, tag, limit=3000):
    stall = 0
    lastx = frames[-1]['x']
    while frames[-1]['x'] < target and limit > 0:
        limit -= 1
        f = frames[-1]
        k, s = walker_ahead()
        if k is not None and s['x'] - f['x'] < 100:
            fight_walker(f'{tag}-w')
            stall = 0
            continue
        if f['x'] == lastx and f['ground']:
            stall += 1
        else:
            stall = 0
        lastx = f['x']
        if stall >= 14:
            print(f'  [{tag}] stall at x={f["x"]} my={f["y"]} — hop')
            hop_right(f'{tag}-hop')
            stall = 0
            continue
        run(['right'], 1)
        check_dead()
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]}')
    dump_slots(tag)

drive_right(1640, 'pre-anim8')
drive_right(1760, 'over-plateau')
drive_right(2010, 'pre-arena')
dump_slots('pre-arena')

while frames[-1]['x'] < 2032:
    run(['right', 'b'], 1)
    check_dead()
run(['a', 'right', 'b'], 36, 'jump1')
lastx = frames[-1]['x']
for _ in range(400):
    f = frames[-1]
    if f['ground'] and f['y'] <= 420 and f['x'] >= 2144:
        break
    if f['x'] == lastx and f['ground']:
        hop_right('col2-hop')
        lastx = frames[-1]['x']
        continue
    lastx = f['x']
    run(['right'], 1)
    check_dead()
print('after col2 hop: mx=%d my=%d ground=%d' % (frames[-1]['x'], frames[-1]['y'], frames[-1]['ground']))
while frames[-1]['ground'] and frames[-1]['y'] <= 420 and frames[-1]['x'] < 2164:
    run(['right'], 1)
    check_dead()
run(['a', 'right', 'b'], 36, 'jump2')
run(['right', 'b'], 60, 'after-jump2')
dump_slots('post-arena')
check_dead()

drive_right(2460, 'pre-pair')
dump_slots('at-pair')
drive_right(2900, 'pre-anim23')
dump_slots('past-anim23')
drive_right(3090, 'end-zone')
dump_slots('end-zone')

# =====================================================================
# v11 tail: east past 3094 — up the tower (3616-3807), the coin cells
# =====================================================================
def probe_tiles(tag):
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]} '
          f'hvel={f["hvel"]:02X} vvel={f["vvel"]:02X} dead={f["dead"]} '
          f'coin={f["coinho"]:02X}{f["coinlo"]:02X}')

def jump_over(tag):
    f = frames[-1]
    k, s = walker_ahead()
    print(f'  [{tag}] jump-over at dx={f["x"] - s["x"] if s else None} '
          f'mx={f["x"]} my={f["y"]} wx={s["x"] if s else None} hvel={f["hvel"]:02X}')
    run(['a', 'right', 'b'], 40)
    run(['right', 'b'], 40)
    run([], 30, tag + '-done')
    check_dead()

def obj_ahead(kind, min_dx=-8):
    mx = frames[-1]['x']
    best = None
    for k in range(16):
        sl = slot(k)
        if sl['state'] and sl['def_'] == kind and sl['x'] > mx + min_dx:
            if best is None or sl['x'] < best[1]['x']:
                best = (k, sl)
    return best or (None, None)

def cross_bobber(tag):
    while True:
        f = frames[-1]
        k, s = obj_ahead(12)
        if k is None:
            return
        if s['x'] - f['x'] <= 45:
            break
        run(['right'], 1)
        check_dead()
    for _ in range(80):
        if frames[-1]['hvel'] == 0x80:
            break
        run([], 1)
        check_dead()
    for _ in range(300):
        f = frames[-1]
        k, s = obj_ahead(12)
        if k is None or s['x'] - f['x'] <= 24:
            break
        run(['right'], 1)
        check_dead()
        run([], 2)
    for _ in range(400):
        k, s = obj_ahead(12)
        if k is None:
            break
        if s['state'] == 0 and s['y'] >= 492:
            break
        run([], 1)
        check_dead()
    run([], 6, tag + '-wait')
    run(['right', 'b'], 30, tag + '-cross')
    probe_tiles(tag + '-passed')

def drive_east(target, tag, limit=2000):
    stall = 0
    lastx = frames[-1]['x']
    while frames[-1]['x'] < target and limit > 0:
        limit -= 1
        f = frames[-1]
        k, s = walker_ahead()
        if k is not None and 0 < s['x'] - f['x'] < 60 and f['ground']:
            jump_over(tag + '-w')
            stall = 0
            continue
        kb, sb = obj_ahead(12)
        if kb is not None and 0 < sb['x'] - f['x'] < 52 and f['ground']:
            cross_bobber(tag + '-b')
            stall = 0
            continue
        if f['x'] == lastx and f['ground']:
            stall += 1
        else:
            stall = 0
        lastx = f['x']
        if stall >= 14:
            print(f'  [{tag}] stall at x={f["x"]} my={f["y"]} — hop')
            hop_right(tag + '-hop')
            stall = 0
            continue
        run(['right'], 1)
        check_dead()
    probe_tiles(tag)

probe_tiles('east-start')
drive_east(3240, 'past-marker15')
drive_east(3480, 'past-walker3')
drive_east(3580, 'past-bobber3')
def hop_east(tag):
    run(['right', 'b'], 18)
    run(['a', 'right', 'b'], 36)
    run([], 2, tag)
    check_dead()

run([], 60, 'pre-tower-settle')
probe_tiles('pre-tower')

# The coin cells live at (24,231)/(24,232): during the tower climb the side
# probes read them and the counter ticks. Stop as soon as both are gone (the
# `$60` write makes the cells non-collectable, so this is self-limiting).
for i in range(24):
    f = frames[-1]
    if f['x'] >= 3800 or f['coinlo'] >= 2:
        break
    hop_east(f'tower-{i}')
    f = frames[-1]
    print(f'    ..tower{i:02d} mx={f["x"]} my={f["y"]} ground={f["ground"]} dead={f["dead"]} '
          f'coin={f["coinho"]:02X}{f["coinlo"]:02X} ftile={f["ftile"]:02X} '
          f'cells={f["cell_r24c231"]:02X},{f["cell_r24c232"]:02X}')
probe_tiles('past-tower')

json.dump(frames, open(f'{OUT}/probe_coin_touch.json', 'w'))
print('frames:', len(frames))

# --- report -----------------------------------------------------------------
print('\n=== coin-counter changes (frame, coin hi, coin lo, mario x/y) ===')
prev = None
for i, d in enumerate(frames):
    if d['coinlo'] != prev:
        print(f'  tick {i}: coin={d["coinho"]:02X}{d["coinlo"]:02X} '
              f'mx={d["x"]} my={d["y"]} ftile={d["ftile"]:02X} '
              f'cells(230..233)={d["cell_r24c230"]:02X} {d["cell_r24c231"]:02X} '
              f'{d["cell_r24c232"]:02X} {d["cell_r24c233"]:02X}')
        prev = d['coinlo']

print('\n=== level-map cell changes (frame, cell, old -> new) ===')
keys = ['cell_r24c230', 'cell_r24c231', 'cell_r24c232', 'cell_r24c233']
last = {k: None for k in keys}
for i, d in enumerate(frames):
    for k in keys:
        if last[k] is not None and d[k] != last[k]:
            print(f'  tick {i}: {k}: ${last[k]:02X} -> ${d[k]:02X} '
                  f'mx={d["x"]} my={d["y"]} coin={d["coinho"]:02X}{d["coinlo"]:02X}')
        last[k] = d[k]

f = frames[-1]
print(f'\nsvbk=0x{f["svbk"]:02X} last frame: mx={f["x"]} my={f["y"]} '
      f'coin={f["coinho"]:02X}{f["coinlo"]:02X} dead={f["dead"]}')
p.stop(save=False)
