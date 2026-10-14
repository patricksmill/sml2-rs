#!/usr/bin/env python3
"""Scene capture 1 — LEVEL 3 (0-based index 2), the first capture that LATCHES a scene.

Level 3 is the deliberate minimum for a scene capture. Reached with the map-node route
in `probe_level_select.py` (poke sMapScreenId/sMapNode, tap A): node 34 -> `$4218[34] = 2`
-> `levels/level03.bin`. Its scene map (`extracted/levels/level03.scene.bin`, cell index
`(Yhi & $0F) * 16 + (Xhi & $0F)`) has exactly two nonzero cells: **i4 -> scene 7** and
i21 -> scene 6, and both scripts are pure `$4x` teleports (`40 16 FF`, `40 03 FF` from
`extracted/scenes/script_table.bin`). So a latch exercises the `$0F0B` latch, the `$20A4`
script VM and the `$4x` teleport with NO boss load and NO `$0EB0` branch (`$2x` never
fires, so `sSceneState` stays 0 and the low-nibble test never passes).

WHY THE ROUTE IS "WALK EAST IN Y PAGE 0": a scene id is latched from the cell Mario
occupies when the hard-edge screen transition fires, and the cell index packs the Y page
in the HIGH nibble. Cell i4 = Y page 0, X page 4, i.e. x 1024-1279 with y < 256 — the
same Y page the spawn is in (46,164 per `level03.manifest.json`, camera 80,176). The
right transition out of cell (0,3) is permitted (`edge[3] = $05`, bit 0 set), so walking
east while staying above y = 256 crosses into cell (0,4) and latches scene 7. The `$4x`
operand `$16` then moves Mario AND the camera to Y page 1 / X page 6 (only the page
bytes move), which the capture records raw (`$FFC0-$FFCB`) so the teleport is visible.

The `$05E4` hook records one entry per LOGIC frame (the traj14+ method) and the `$028D`
hook is spliced on as the safety net (same frame boundary, so one pairing covers both).
The driver holds RIGHT and hops when the console stops advancing (walls, ledges, walkers)
- inputs are recorded per frame, so the replay reproduces them.

Run:    /home/pmill/.venvs/sml2-pyboy/bin/python captures/scene_capture1.py
Then:   python3 captures/apply_pairing.py scene1_raw.json captures/scene1.json
Replay: cargo run -q -p sml2-runtime -- --level 2 --replay captures/scene1.json
"""
import os, json, sys
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pyboy import PyBoy
import probe_level_select as pls

NODE = 34              # $4218[34] = 2 -> levels/level03.bin
LEVEL_INDEX = 2
CAP = int(os.environ.get('SML2_SCENE1_CAP', '6000'))     # records
POST_LATCH = int(os.environ.get('SML2_SCENE1_POST', '90'))  # frames recorded after a latch

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'sml2decomp')))
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

pls.boot_to_map(p, tick, tap, rd)
first = {}
ok = pls.enter_level(p, tick, tap, rd, NODE, first_frame=first)
if not ok:
    raise SystemExit('level entry failed')
print(f"entry: curlevel={first['curlevel']:02X} mario=({first['mario_x']},{first['mario_y']}) "
      f"cam=({first['cam_x']},{first['cam_y']})   [level03 header: curlevel=02 mario=46,164 cam=80,176]")
assert first['curlevel'] == LEVEL_INDEX, f"entered level {first['curlevel']}, expected {LEVEL_INDEX}"

def slots_hex():
    return [''.join(f'{rd(0xAD00 + k * 0x20 + j):02X}' for j in range(0x20)) for k in range(16)]

def state(keys):
    sl = []
    for k in range(16):
        b = 0xAD00 + k * 0x20
        sl.append(dict(state=rd(b + 8), x=(rd(b) << 8) | rd(b + 1),
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
        pan=rd(0xA2B3), slot_bytes=slots_hex(), slots=sl,
        inv=rd(0xA217), sonobj=rd(0xA26B), lvl9b=rd(0xFF9B),
        coins=rd(0xA262), coinshi=rd(0xA263),
        pipe=rd(0xA221), tr=rd(0xA20E), col=rd(0xA20D),
        # the scene system + the per-level property bytes + Mario's screen Y
        scene=rd(0xA28E), scenehi=rd(0xA28F), sst=rd(0xA28B),
        trflag=rd(0xA24A), latch=rd(0xA249), doneflag=rd(0xA2A8),
        moon=rd(0xA287), msx=rd(0xA23C), msy=rd(0xA23B), curlevel=rd(0xA269),
        swim=rd(0xA210), lpf=rd(0xA27D), exit=rd(0xA2B4),
        fadelo=rd(0xA266), fadehi=rd(0xA267), tally=rd(0xA2C5),
        # raw pages the $4x teleport rewrites
        py_mario=rd(0xFFC9), px_mario=rd(0xFFCB),
        py_cam=rd(0xFFC1), px_cam=rd(0xFFC3),
        tick=TICKS['n'],
        right='right' in keys, left='left' in keys, up='up' in keys,
        down='down' in keys, a='a' in keys, b='b' in keys)

REC = []
POST = []
CUR = set()
def frame_hook(_ctx):
    if len(REC) < CAP:
        REC.append(state(CUR))
def post_hook(_ctx):
    POST.append(state(CUR))
p.hook_register(0, 0x05E4, frame_hook, None)
p.hook_register(0, 0x028D, post_hook, None)

BTNS = ('right', 'left', 'up', 'down', 'a', 'b')
_cur_buttons = set()
def held(keys):
    for b in BTNS:
        if b in keys and b not in _cur_buttons:
            p.button_press(b)
        elif b not in keys and b in _cur_buttons:
            p.button_release(b)
    _cur_buttons.clear(); _cur_buttons.update(keys)
    CUR.clear(); CUR.update(keys)

# --- the driver: east, jump over walkers, hop when the console stops advancing
print('driving east until a scene latch ...')

def slots_full():
    out = []
    for k in range(16):
        b = 0xAD00 + k * 0x20
        out.append(dict(state=rd(b + 8), x=(rd(b) << 8) | rd(b + 1),
                        y=(rd(b + 3) << 8) | rd(b + 4), def_=rd(b + 5)))
    return out

def walkers_ahead(mx, span=88):
    return [s for s in slots_full() if s['state'] and mx - 4 < s['x'] < mx + span]

mode = 'jump' if os.environ.get('SML2_SCENE1_MODE') == 'bounce' else 'walk'
BOUNCE = os.environ.get('SML2_SCENE1_MODE') == 'bounce'
jumpn = 0; stall = 0; lastx = -1
latch_at = None
EVENTS = []
prev = None
prev_key = None
loops = 0
while len(REC) < CAP:
    loops += 1
    r = state(CUR)
    key = (r['x'], r['y'], r['scene'], r['sst'], r['tr'], r['dead'], r['lvl9b'])
    if prev is None or key != prev_key:
        if prev is None or (r['scene'], r['sst'], r['tr'], r['dead'], r['lvl9b']) != \
           (prev['scene'], prev['sst'], prev['tr'], prev['dead'], prev['lvl9b']):
            EVENTS.append(f"{r['tick']:>6} x={r['x']:>5} y={r['y']:>5} cam=({r['camxh']:X}{r['camx']:02X},{r['camyh']:X}{r['camy']:02X}) "
                          f"edge={r['edge']:02X} msx={r['msx']:02X} scene={r['scene']:02X} sst={r['sst']:02X} "
                          f"latch={r['latch']:02X} trflag={r['trflag']:02X} tr={r['tr']:02X} dead={r['dead']:02X} lb={r['lvl9b']:02X}")
            print('  ' + EVENTS[-1])
        prev_key = key
    prev = r
    if len(REC) % 600 == 0 and len(REC):
        print(f"  ..rec {len(REC)} x={r['x']} y={r['y']} cam=({r['camxh']:X}{r['camx']:02X}) "
              f"ground={r['ground']} swim={r['swim']:02X} scene={r['scene']}")

    # stop conditions
    if r['scene'] or r['sst'] or r['trflag'] or r['latch']:
        if latch_at is None:
            latch_at = len(REC)
            print(f"  *** SCENE LATCH at record {latch_at} (tick {r['tick']}): scene={r['scene']:02X} "
                  f"sst={r['sst']:02X} latch={r['latch']:02X} trflag={r['trflag']:02X} - recording "
                  f"{POST_LATCH} more frames for the VM/teleport")
        elif len(REC) - latch_at >= POST_LATCH:
            print('  *** post-latch window complete')
            break
    if r['dead']:
        print(f"  *** DEAD at record {len(REC)} x={r['x']} y={r['y']} - stopping (no death recorded)")
        break
    if r['lvl9b'] != 4 and latch_at is None:
        print(f"  *** state left 4 -> {r['lvl9b']:02X} at record {len(REC)} - stopping")
        break

    if mode == 'walk':
        if r['x'] == lastx:
            stall += 1
        else:
            stall = 0
        lastx = r['x']
        if walkers_ahead(r['x']) and r['ground']:
            mode = 'jump'; jumpn = 0
            print(f"    [{r['tick']}] walker ahead of x={r['x']} -> jump")
        elif stall >= 20:
            mode = 'jump'; jumpn = 0
            print(f"    [{r['tick']}] stall at x={r['x']} y={r['y']} -> hop")
    else:
        jumpn += 1
        if jumpn >= 30:
            if BOUNCE:
                jumpn = 0                      # keep bouncing
            else:
                mode = 'walk'; stall = 0; lastx = -1

    held(('right', 'a') if mode == 'jump' else ('right',))
    tick()

print(f"\n=== scene_capture1 (level index {LEVEL_INDEX}, node {NODE}) ===")
print(f"  records: {len(REC)} (ticks {REC[0]['tick']}..{REC[-1]['tick']})  post-hook fires: {len(POST)}")
f0, fl = REC[0], REC[-1]
print(f"  rec0:  x={f0['x']} y={f0['y']} cam=({f0['camxh']:X}{f0['camx']:02X},{f0['camyh']:X}{f0['camy']:02X}) edge={f0['edge']:02X}")
print(f"  last:  x={fl['x']} y={fl['y']} cam=({fl['camxh']:X}{fl['camx']:02X},{fl['camyh']:X}{fl['camy']:02X}) edge={fl['edge']:02X}")
print(f"  x range {min(r['x'] for r in REC)}..{max(r['x'] for r in REC)}; y range {min(r['y'] for r in REC)}..{max(r['y'] for r in REC)}")
print(f"  scene  values: {sorted(set(r['scene'] for r in REC))}")
print(f"  sst    values: {sorted(set(r['sst'] for r in REC))}")
print(f"  latch  values: {sorted(set(r['latch'] for r in REC))}")
print(f"  trflag values: {sorted(set(r['trflag'] for r in REC))}")
print(f"  tr     values: {sorted(set(r['tr'] for r in REC))}")
print(f"  lvl9b  values: {sorted(set(r['lvl9b'] for r in REC))}")
print(f"  page bytes (rec0 -> last): marioY {f0['py_mario']:02X}->{fl['py_mario']:02X} marioX {f0['px_mario']:02X}->{fl['px_mario']:02X} "
      f"camY {f0['py_cam']:02X}->{fl['py_cam']:02X} camX {f0['px_cam']:02X}->{fl['px_cam']:02X}")
print(f"  object classes seen: {sorted(set(s['def_'] for r in REC for s in r['slots'] if s['state']))}")
if latch_at:
    print(f"  latch at record {latch_at}; the frames after it carry the VM + teleport")
for e in EVENTS[-12:]:
    print(f"    event {e}")
json.dump(REC, open(f'{OUT}/scene1_raw.json', 'w'))
print(f"saved {OUT}/scene1_raw.json ({len(REC)} records)")
print('now run: python3 captures/apply_pairing.py scene1_raw.json captures/scene1.json')
p.stop(save=False)
