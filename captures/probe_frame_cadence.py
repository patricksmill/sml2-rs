"""Frame-cadence probe: is the tick-sampled trajectory list 1:1 with the
console's logic frames?

Motivation (wp7e open item, the "ledge-side wall push-out"): `traj13_full`'s
tail diverges from f4652 (x +2, cam +2, ff0/uphase off) yet the only fields
that move between records 4651 and 4652 are `sptr`/`standby`. Dropping records
4652 and 4661 makes the whole tail (including the death at 4665) replay
all-zero, which points at the *capture sampling*, not the port's physics.

The trajectory captures sample once per PyBoy tick (`tick(); state(keys)`),
while the logic runs once per VBlank. If the game's frame overruns its VBlank
slot the loop drifts against the tick boundary, and a tick can then sample
mid-frame (state before the update) or after two logic frames. This probe
re-runs `trajectory_capture13.py`'s route unchanged and, per record, stores
the *cumulative* call counts of the frame's own addresses:

  $05E4 (bank 0)  in-level frame entry   — once per logic frame
  $0F71 (bank 0)  the player update      — called from $05E4 (unless a
                                           screen transition / death skips it)
  $8000 (bank 2)  the object engine      — the frame loop's LAST call
  $8C31 (bank 2)  the stream scan        — the tail of $8000
  $8C8D (bank 2)  the spawn (slot alloc) — inside the scan

The deltas between consecutive records say exactly which of them ran in each
tick, so a "duplicate" record can be read off as "the tick ran no $0F71".

Prints the tail (records 4640..4690) plus a histogram, and writes the full
per-record counter log to $OUT/probe_frame_cadence.json.
"""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'sml2decomp')))
OUT = '/home/pmill/.hermes/cache/scratch'
# The bank-2 routines live at $4000-$4Cxx on the bus, so a raw `bank=2, addr=$8000`
# hook trips PyBoy's VRAM-bank path ("bank of VRAM is only supported for CGB
# mode"). Resolve them through the ROM's symbol file instead (rgblink builds it:
# `rgblink -n sml2.sym -o /dev/null main.o`; `baserom.gb.sym` is its in-tree copy).
p = PyBoy(f'{ROOT}/baserom.gb', window='null', symbols=f'{ROOT}/baserom.gb.sym')
p.set_emulation_speed(0)

def rd(a): return p.memory[a]
def tick(n=1):
    for _ in range(n): p.tick()
def tap(btn, hold=6, gap=4):
    p.button_press(btn); tick(hold); p.button_release(btn); tick(gap)

# --- the cadence hooks (cumulative counters) -------------------------------
CNT = {'05E4': 0, '0F71': 0, '8000': 0, '8C31': 0, '8C8D': 0}
def mk(tag):
    def cb(_ctx):
        CNT[tag] += 1
    return cb
p.hook_register(None, 'UnknownRJump_0x05E4', mk('05E4'), None)  # in-level frame entry
p.hook_register(None, 'UnknownCall_0x0F71', mk('0F71'), None)   # player update
p.hook_register(None, 'UnknownCall_0x8000', mk('8000'), None)   # object engine (last in frame)
p.hook_register(None, 'UnknownCall_0x8C31', mk('8C31'), None)   # stream scan
p.hook_register(None, 'UnknownRJump_0x8C8D', mk('8C8D'), None)  # spawn (slot alloc)

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
        ff97=rd(0xFF97), timer=rd(0xA222), frame=rd(0xFFC6), edge=rd(0xFF98),
        limit=rd(0xA2D9) != 0, autoscroll=rd(0xA2C8),
        en_r=rd(0xA232) != 0, en_l=rd(0xA233) != 0,
        sptr=rd(0xAF1E) << 8 | rd(0xAF1F), spx=rd(0xAF00) << 8 | rd(0xAF01),
        spxlo=rd(0xAF00), spxhi=rd(0xAF01),
        sdir=rd(0xAF22), standby=rd(0xAF40), active=rd(0xAF41),
        easy=rd(0xA2E4), odd=rd(0xFFF0) & 1,
        ff0=rd(0xFFF0), uphase=rd(0xAF08), mf=rd(0xAF2F), fac=rd(0xAF2C),
        dead=rd(0xA224), sbt=rd(0xA27A), scc=rd(0xA279), mb=rd(0xA28D),
        pan=rd(0xA2B3), slot_bytes=slots_hex(), slots=slots,
        inv=rd(0xA217), sonobj=rd(0xA26B), lvl9b=rd(0xFF9B),
        pow=rd(0xA216), lives=rd(0xA22C), goal=rd(0xA24F), secret=rd(0xA2A0),
        pipe=rd(0xA221), tr=rd(0xA20E),
        n05E4=CNT['05E4'], n0F71=CNT['0F71'], n8000=CNT['8000'],
        n8C31=CNT['8C31'], n8C8D=CNT['8C8D'],
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
    for k in keys:
        p.button_release(k)

def abort(msg):
    print('  ABORT:', msg)
    json.dump(frames, open(f'{OUT}/traj12_fail.json', 'w'))
    raise SystemExit(1)

def check_dead():
    if frames[-1]['dead']:
        abort('Mario died')

def dump_slots(tag):
    f = frames[-1]
    print(f'  == slots @ {tag} (frames {len(frames)}) mario mx={f["x"]} my={f["y"]} ground={f["ground"]}')

def stomp(slot_minx, label):
    run(['right', 'b'], 240)
    k, s = walker_slot(slot_minx)
    while k is None:
        run(['right', 'b'], 4)
        check_dead()
        k, s = walker_slot(slot_minx)
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
    check_dead()

def walk_blocked(tag, limit=200):
    prev = -1
    for _ in range(limit):
        run(['right', 'b'], 2)
        check_dead()
        if frames[-1]['x'] == prev and frames[-1]['hvel'] == 0x80:
            break
        prev = frames[-1]['x']

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
if s and frames[-1]['x'] - s['x'] < -28:
    run(['a'], 30)
    run([], 110, 'w2-done')
    check_dead()

walk_blocked('pipeWall')
run(['a'], 4)
run(['a', 'right'], 20)
run(['a', 'left'], 7)
run(['a'], 10)
run([], 30, 'hopPipe')
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
run(['down'], 260, 'travel')

run(['right'], 200, 'room walk')
for _ in range(200):
    x = frames[-1]['x']
    if 1455 <= x <= 1462:
        break
    run(['right' if x < 1455 else 'left'], 1)
run([], 3, 'room aligned')
p.button_press('up')
p.button_press('a')
travel_started = False
for i in range(140):
    tick()
    frames.append(state(('up', 'a')))
    d = frames[-1]['pipe']
    if d == 0x20 and not travel_started:
        travel_started = True
    if travel_started and d != 0x20:
        break
p.button_release('a')
p.button_release('up')
if not travel_started:
    abort('up travel never started')
run([], 20, 'surfaced')

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
            hop_right(f'{tag}-hop')
            stall = 0
            continue
        run(['right'], 1)
        check_dead()

drive_right(1640, 'pre-anim8')
drive_right(1760, 'over-plateau')
drive_right(2010, 'pre-arena')
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
while frames[-1]['ground'] and frames[-1]['y'] <= 420 and frames[-1]['x'] < 2164:
    run(['right'], 1)
    check_dead()
run(['a', 'right', 'b'], 36, 'jump2')
run(['right', 'b'], 60, 'after-jump2')
check_dead()

drive_right(2460, 'pre-pair')
run([], 30, 'settle-pair')

def flag():
    f = frames[-1]
    return (f['secret'], f['pow'], f['lives'], f['inv'], f['goal'])

touched = False
def report(kind, tag):
    global touched
    f = frames[-1]
    sec, pw, lvs, invv, gol = flag()
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
json.dump(frames, open(f'{OUT}/probe_frame_cadence.json', 'w'))

# ---- report ---------------------------------------------------------------
def d(i, k):
    return frames[i][k] - frames[i - 1][k]

print(f"{'rec':>5} {'x':>5} {'y':>4} {'ph':>3} {'ff0':>4} {'uph':>3} {'sptr':>6} {'sb':>3} "
      f"{'spx':>5} {'d05E4':>5} {'d0F71':>5} {'d8000':>5} {'d8C31':>5} {'d8C8D':>5}")
for i in range(4640, min(4700, len(frames))):
    f = frames[i]
    print(f"{i:>5} {f['x']:>5} {f['y']:>4} {f['phase']:>3} {f['ff0']:>4} {f['uphase']:>3} "
          f"{f['sptr']:>6} {f['standby']:>3} {f['spx']:>5} {d(i,'n05E4'):>5} {d(i,'n0F71'):>5} "
          f"{d(i,'n8000'):>5} {d(i,'n8C31'):>5} {d(i,'n8C8D'):>5}")

from collections import Counter
hist = Counter((d(i, 'n05E4'), d(i, 'n0F71'), d(i, 'n8000')) for i in range(1, len(frames)))
print('\ndelta histogram (05E4, 0F71, 8000) over the whole run:', dict(hist))
print('ticks with no frame entry:', [i for i in range(1, len(frames)) if d(i, 'n05E4') == 0])
print('ticks with two frame entries:', [i for i in range(1, len(frames)) if d(i, 'n05E4') >= 2])
p.stop(save=False)
