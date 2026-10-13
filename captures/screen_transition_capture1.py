#!/usr/bin/env python3
"""Trajectory capture v12 — traj10's run continued east of x=3094: the anim-15
marker (3192), walker (3352), bobber (3456), then the 496/480/464 staircase
tower (3616-3807, spikes $81 at 3696-3727) climbed with repeated running hops,
walkers, the fallwalker at 3744 and the anim-22 pose-86 object at 4008. Stops
short of the exit tile at x=4064.

Prefix is byte-identical to trajectory_capture11.py, so traj12 reproduces
traj11's first 6470 frames and then keeps going. Records
sPipeTravelDirection ($A221) per frame; aborts on death with a dump."""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', '/home/pmill/sml2decomp')
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
        pipe=rd(0xA221), tr=rd(0xA20E),
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

frames = []
DBG = {'next': 0}

# --- the records: one per LOGIC frame ($05E4), keys as held during that frame
REC = []
CUR = set()
def frame_hook(_ctx):
    REC.append(state(CUR))
p.hook_register(0, 0x05E4, frame_hook, None)

def run(keys, n, tag=''):
    for k in keys:
        p.button_press(k)
    CUR.clear(); CUR.update(keys)
    for _ in range(n):
        tick()
        frames.append(state(keys))
        if len(frames) >= DBG['next']:
            DBG['next'] = len(frames) + 240
            f = frames[-1]
            print(f'    ..f{len(frames)} mx={f["x"]} my={f["y"]} dead={f["dead"]} pipe={f["pipe"]:02X}')
    for k in keys:
        p.button_release(k)
    CUR.clear()
    if tag:
        f = frames[-1]
        print(f'  [{tag}] n={len(frames)} mx={f["x"]} my={f["y"]} hvel={f["hvel"]:02X} '
              f'dead={f["dead"]} pipe={f["pipe"]:02X} tr={f["tr"]:02X}')

def abort(msg):
    print('  ABORT:', msg)
    json.dump(frames, open(f'{OUT}/traj12_fail.json', 'w'))
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
# NEW: exit the coin room via the second pipe (jump + UP under it)
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
CUR.clear(); CUR.update(('up', 'a'))
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
CUR.clear()
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
    """Approach the nearest walker, settle, stomp it (capture9 recipe, with a
    drift-into-it variant for the low-ledge section where a straight jump
    bonks the overhang at y=454)."""
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
    """Walk right to x >= target: stomp walkers ahead, hop when stalled."""
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
drive_right(1760, 'over-plateau')   # hops onto the plateau, crosses it
drive_right(2010, 'pre-arena')      # floor west of pipe-1 + its bobber
dump_slots('pre-arena')

# Arena (exact geometry): pipe-1 x2080-2111 (top 464, bobber-1 at 2096) →
# column-2 x2144-2175 (top 448) → pipe-2 x2208-2239 (top 464, bobber-2 at
# 2224) → column-3 x2272-2303 (top 448). Bobber box: marioY in
# [objY-51, objY-13), dx in [-19, +2). Airborne at y <= 412 is safe against
# every phase, so cross each x-band high in a running jump.
# Jump #1: from ~2032 over pipe-1/bobber-1, landing in the 2112-2143 gap.
while frames[-1]['x'] < 2032:
    run(['right', 'b'], 1)
    check_dead()
run(['a', 'right', 'b'], 36, 'jump1')
# Walk/hop onto column-2, then stop on its top near the right edge.
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
# Jump #2: from column-2's top edge over bobber-2, onto pipe-2's top/past it.
run(['a', 'right', 'b'], 36, 'jump2')
run(['right', 'b'], 60, 'after-jump2')
dump_slots('post-arena')
check_dead()

drive_right(2460, 'pre-pair')       # past the hopper/item pair (harmless)
dump_slots('at-pair')
drive_right(2900, 'pre-anim23')     # past walkers + the anim-23 ledge
dump_slots('past-anim23')
drive_right(3090, 'end-zone')       # past the anim-1s and anim-18
dump_slots('end-zone')

# =====================================================================
# NEW (v11): continue east past traj10's stop through the level-0 objects
# east of x=3094 — up the 496/480/464 staircase tower (3616-3807, spikes at
# 3696-3727), over the fallwalker at 3744, then the pose-86 object at 4008.
# =====================================================================
SHOT = {'next': 0}
def shot(tag):
    f = frames[-1]
    path = f'{OUT}/traj12_{len(frames):05d}_{tag}.png'
    p.screen.image.save(path)
    print(f'    shot {path}  mx={f["x"]} my={f["y"]} ground={f["ground"]} dead={f["dead"]}')

def probe_tiles(tag):
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]} '
          f'hvel={f["hvel"]:02X} vvel={f["vvel"]:02X} dead={f["dead"]}')

def jump_over(tag):
    """Running jump east over a walker that is closing in. Standing still and
    settling first (traj10's recipe) loses: the walker reaches Mario while he
    decelerates and the side contact is fatal (mx=3312/wx=3331 walked into him
    during the settle). A straight-up jump is no better — the walker at 19 px
    is already inside the box. So: jump *over* it."""
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
    """Bobber (anim 12, `$584F3`/`$5850C`): state 0 writes type `$80` and
    returns (harmless item) while `dist_mario_x < $18` (24 px); only when Mario
    is OUTSIDE 24 px does it arm state 1 with type `$44` (bit 6 → `$2E6F` kill)
    and rise 1 px/frame for 32 frames. Its harm box is x in [objX-19, objX+2),
    y in [objY-51, objY-25) — 38 px tall, and Mario on the 448 ledge (y=436) is
    inside that y-band at BOTH ends of the 28 px travel, so no jump timing or
    crossing height wins: earlier attempts died at exactly objY-51.
    Derivation: park inside the 24 px trigger but OUTSIDE the 21 px box, wait
    for the object to cycle into state 0 (it then latches type `$80` and stays),
    then walk through — contact sets sPendingItemId = 0, which `$3451` drops."""
    # 1) close to dx=45 (outside the 24 px trigger) then brake to a standstill:
    #    parking while still coasting drifts Mario east into the box.
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
    # 2) creep east in single-frame taps to dx <= 24: inside the trigger radius
    #    (it can no longer arm) but still west of the box at objX-19.
    for _ in range(300):
        f = frames[-1]
        k, s = obj_ahead(12)
        if k is None or s['x'] - f['x'] <= 24:
            break
        run(['right'], 1)
        check_dead()
        run([], 2)
    # 3) wait for the object to cycle into state 0 / back down: it then writes
    #    type $80 and returns every frame, so it stays harmless while we sit here.
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
    """Walk east; stomp walkers ahead straight-up; hop when stalled."""
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
drive_east(3240, 'past-marker15')    # anim-15 marker at 3192
drive_east(3480, 'past-walker3')     # walker 3352
drive_east(3580, 'past-bobber3')     # bobber 3456; stop west of the tower
def hop_east(tag):
    # 18 frames of run-up: the 36-frame jump only clears the 33 px spike gap
    # (tiles 231-232 at row 29) if hvel is already at run speed when it starts.
    run(['right', 'b'], 18)
    run(['a', 'right', 'b'], 36)
    run([], 2, tag)
    check_dead()

run([], 60, 'pre-tower-settle')   # hop from a standstill: the first jump's
# landing point depends on the speed at take-off
probe_tiles('pre-tower')
# Hops climb the tower; stop hopping once past it (x >= 3800). The stretch from
# 3800 east is flat floor, and hopping through it head-bumps the $7F bumpable
# block at (3872-3887, row 26): the console answers with the $5B06 block-bump
# pose ($FFC6 = 66 for 5 frames, drawn at the block's position), which the port
# does not model (bounce blocks are a listed omission). Walking under it is
# clean, and it also keeps Mario's floor probe off the exit tile at 4064.
for i in range(24):
    f = frames[-1]
    if f['x'] >= 3800:
        break
    hop_east(f'tower-{i}')
    f = frames[-1]
    print(f'    ..tower{i:02d} mx={f["x"]} my={f["y"]} ground={f["ground"]} dead={f["dead"]}')
probe_tiles('past-tower')
drive_east(4020, 'east-walk')
probe_tiles('east-end')
run([], 90, 'settle-east')
# --- transition report + dump ------------------------------------------------
print('logic-frame records:', len(REC))
prev = None
tetr = []
for i, d in enumerate(REC):
    if d['tr'] and (prev is None or prev == 0):
        tetr.append((i, 'START'))
    if not d['tr'] and prev:
        tetr.append((i - 1, 'END'))
    prev = d['tr']
print('=== tr != 0 runs ===')
for i, d in enumerate(REC):
    if d['tr']:
        pass
runs = []
start = None
for i, d in enumerate(REC):
    if d['tr'] and start is None:
        start = i
    if not d['tr'] and start is not None:
        runs.append((start, i - 1))
        start = None
if start is not None:
    runs.append((start, len(REC) - 1))
for a, b in runs:
    d0, d1 = REC[a], REC[b]
    camy0 = d0['camy'] | (d0['camyh'] << 8); camy1 = d1['camy'] | (d1['camyh'] << 8)
    camx0 = d0['camx'] | (d0['camxh'] << 8); camx1 = d1['camx'] | (d1['camxh'] << 8)
    print(f'  records {a}..{b} ({b-a+1} frames) tr={d0["tr"]:02X}: '
          f'camx {camx0}->{camx1} camy {camy0}->{camy1} '
          f'mario x {d0["x"]}->{d1["x"]} y {d0["y"]}->{d1["y"]}')
    for i in (a, a + 1, b):
        d = REC[i]
        camx = d['camx'] | (d['camxh'] << 8); camy = d['camy'] | (d['camyh'] << 8)
        print(f'    rec {i}: tr={d["tr"]:02X} camx={camx} camy={camy} x={d["x"]} y={d["y"]} '
              f'pipe={d["pipe"]:02X} lvl9b={d["lvl9b"]:02X}')
f = REC[-1]
print('final: mx=%d my=%d cam=(%d,%d) dead=%02X tr=%02X pipe=%02X' % (
    f['x'], f['y'], f['camx'] | (f['camxh'] << 8), f['camy'] | (f['camyh'] << 8),
    f['dead'], f['tr'], f['pipe']))
json.dump(REC, open(f'{OUT}/screen_transition1_raw.json', 'w'))
print('saved screen_transition1_raw.json (%d records) — now run:' % len(REC))
print('  apply_pairing.py screen_transition1_raw.json screen_transition1.json')
p.stop(save=False)
