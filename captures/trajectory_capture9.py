#!/usr/bin/env python3
"""Trajectory capture v9 — stomp both walkers, climb level 1's block staircase
(cols 52-59), cross to the raised floor (row 30), climb the x=1136 down-pipe,
align to the entry window and hold DOWN into the shaft.

Terrain (level01, rows 27-31, x):
  811  wall of block A (cols 52-54, top y=464 -> stand 436)
  880  gap (ground y=496 -> stand 468)
  912  wall of block B (cols 57-59, same height)
  960  plain ground (stand 468)
  1039 wall of the raised floor (cols 65-75, top y=480 -> stand 452)
  1136 the pipe (mouth row 28, top y=448 -> stand 420)
Records sPipeTravelDirection ($A221) + sScreenTransition ($A20E). The DOWN hold runs
through the $0DED transition, the resumed travel, and the fall into the
underground room — the full run must replay zero (no trim).
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
            DBG['next'] = len(frames) + 120
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
    json.dump(frames, open(f'{OUT}/traj8_fail.json', 'w'))
    raise SystemExit(1)

def check_dead():
    if frames[-1]['dead']:
        abort('Mario died')

def stomp(slot_minx, label):
    """Approach a walker, settle, jump on it at dx ~ -26 (verified recipe)."""
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
    """Jump, then drift right once the wall probes clear.
    The right press must wait: while y >= 440 the block's face still trips the
    wall probe and the h-integration resets hvel to $80. Air drift is
    SPEED_H[8 phases]/8 per frame: $B8 = 1 px/f exactly, no air friction, so
    holding right through the landing is safe (drift ~30-36 px)."""
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
    """Hold right+b until x stops changing (a wall)."""
    prev = -1
    for _ in range(limit):
        run(['right', 'b'], 2)
        check_dead()
        if frames[-1]['x'] == prev and frames[-1]['hvel'] == 0x80:
            break
        prev = frames[-1]['x']
    f = frames[-1]
    print(f'  [{tag}] mx={f["x"]} my={f["y"]} ground={f["ground"]}')

# --- walker 1 (record 0, x=472) on the flat ground ---
stomp(400, 'w1')

# --- climb the staircase ---
walk_blocked('wallA')          # blocked at ~811 by block A
hop('hopA')                 # land on block A top (y=436)
walk_until_x(870, 'walkA')     # walk right on A
walk_until_x(895, 'dropA')     # walk off A's edge -> fall into the gap (468)
run([], 20)
walk_blocked('gapB')           # settle against block B's wall (~899)
hop('hopB')                 # land on block B top (y=436)
walk_until_x(950, 'walkB')     # walk right on B
run(['right'], 25)             # walk off B -> fall to the ground (468)
run([], 20)
walk_blocked('wallFloor')      # blocked at ~1019 by the raised floor
hop('hopFloor', 4)             # land on the raised floor (y=452)

# --- walker 2 walks the raised floor; stomp it ---
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

# --- walk to the pipe wall and climb onto the pipe top ---
walk_blocked('pipeWall')
# The pipe top's standable range is x <= 1155 (probe points), so brake the
# drift: right 20 frames (~18 px at hvel $B8 = 1 px/f), left 7 to hvel $80.
run(['a'], 4)
run(['a', 'right'], 20)
run(['a', 'left'], 7)
run(['a'], 10)
run([], 30, 'hopPipe')
f = frames[-1]
print('after pipe hop: mx=%d my=%d ground=%d pipe=%02X' % (f['x'], f['y'], f['ground'], f['pipe']))

# --- step right in 1-frame taps into the entry window (x & $1F in $15..$1B) ---
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

# --- hold DOWN and travel ---
run(['down'], 260, 'travel')
print('frames:', len(frames))

json.dump(frames, open(f'{OUT}/traj9.json', 'w'))
tr_i = next((i for i, fr in enumerate(frames) if fr.get('tr')), None)
print('transition at frame', tr_i)
print('final: mx=%d my=%d dead=%d pipe=%02X tr=%02X' % (
    frames[-1]['x'], frames[-1]['y'], frames[-1]['dead'], frames[-1]['pipe'], frames[-1]['tr']))
print('saved traj9.json (%d frames)' % len(frames))
p.stop(save=False)
