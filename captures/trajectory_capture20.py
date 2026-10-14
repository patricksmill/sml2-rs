#!/usr/bin/env python3
"""Trajectory capture v19 — LEVEL 20 (0-based index 19), the Space Zone "Star Course".

The FIRST capture on a level other than level 1. Reachable only via the map-node
route in `probe_level_select.py` (poke sMapScreenId/sMapNode, tap A): node 61 ->
`sCurLevel = $4218[61] = 19` -> `levels/level20.bin` (Star Course: autoscroll +
`sMoonPhysics` = 1 + scene map cell 15 -> scene id 13).

WHAT THIS CAPTURE CARRIES (all console-measured):
  * the `$0DAF` autoscroll path: `sAutoScroll = 1`, camera +1 px every TWO
    frames (parity off `$FF97 & 1`), left enable cleared;
  * `sMoonPhysics` ($A287) = 1, recorded raw every frame;
  * the `$0FBF` screen clamp: idle Mario is dragged at the left edge
    (`sMarioScreenX`, $A23C = the recorded `msx`, rides at `$10` and stays
    under `$21` for the whole span — measured, min `$0F`, max `$21`).
  * the scene bytes (`sSceneId`/`sSceneState`/`sSceneTransitionFlag`/Latch/done)
    every frame — all zero: on this level the console's `$0F0B` latch never
    fires inside this capture's span, because the right-edge trigger needs
    `sMarioScreenX >= $A1` while `$0FBF` clamps it to `$A0`, and the
    pipe-travel `$30` override that bypasses the clamp sits at the end pipe
    (x = 4064), unreached here (the recording ends at the first chaser's
    contact, x = 700).

THE ROUTE (deliberately inputless): ride from the spawn. The autoscroll drags
Mario along at the screen's left edge ($0FBF clamp); the first floor chaser (an
anim-95 "angry star", spawn record x=728, type $48) walks into him at x = 700,
so the recording ends at the death trigger frame (record 1309, the last alive
frame — the span carries no death). The unreached half — surviving the star
maze to the end pipe (x 4064, the `$0C` mouth whose `$30` travel bypasses the
clamp) and latching scene 13 by `$0F0B` — is the next target
(`scene_latch_capture`); the replay divergences this capture exposes are
documented in captures/README.md.

Records come from the `$05E4` hook (one per LOGIC frame; the traj14+ method) +
a second hook at `$028D` as the splice safety net (the two sample the same
frame boundary). New recorded fields (all ignored by older harnesses and read
by none of the current checks): scene/scenehi/sst/trflag/latch/doneflag/
autoscroll/moon/sclimit/msx — the scene-system + per-level property bytes.

Run:   /home/pmill/.venvs/sml2-pyboy/bin/python captures/trajectory_capture20.py
Then:  python3 captures/apply_pairing.py traj19_raw.json traj19.json
Replay: cargo run -q -p sml2-runtime -- --level 19 --replay captures/traj19.json
"""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pyboy import PyBoy
import probe_level_select as pls

# Hard safety cap on records. The route is inputless and deterministic, and the
# recording ends by itself at the console's death trigger (record ~1309) — this
# cap only guards against a runaway if the entry path changes.
CAP = int(os.environ.get('SML2_TRAJ19_CAP', '2500'))

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

# --- the level-entry route (probe_level_select.py, measured on two targets) ---
pls.boot_to_map(p, tick, tap, rd)
first = {}
ok = pls.enter_level(p, tick, tap, rd, 61, first_frame=first)
if not ok:
    raise SystemExit('level entry failed')
print(f"entry: curlevel={first['curlevel']:02X} mario=({first['mario_x']},{first['mario_y']}) "
      f"cam=({first['cam_x']},{first['cam_y']})  [level20 header: 63,212 / 96,192]")

def slots_hex():
    return [''.join(f'{rd(0xAD00 + k * 0x20 + j):02X}' for j in range(0x20)) for k in range(16)]

def slot(i):
    h = slots_hex()[i]
    return dict(state=int(h[16:18], 16), x=int(h[0:4], 16), y=int(h[6:10], 16),
                def_=int(h[10:12], 16), sprite=int(h[22:24], 16),
                rec=int(h[12:16], 16), ty=int(h[36:38], 16))

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
        # --- v19: the scene system + per-level property bytes + the clamp byte --
        scene=rd(0xA28E), scenehi=rd(0xA28F), sst=rd(0xA28B),
        trflag=rd(0xA24A), latch=rd(0xA249), doneflag=rd(0xA2A8),
        moon=rd(0xA287), msx=rd(0xA23C), curlevel=rd(0xA269),
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

def held(keys):
    CUR.clear(); CUR.update(keys)

# --- the route: no input at all; the autoscroll drag does the moving ---------
print('riding inputless until the trim point ...')
DEATH = None
while len(REC) < CAP:
    tick()
    if rd(0xA224) and DEATH is None:
        DEATH = (len(REC), rd(0xFFC2) | (rd(0xFFC3) << 8), rd(0xFFC0) | (rd(0xFFC1) << 8))
        print(f'  (note: the console death trigger fired at record {DEATH[0]} '
              f'({DEATH[1]},{DEATH[2]}) - the capture span ends there)')
        break
    if len(REC) % 400 == 0 and len(REC):
        print(f'  ..rec {len(REC)} x={rd(0xFFC2) | (rd(0xFFC3) << 8)} '
              f'cam={rd(0xFFCA) | ((rd(0xFFCB) & 0xF) << 8)} dead={rd(0xA224)}')

assert len(REC) >= 2, 'not enough records'
# The idle ride must never have gone through a state change before the cut.
assert all(r['lvl9b'] == 4 for r in REC), 'state left 4 inside the capture span'
assert all(r['dead'] == 0 for r in REC), 'a death was recorded inside the capture span'
assert len(POST) >= len(REC), f'$028D fires {len(POST)} < $05E4 records {len(REC)}'

# --- the console report -----------------------------------------------------
f0, fl = REC[0], REC[-1]
deltas = {}
prev = None
for r in REC:
    cam = r['camxh'] << 8 | r['camx']
    if prev is not None:
        d = cam - prev
        deltas[d] = deltas.get(d, 0) + 1
    prev = cam
print(f'\n=== capture ===\n  records: {len(REC)} (ticks {REC[0]["tick"]}..{REC[-1]["tick"]})')
print(f'  rec0:  x={f0["x"]} y={f0["y"]} cam=({f0["camxh"]:X}{f0["camx"]:02X}) sx={f0["msx"]:02X} '
      f'autoscroll={f0["autoscroll"]} moon={f0["moon"]} phase0={f0["phase"]}')
print(f'  last:  x={fl["x"]} y={fl["y"]} cam=({fl["camxh"]:X}{fl["camx"]:02X}) sx={fl["msx"]:02X} '
      f'dead={fl["dead"]} lvl9b={fl["lvl9b"]:02X}')
print(f'  camera deltas per logic frame: {dict(sorted(deltas.items()))}')
print(f'  camera total: {fl["camxh"] << 8 | fl["camx"]} - {f0["camxh"] << 8 | f0["camx"]} over {len(REC)-1} frames')
print(f'  msx range: {min(r["msx"] for r in REC):02X}..{max(r["msx"] for r in REC):02X} (the $0FBF clamp)')
print(f'  scene/latch/flag across the capture: scene={sorted(set(r["scene"] for r in REC))} '
      f'sst={sorted(set(r["sst"] for r in REC))} trflag={sorted(set(r["trflag"] for r in REC))} '
      f'latch={sorted(set(r["latch"] for r in REC))} tr={sorted(set(r["tr"] for r in REC))}')
print(f'  active object classes seen: {sorted(set(sl["def_"] for r in REC for sl in r["slots"] if sl["state"]))}')
if DEATH:
    print(f'  (console death trigger seen at rec {DEATH[0]} - the capture span ends there)')

json.dump(REC, open(f'{OUT}/traj19_raw.json', 'w'))
print(f'saved {OUT}/traj19_raw.json ({len(REC)} records)')
print('now run: python3 captures/apply_pairing.py traj19_raw.json traj19.json')
p.stop(save=False)
