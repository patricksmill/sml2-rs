#!/usr/bin/env python3
"""Trajectory capture v16 — the coin touch (`$1D75`/`$1DA5`), console-verified.

`$1D75` (home.asm:3770) is called by the `$1E7C`/`$1E8D` side probes for every
probed tile at or above `$38` (`$1413`, `$181C`, and the `$1189` side-pipe
travel all call those). `sCollisionTile & $FE == $48` requests sfx 7, runs
`$1DA5` — which rewrites the level map at the probe's own cell via `$2C0D` →
bank-1 `$5CF2`: `$48` → 96 (`$60`, empty air), `$49` → 127 (`$7F`) — and
credits the BCD counter (`inc a`/`daa` + `adc 0`, `$0999`-clamped).

Level 1's only `$48` cells are (24,231) and (24,232) (x 3688-3719, y 384-415):
in the air above the 496/480/464 tower (3616-3807, spikes at 3696-3727), so the
tower climb of `trajectory_capture12.py` is the one route in the suite whose
side probes read them. Measured on the console: the coin counter goes `00` → `01`
during the tick whose end state is mx=3691, my=383 and `01` → `02` at
mx=3700, my=365, with cells (24,231)/(24,232) reading `$48` before and `$60`
after — exactly the frames the port's `PlayerSim::coin_touch` fires on.

The route is `trajectory_capture12.py`'s verbatim prefix (its first 6522
per-tick records are field-for-field identical, checked in the probe run) up to
x >= 3800, the tower-climb exit; the east walk to 4026 is `traj12`'s own.

Records come from the `$05E4` hook (one per LOGIC frame, the capture14/15
method) — run `apply_pairing.py traj16_raw.json traj16.json` before replaying.
The recorded field set is traj12's plus the coin observables `coins`/`coinshi`
(`$A262`/`$A263`, BCD) — the checks the coin touch needs to be replay-verified.
"""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'sml2decomp')))
OUT = '/home/pmill/.hermes/cache/scratch'
p = PyBoy(f'{ROOT}/baserom.gb', window='null')
p.set_emulation_speed(0)

TICKS = {'n': 0}
def rd(a): return p.memory[a]
def tick(n=1):
    for _ in range(n):
        p.tick()
        TICKS['n'] += 1
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
        inv=rd(0xA217), sonobj=rd(0xA26B), lvl9b=rd(0xFF9B), star_hi=rd(0xA25E),
        # the coin touch's observables ($1D75's credit) ----------------------
        coins=rd(0xA262), coinshi=rd(0xA263),
        pipe=rd(0xA221), tr=rd(0xA20E),
        tick=TICKS['n'],
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

# --- the records: one per LOGIC frame ($05E4), keys as held during that frame --
REC = []
CUR = set()
def frame_hook(_ctx):
    REC.append(state(CUR))

p.hook_register(0, 0x05E4, frame_hook, None)

def held(keys):
    CUR.clear()
    CUR.update(keys)

# The route list is the per-tick sample trajectory_capture12.py used, so every
# route decision below reads exactly what capture12 read.
frames = []
DBG = {'next': 0}
def run(keys, n, tag=''):
    for k in keys:
        p.button_press(k)
    held(keys)
    for _ in range(n):
        tick()
        frames.append(state(keys))
        if len(frames) >= DBG['next']:
            DBG['next'] = len(frames) + 240
            f = frames[-1]
            print(f'    ..f{len(frames)} mx={f["x"]} my={f["y"]} dead={f["dead"]} '
                  f'pipe={f["pipe"]:02X} coin={f["coinshi"]:02X}{f["coins"]:02X}')
    for k in keys:
        p.button_release(k)
    held(())
    if tag:
        f = frames[-1]
        print(f'  [{tag}] n={len(frames)} mx={f["x"]} my={f["y"]} hvel={f["hvel"]:02X} '
              f'dead={f["dead"]} pipe={f["pipe"]:02X} tr={f["tr"]:02X} '
              f'coin={f["coinshi"]:02X}{f["coins"]:02X}')

def abort(msg):
    print('  ABORT:', msg)
    json.dump(frames, open(f'{OUT}/traj16_fail.json', 'w'))
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

# jump + UP; keep UP held while the travel runs (dir $20)
p.button_press('up')
p.button_press('a')
held(('up', 'a'))
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
held(())
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
# v11 tail: east past 3094 — up the tower (3616-3807); the $48 coin cells
# =====================================================================
def probe_tiles(tag):
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]} '
          f'hvel={f["hvel"]:02X} vvel={f["vvel"]:02X} dead={f["dead"]} '
          f'coin={f["coinshi"]:02X}{f["coins"]:02X}')

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
    # 18 frames of run-up: the 36-frame jump only clears the 33 px spike gap
    # (tiles 231-232 at row 29) if hvel is already at run speed when it starts.
    run(['right', 'b'], 18)
    run(['a', 'right', 'b'], 36)
    run([], 2, tag)
    check_dead()

run([], 60, 'pre-tower-settle')
probe_tiles('pre-tower')
# Hops climb the tower; the side probes read the $48 cells at (24,231)/(24,232)
# during the climb, so this is where `$1D75` fires (the console coin report is
# printed after the run). Stop past the tower (x >= 3800) like traj12.
for i in range(24):
    f = frames[-1]
    if f['x'] >= 3800:
        break
    hop_east(f'tower-{i}')
    f = frames[-1]
    print(f'    ..tower{i:02d} mx={f["x"]} my={f["y"]} ground={f["ground"]} dead={f["dead"]} '
          f'coin={f["coinshi"]:02X}{f["coins"]:02X} ftile={rd(0xA20D):02X} '
          f'cell231={rd(0xB000 + 24 * 256 + 231):02X} cell232={rd(0xB000 + 24 * 256 + 232):02X}')
probe_tiles('past-tower')
run([], 30, 'settle-after-tower')

print('per-tick route frames:', len(frames), 'logic-frame records:', len(REC))
json.dump(REC, open(f'{OUT}/traj16_raw.json', 'w'))
print(f'saved traj16_raw.json ({len(REC)} records) — now run '
      'apply_pairing.py traj16_raw.json traj16.json')

# --- the coin report (console side) -----------------------------------------
print('\n=== coin-counter changes over the logic-frame records ===')
prev = None
for i, d in enumerate(REC):
    if d['coins'] != prev:
        print(f'  rec {i} (tick {d["tick"]}): coin={d["coinshi"]:02X}{d["coins"]:02X} '
              f'mx={d["x"]} my={d["y"]}')
        prev = d['coins']
print('=== level-map cells (live reads; the route prints them per tower hop) ===')
print(f'  cell(24,231)={rd(0xB000 + 24 * 256 + 231):02X} '
      f'cell(24,232)={rd(0xB000 + 24 * 256 + 232):02X} (expect 60 60 = the $1DA5 write)')
f = frames[-1]
print(f'\nlast route frame: mx={f["x"]} my={f["y"]} coin={f["coinshi"]:02X}{f["coins"]:02X} '
      f'dead={f["dead"]} pipe={f["pipe"]:02X} tr={f["tr"]:02X}')
p.stop(save=False)
