"""Crouch-look camera pan capture ($307D: B+Up / B+Down, 1 px/frame).

Route: level-0 boot, 20 frames of B+Down and 20 of B+Up at the spawn (both
guarded off: camY_lo $C0 blocks the down arm, sMarioScreenY 118 >= $70 blocks
the up arm), walk right to the $01 brick at (28,22) (x 360-375, top y 464),
jump onto it (Mario y 420), then hold B+Up (camY 448 -> 406, 42 pan frames,
stopped by sMarioScreenY >= $70) and B+Down (camY 406 -> 448, 42 pan frames,
stopped by the bit3 + camY_lo >= $C0 guard).

Sampled per LOGIC frame in a $05E4 hook (trajectory_capture14.py's method);
run `apply_pairing.py crouch_pan1_raw.json crouch_pan1.json` afterwards.

NOTE: `pan` ($A2B3 sCameraPanTimer) is NOT written by $307D — that timer is
the boss-transition camera bob (main.asm $A9xx-$ADxx); the crouch pan is
observable only in camy ($FFC8/$FFC9). The capture records `pan` as measured
(0 throughout) and the script prints the camy-change frames as the evidence.
"""
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

# --- the level-0 boot path (trajectory_capture6.py's prefix) ----------------
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
        pcx=(rd(0xAF24) << 8) | rd(0xAF25),
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

REC = []
CUR = set()
def frame_hook(_ctx):
    REC.append(state(CUR))

p.hook_register(0, 0x05E4, frame_hook, None)

frames = []          # per-tick samples, route decisions only
def run(keys, n, tag=''):
    for k in keys:
        p.button_press(k)
    CUR.clear(); CUR.update(keys)
    for _ in range(n):
        tick()
        frames.append(state(keys))
    for k in keys:
        p.button_release(k)
    CUR.clear()
    if tag or n > 1:
        f = frames[-1]
        print(f'  [{tag}] tick={len(frames)} recs={len(REC)} x={f["x"]} y={f["y"]} '
              f'cam=({f["camx"]},{f["camy"]}) edge={f["edge"]:02X} pan={f["pan"]:02X} '
              f'ground={f["ground"]}')

def abort(msg):
    print('  ABORT:', msg)
    json.dump(REC, open(f'{OUT}/crouch_pan1_fail.json', 'w'))
    p.stop(save=False)
    raise SystemExit(1)

run([], 10, 'idle')
print('--- B+Down at the spawn: guarded off (bit3 + camY_lo >= $C0) ---')
run(['b', 'down'], 20, 'b+down@spawn')
print('--- B+Up at the spawn: guarded off (sMarioScreenY 118 >= $70) ---')
run(['b', 'up'], 20, 'b+up@spawn')
print('--- walk right to the (28,22) brick ---')
for _ in range(400):
    if frames[-1]['x'] >= 330:
        break
    run(['right'], 1)
run([], 5, 'at-brick')
if frames[-1]['dead']:
    abort('died on the walk')
print('--- jump onto the brick (top y 464 -> Mario y 420) ---')
# The probe's proven sequence (walk to x=330, 5 idle frames, then this jump)
# lands on the brick at x=341; the 'right' phase walks to x=351.
for attempt in range(5):
    run(['a', 'right'], 14)
    run(['a'], 8)
    run(['right'], 10)
    run([], 20)
    f = frames[-1]
    print(f'  attempt {attempt}: x={f["x"]} y={f["y"]} camy={f["camy"]} ground={f["ground"]}')
    if f['y'] == 420:
        break
    if f['x'] > 400:
        abort('jumped past the brick')
if frames[-1]['y'] != 420:
    abort('never landed on the brick')
if frames[-1]['dead']:
    abort('died at the brick')
run([], 10, 'on-brick')
print('--- B+Up on the brick (expect camY 448 -> 406) ---')
run(['b', 'up'], 60, 'b+up@brick')
print('--- B+Down on the brick (expect camY 406 -> 448) ---')
run(['b', 'down'], 60, 'b+down@brick')

# --- verification -----------------------------------------------------------
print('\n=== camy change events (rec: from -> to, keys, x, y) ===')
last = None
up = down = 0
for i, d in enumerate(REC):
    if last is not None and d['camy'] != last:
        delta = d['camy'] - last
        if delta == -1: up += 1
        elif delta == 1: down += 1
        print(f'  rec {i}: camy {last} -> {d["camy"]} ({delta:+d}) keys='
              f'{"bdown" if d["b"] and d["down"] else "bup" if d["b"] and d["up"] else "?"} '
              f'x={d["x"]} y={d["y"]} edge={d["edge"]:02X}')
    last = d['camy']
print(f'pan-up frames (camy -1): {up}   pan-down frames (camy +1): {down}')
nz = [i for i, d in enumerate(REC) if d['pan']]
print(f'frames with pan != 0: {len(nz)} {nz[:10]}')
print('final: x=%d y=%d camy=%d dead=%02X lvl9b=%02X' % (
    REC[-1]['x'], REC[-1]['y'], REC[-1]['camy'], REC[-1]['dead'], REC[-1]['lvl9b']))
print('logic-frame records:', len(REC), ' ticks:', len(frames))
json.dump(REC, open(f'{OUT}/crouch_pan1_raw.json', 'w'))
print('saved crouch_pan1_raw.json — now run:')
print('  apply_pairing.py crouch_pan1_raw.json crouch_pan1.json')
p.stop(save=False)
