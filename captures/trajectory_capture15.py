"""Trajectory capture v15 — the brick-bump ("bounce block") system.

Level 1's early section is the cheapest place to exercise `$1993`/`$5960`/
`$5997`: Mario walks on the row-31 floor (y=468) with the row-28 cells as
blocks above him, and the first two are `$01` cells at x=352 and x=384 (the
ceiling probe is `(x+16, y+28)`, so the probe column wants x in [336,351] and
[368,383]). A `$01` bonk is the richest case in the level:

  - `$19D2` sets `sSpinJumpBounce` ($A220) = `$FF`, allocates a **type 3** slot
    (the `$5B06` bump: `$FFC6 = 66` at timers 3-7, freed at 8),
  - then `$2C77` → `$591F` credits **+1 coin** (BCD, the `$0999` clamp) and
    allocates a **type 4** slot — the coin popping out, drawn from the `$5AF9`
    cycle `[$6D,$6E,$6F,$6E]` indexed by `(timer & $0C) >> 2` until timer `$32`.
    Since that slot is second, its `$FFC6` is what the `frame` check sees.

The route: walk east, settle against x=352, jump twice (the block stays solid —
the "used block" tile write goes to the BG map, not the collision map — so the
second jump bumps it again), then walk east past the first walker (stomped with
`trajectory_capture14.py`'s recipe) to the `$00` block at x=576-607, whose bonk
is the single-slot **type 2** case that draws `$FFC6 = 66` with nothing to
override it.

Records come from the `$05E4` hook (one per LOGIC frame, the death_capture /
`trajectory_capture14.py` method) — run `apply_pairing.py traj15_raw.json
traj15.json` before replaying. New per-frame fields, all `Option`/defaulted so
older captures still replay: `spin` ($A220), `coins`/`coinshi` ($A262/$A263),
`bslots` (the 8 `sBlockAnimSlots` at $AA80 as 16-byte dumps — the whole bump
state, including the stale bytes a freed slot keeps).
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

def bslots_hex():
    return [''.join(f'{rd(0xAA80 + k * 0x10 + j):02X}' for j in range(0x10)) for k in range(8)]

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
        pcx=(rd(0xAF24) << 8) | rd(0xAF25),
        # --- v15: the brick-bump observables ---
        spin=rd(0xA220), coins=rd(0xA262), coinshi=rd(0xA263),
        bslots=bslots_hex(),
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

frames = []
CUR = set()

def hook(_ctx):
    frames.append(state(CUR))

p.hook_register(0, 0x05E4, hook, None)

def run(keys, n, tag=''):
    for k in keys:
        p.button_press(k)
    CUR.clear(); CUR.update(keys)
    for _ in range(n):
        tick()
    for k in keys:
        p.button_release(k)
    CUR.clear()
    if tag:
        f = frames[-1]
        print(f'  [{tag}] n={len(frames)} mx={f["x"]} my={f["y"]} ground={f["ground"]} '
              f'frame={f["frame"]} coin={f["coins"]:02X} spin={f["spin"]:02X} '
              f'dead={f["dead"]} bslot0={f["bslots"][0]}')

def abort(msg):
    print('  ABORT:', msg)
    json.dump(frames, open(f'{OUT}/traj15_fail.json', 'w'))
    raise SystemExit(1)

def check_dead():
    if frames[-1]['dead']:
        abort('Mario died')

def col_at(x):
    return (x + 16 - 8) >> 4

print('== bump phase 1: the $01 block at col 22 (x 352-367)')
run(['right'], 60, 'walk-1')
while frames[-1]['x'] < 336:
    run(['right'], 1)
run([], 20, 'settle-1')
for _ in range(20):
    if frames[-1]['x'] > 347:
        run(['left'], 1)
        run([], 2)
    else:
        break
f = frames[-1]
print(f'  at block: mx={f["x"]} probe-col={col_at(f["x"])} '
      f'cell28={rd(0xB000 + 28 * 256 + col_at(f["x"])):02X} my={f["y"]} '
      f'coin={f["coins"]:02X}')
run(['a'], 30, 'jump-1')
run([], 45, 'after-jump-1')
run(['a'], 30, 'jump-2')
run([], 60, 'after-jump-2')

print('== third jump at the $01 block: its cell reads $07 by now (the bump')
print('   wrote 24 at timer 2 and 7 at timer 8), so this must be a plain bonk')
run(['a'], 30, 'jump-3')
run([], 60, 'after-jump-3')

# The tail is deliberately tolerant: whatever happens (the walker at x=472 is
# the immediate obstacle) is a faithful console run, so the capture stays
# replayable. `check_dead` is only used where a death would mean the route
# never reached the interesting geometry.
def dead() -> bool:
    return frames[-1]['dead'] != 0

print('== walk east toward the $00 block at col 36-37 (x 576-607)')
# capture14's stomp recipe, in small steps, so the walker does not land a side
# contact while Mario is still running.
for _ in range(600):
    if dead():
        break
    k, s = walker_slot(0)
    if k is not None and frames[-1]['x'] - s['x'] >= -56:
        break
    run(['right', 'b'], 1)
f = frames[-1]
k, s = walker_slot(0)
print(f'  approach: mx={f["x"]} walker={s["x"] if s else None} dead={f["dead"]}')
if not dead():
    run([], 1)
    for _ in range(40):
        if frames[-1]['hvel'] == 0x80:
            break
        run([], 1)
    for _ in range(300):
        if dead():
            break
        k, s = walker_slot(0)
        if k is None or frames[-1]['x'] - s['x'] >= -28:
            break
        run([], 1)
    f = frames[-1]
    print(f'  stomp setup: mx={f["x"]} walker={s["x"] if s else None}')
    run(['a'], 30, 'stomp')
    run([], 100, 'stomp-done')

if not dead():
    print('== walk to the $00 block at col 36-37 (x 576-607)')
    for _ in range(400):
        if dead() or frames[-1]['x'] >= 561:
            break
        run(['right'], 1)
    for _ in range(30):
        if frames[-1]['x'] > 588:
            run(['left'], 1)
            run([], 2)
        else:
            break
    run([], 20, 'settle-2')
    f = frames[-1]
    print(f'  at $00 block: mx={f["x"]} probe-col={col_at(f["x"])} '
          f'cell28={rd(0xB000 + 28 * 256 + col_at(f["x"])):02X} my={f["y"]} '
          f'coin={f["coins"]:02X}')
    run(['a'], 30, 'jump-4')
    run([], 60, 'after-jump-4')

print('== settle')
run([], 90, 'settle-end')
print('logic-frame records:', len(frames))
f = frames[-1]
print('final: mx=%d my=%d dead=%02X frame=%d coins=%02X spin=%02X' % (
    f['x'], f['y'], f['dead'], f['frame'], f['coins'], f['spin']))
json.dump(frames, open(f'{OUT}/traj15_raw.json', 'w'))
print('saved traj15_raw.json (%d records) — now run '
      'apply_pairing.py traj15_raw.json traj15.json' % len(frames))
p.stop(save=False)
