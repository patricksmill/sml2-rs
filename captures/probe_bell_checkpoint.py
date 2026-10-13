#!/usr/bin/env python3
"""Midpoint-bell probe: does `$3451` id 6 (the goal bell) save respawn coords?

The port plan claimed: "when sGoalBell is set, Mario's current stage
coordinates are saved as the checkpoint respawn coordinates (sLevelStartXLo/Hi,
sLevelStartYLo/Hi)". The disassembly says otherwise — `$34EC` only writes
sSfxRequest0/sGoalBell/sLowTimeFlag/sVVelocityIndex, and the only writers of
`$A800-$A803` are `LoadLevelHeader` ($5589/$559F, 20 bytes from the level
header) and the secret-route overlay ($55C0, 8 bytes from
`UnknownData_0x377B`).

This probe measures the console directly: boot level 0, then poke
`sPendingItemId` ($A26C) = 6 — exactly what `$2DC6`'s bit-7 branch stages when
a type-$86 object is touched — and watch $A800-$A803 vs sGoalBell ($A24F)
across the frames. `$3451` runs at the tail of `$0F71`, so the poke lands in
the next logic frame.

MEASURED (2026-10-13, 119 logic frames): the poke is consumed on the next
frame (`sGoalBell` 00 -> 0C, `sPendingItemId` 06 -> 00, and vvel's `$34EC`
lift to 24), while `$A800-$A807` hold their header value — `D4 01 22 00 C0 01
50 00` = respawn (34, 468), camera (80, 448) — the whole run, including 85 px
of walking before the second poke. The bell strike saves no coordinates:
`$0F2A` only *reads* the block when the death fade reloads the level, so the
respawn point is the header's spawn (or the secret route's) and the bell never
moves it. See DESIGN.md's below-hit bullet.

Writes `$OUT/probe_bell_checkpoint.json` (a per-logic-frame record list); it is
a diagnostic, never a capture for the suite.
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

# --- per logic-frame log ----------------------------------------------------
LOG = []
CUR = set()
def log():
    return dict(
        n=len(LOG),
        goal=rd(0xA24F), pend=rd(0xA26C), vvel=rd(0xA201),
        x=rd(0xFFC2) | (rd(0xFFC3) << 8), y=rd(0xFFC0) | (rd(0xFFC1) << 8),
        start=[rd(0xA800 + i) for i in range(8)],
        secret=rd(0xA2A0), lvl9b=rd(0xFF9B), dead=rd(0xA224))

def hook(_ctx):
    LOG.append(log())

p.hook_register(0, 0x05E4, hook, None)

def run(n, tag=''):
    for _ in range(n):
        tick()
    f = LOG[-1]
    print(f'  [{tag}] n={f["n"]} goal={f["goal"]:02X} pend={f["pend"]:02X} '
          f'vvel=${f["vvel"]:02X} xy=({f["x"]},{f["y"]}) '
          f'$A800-07={" ".join(f"{b:02X}" for b in f["start"])}')

run(20, 'idle')
print('--- poke sPendingItemId ($A26C) = 6 (the bell\'s staged id) ---')
p.memory[0xA26C] = 6
run(4, 'poke')
print('--- walk right a bit, then poke 6 again (Mario elsewhere) ---')
CUR.add('right'); p.button_press('right')
run(60, 'walk')
CUR.clear(); p.button_release('right')
run(30, 'settle')
p.memory[0xA26C] = 6
run(4, 'poke2')

print('\n=== every record where a logged field changed ===')
keys = ['goal', 'pend', 'vvel', 'x', 'y', 'start', 'secret']
last = {k: None for k in keys}
for d in LOG:
    for k in keys:
        if last[k] is not None and d[k] != last[k]:
            print(f'  rec {d["n"]}: {k}: {last[k]} -> {d[k]}  (goal={d["goal"]:02X} '
                  f'pend={d["pend"]:02X} vvel=${d["vvel"]:02X} xy=({d["x"]},{d["y"]}))')
        last[k] = d[k]

f = LOG[-1]
print(f'\nfinal: goal={f["goal"]:02X} vvel=${f["vvel"]:02X} xy=({f["x"]},{f["y"]}) '
      f'$A800-07={" ".join(f"{b:02X}" for b in f["start"])}')
print('level-start block unchanged:', all(d['start'] == LOG[0]['start'] for d in LOG))
json.dump(LOG, open(f'{OUT}/probe_bell_checkpoint.json', 'w'))
p.stop(save=False)
