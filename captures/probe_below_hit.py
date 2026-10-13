#!/usr/bin/env python3
"""Below-hit probe: what does the console do to a type-$86 object flagged `$20`?

`$2F72` (the block-bump alloc's tail, home.asm:6342) is the only writer of the
`+23` bit5 "hit from below" flag that `$85AA` (main.asm:2975) dispatches to
`$86A0`. A real bonk needs an object on the bonked tile — no route in the
capture suite is close (traj15's walker is 100+ px from the `$01` block) — so
this probe pokes the flag *directly*, which is exactly the state the pass
leaves behind, and lets the same frame's object engine ($8000) run the
dispatch. No route, no jump timing.

Boot level 0 (capture6's prefix), then poke a free slot ($AD00) into an
*active* type-$86 pose: anim 22 (`$587BD`), a deliberately bogus sprite `$7F`
(so the `$863C` re-derive is visible), a real stream record pointer (`$AB06`,
so `$8240` does not take its `hi >= $AD` early return), state 2, type `$86`,
and `+23` = `$20`.

MEASURED (2026-10-13, per logic frame via the `$05E4` hook; slot bytes):
  rec 0 (entry):  spr=$7F ty=$86 fl=$20 fst=0 hit=$00 flip=$00   (100,468)
  rec 1 (after $8000): spr=$1F ty=$86 fl=$00 fst=0 hit=$02 flip=$00
  rec 2:          fst=1 flip=$20            (the $58101 fstate-0 arm)
  rec 3:          (101,465) phase=1         ($411A: X+1, Y−3)
  rec 4:          unchanged                 (the slot-parity gate)
  rec 5:          (102,462) phase=2
So the console's `$86A0` → `$86E4` → `$8721` is: the sprite re-derives from
the anim (`$7F` → `$1F` = the anim-22 map entry), the *type survives* the
`$8240` clear that consumes the flag and wipes `+14..+30`, `+27` becomes 2,
and the next frame's `$58101` faces Mario (`+15` 1, `+13` `$20`) before the
knockback runs on the parity cadence. `objects.rs`'s
`stepped_below_hit_on_the_bell_re_derives_and_faces_mario` replays this same
sequence through the port's entry points and asserts byte-for-byte the same
values. It writes `$OUT/probe_below_hit.json`, never a capture for the suite.
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

def slot_hex(k=0):
    b = 0xAD00 + k * 0x20
    return ' '.join(f'{rd(b + j):02X}' for j in range(0x20))

def fields(k=0):
    b = 0xAD00 + k * 0x20
    return dict(state=rd(b + 8), x=(rd(b) << 8) | rd(b + 1),
                y=(rd(b + 3) << 8) | rd(b + 4), anim=rd(b + 5),
                rec=(rd(b + 6) << 8) | rd(b + 7), sprite=rd(b + 0x0B),
                fstate=rd(b + 0x0F), phase=rd(b + 0x10), type=rd(b + 0x12),
                flags=rd(b + 0x17), t24=rd(b + 0x18), animframe=rd(b + 0x1A),
                hitreact=rd(b + 0x1B), t28=rd(b + 0x1C),
                flip=rd(b + 0x0D), flash=rd(b + 0x0E))

LOG = []
def hook(_ctx):
    LOG.append((fields(0), dict(mario=(rd(0xFFC2) | (rd(0xFFC3) << 8),
                                      rd(0xFFC0) | (rd(0xFFC1) << 8)),
                               cam=(rd(0xFFCA) | ((rd(0xFFCB) & 0xF) << 8),
                                    rd(0xFFC8) | ((rd(0xFFC9) & 0xF) << 8)))))

p.hook_register(0, 0x05E4, hook, None)

tick(5)
print('--- poke slot 0: an active type-$86 pose, sprite $7F, flags $20 ---')
b = 0xAD00
p.memory[b + 0x00] = 0x00; p.memory[b + 0x01] = 0x64   # x = 100
p.memory[b + 0x03] = 0x01; p.memory[b + 0x04] = 0xD4   # y = 468
p.memory[b + 0x05] = 22                                # anim 22 (the pose86)
p.memory[b + 0x06] = 0xAB; p.memory[b + 0x07] = 0x06   # record pointer
p.memory[b + 0x08] = 2                                 # state: active
p.memory[b + 0x0B] = 0x7F                              # sprite: bogus on purpose
p.memory[b + 0x12] = 0x86                              # type: the bell pose
p.memory[b + 0x17] = 0x20                              # $2F72's below-hit flag
before = len(LOG)
for _ in range(6):
    tick()
for i in range(before, len(LOG)):
    f, m = LOG[i]
    print(f'  rec {i - before}: st={f["state"]} xy=({f["x"]},{f["y"]}) anim={f["anim"]} '
          f'spr=${f["sprite"]:02X} ty=${f["type"]:02X} fl=${f["flags"]:02X} '
          f'fst={f["fstate"]} ph={f["phase"]} hit=${f["hitreact"]:02X} '
          f'flip=${f["flip"]:02X} flash=${f["flash"]:02X} t24={f["t24"]} '
          f'mario={m["mario"]} cam={m["cam"]}')
json.dump([dict(slot=f, env=m) for f, m in LOG], open(f'{OUT}/probe_below_hit.json', 'w'))
print('slot bytes after:', slot_hex())
p.stop(save=False)
