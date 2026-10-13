"""Trajectory capture v13 - ITEM PICKUP (`$3451` + the `$2DC6` item branch).

Route: trajectory_capture12.py's prefix up to the hopper/item pair (x~2460),
then dedicated jump attempts at the anim-23 pose87 object at (2792, 416), whose
handler writes type `$87`. `$2DC6` takes the bit-7 branch and stages
`sPendingItemId = $87 & $7F = 7`; the next frame's `$3451` sees id 7 and sets
`$A2A0 = $FF` (the secret route) - an observable that survives to the capture's
end, so the run only needs ONE of the attempts to connect. The script prints the
flag after each attempt and stops at the first touch.

Level 1's other bit-7 objects: the anim-15 marker at (3192, 496), type `$85`
(id 5, a 1-UP) - it sits in a 2-tile cavity under the 448 ledge (rows 28 and 31
solid, no entrance), so it is not reachable; the anim-22 pose86 at (4008, 360),
type `$86` (id 6, the goal bell) - 108 px above the floor, out of jump range.
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
        inv=rd(0xA217), sonobj=rd(0xA26B), lvl9b=rd(0xFF9B),
        pow=rd(0xA216), lives=rd(0xA22C), goal=rd(0xA24F), secret=rd(0xA2A0),
        pipe=rd(0xA221), tr=rd(0xA20E),
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
            print(f'    ..f{len(frames)} mx={f["x"]} my={f["y"]} dead={f["dead"]} pipe={f["pipe"]:02X}')
    for k in keys:
        p.button_release(k)
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

drive_right(2460, 'pre-pair')
dump_slots('at-pair')
run([], 30, 'settle-pair')

# =====================================================================
# NEW (v13): item pickup. The anim-23 pose87 at (2792, 416) writes type $87, so
# $2DC6 stages item id 7 and the next frame's $3451 sets sSecretRouteFlag
# ($A2A0) = $FF. The object sits ~52 px above the floor and its box geometry is
# not obvious from the slot y alone, so the capture covers two crossing
# families: (a) running hops eastward with different run-ups (each shifts the
# apex relative to x=2792), and (b) jumps west off the 448 plateau at 2880-3039.
# The flag is never cleared here, so one connecting attempt is enough; it is
# printed after every attempt.
# =====================================================================
def flag():
    f = frames[-1]
    return (f['secret'], f['pow'], f['lives'], f['inv'], f['goal'])

dump_slots('item-zone')
print('  item: before, secret/pow/lives/inv/goal =', flag())
touched = False

def report(kind, tag):
    global touched
    f = frames[-1]
    sec, pw, lvs, invv, gol = flag()          # NB: do not shadow `p` (PyBoy)
    print(f'  item {kind} {tag}: mx={f["x"]} my={f["y"]} secret=${sec:02X} pow={pw} '
          f'lives={lvs:02X} inv={invv} goal={gol} dead={f["dead"]}')
    if sec == 0xFF:
        touched = True
    return touched

drive_right(2700, 'pre-item')
for r, R in enumerate((6, 10, 14, 18, 22, 26)):
    run(['right', 'b'], R)
    run(['a', 'right', 'b'], 36)
    run([], 2)
    check_dead()
    if report('hop', f'r{r}-runup{R}'):
        break
if not touched:
    drive_right(2900, 'to-plateau')
    for r, R in enumerate((0, 4, 8, 12)):
        run(['left'], 4 + R)
        run(['a', 'left', 'b'], 36)
        run([], 2)
        check_dead()
        if report('leap', f'r{r}-back{R}'):
            break
run([], 90, 'settle-item')

print('item pickup', 'HIT' if touched else 'MISSED')
print('frames:', len(frames))
json.dump(frames, open(f'{OUT}/traj13.json', 'w'))
f = frames[-1]
print('final: mx=%d my=%d secret=$%02X pow=%d lives=$%02X' % (f['x'], f['y'], f['secret'], f['pow'], f['lives']))
print('saved traj13.json (%d frames)' % len(frames))
p.stop(save=False)
