#!/usr/bin/env python3
"""Full death cycle capture v2 — death → fade → map → re-enter → reload → respawn → play.

Hook `$05E4` (in-level logic frames) as v1, recording the same fields plus
`lives` ($A22C) and `tick` (VBlank counter). The fade / map / reload run no
`$05E4`, so the capture has a gap there; the driver polls `$FF9B` every tick
and stores the state-machine sequence as `gap` on the first post-respawn
record: {"total": <ticks between records>, "seq": [[state, ticks], ...]}.

Drive: run into the walker → death arc → fade (192) → map ($0C) → tap A →
re-enter ($0B→$02→$03) → walk right (no run) 280 frames → stop.
"""
import os, json
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'sml2decomp')))
OUT = '/home/pmill/.hermes/cache/scratch/wp7_extract'
p = PyBoy(f'{ROOT}/baserom.gb', window='null')
p.set_emulation_speed(0)

TICK = [0]
def rd(a): return p.memory[a]
def tick(n=1):
    for _ in range(n):
        p.tick(); TICK[0] += 1
def tap(btn, hold=6, gap=4):
    p.button_press(btn); tick(hold); p.button_release(btn); tick(gap)

held = {'right': False, 'left': False, 'up': False, 'down': False, 'a': False, 'b': False}

def slots_hex():
    return [''.join(f'{rd(0xAD00 + k * 0x20 + j):02X}' for j in range(0x20)) for k in range(16)]

def state():
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
        pcx=(rd(0xAF24) << 8) | rd(0xAF25),
        lives=rd(0xA22C), tick=TICK[0],
        right=held['right'], left=held['left'], up=held['up'],
        down=held['down'], a=held['a'], b=held['b'])

frames = []
def frame_hook(_ctx):
    frames.append(state())
p.hook_register(0, 0x05E4, frame_hook, None)

trans = []          # [tick, state] every $FF9B change after the hit
last_st = [None]
def poll_st():
    st = rd(0xFF9B)
    if st != last_st[0]:
        trans.append([TICK[0], st])
        last_st[0] = st
        return True
    return False

# --- nav to level 0 (title -> file select -> hub -> level) ---
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
print('level start: frames=%d mx=%d' % (len(frames), rd(0xFFC2) | (rd(0xFFC3) << 8)))

# --- run right into the walker ---
held['right'] = True; held['b'] = True
p.button_press('right'); p.button_press('b')
for i in range(400):
    tick()
    if rd(0xA224) != 0:
        print('hit: dead=%02X at frame %d tick %d' % (rd(0xA224), len(frames), TICK[0]))
        break
held['right'] = False; held['b'] = False
p.button_release('right'); p.button_release('b')
poll_st()

# --- phase 1: fade + map ---
for i in range(600):
    tick()
    if poll_st() and last_st[0] == 0x0C:
        break
print('on map: tick %d trans %s' % (TICK[0], trans))

# --- phase 2: settle, tap A until the state leaves the map ---
tick(40)
for i in range(20):
    tap('a')
    for _ in range(30):
        tick()
        poll_st()
        if last_st[0] != 0x0C:
            break
    if last_st[0] != 0x0C:
        break
print('left map: tick %d trans %s' % (TICK[0], trans))

# --- phase 3: wait for state 4 (level re-entered) ---
for i in range(600):
    tick()
    poll_st()
    if last_st[0] == 4:
        break
print('re-entered: tick %d frames %d trans %s' % (TICK[0], len(frames), trans))
print('respawn: x=%d y=%d lives=%02X' % (
    rd(0xFFC2) | (rd(0xFFC3) << 8), rd(0xFFC0) | (rd(0xFFC1) << 8), rd(0xA22C)))

# --- phase 4: play (walk right, no run) ---
second_death = False
held['right'] = True
p.button_press('right')
for i in range(280):
    tick()
    if rd(0xA224) != 0:
        second_death = True
        print('WARNING: second death at frame %d tick %d' % (len(frames), TICK[0]))
        break
held['right'] = False
p.button_release('right')
if not second_death:
    tick(20)

# --- attach the gap to the first post-gap record ---
gap_at = None
for i in range(1, len(frames)):
    if frames[i]['tick'] - frames[i - 1]['tick'] > 2:
        gap_at = i
        break
if gap_at is None:
    print('ERROR: no gap found!')
else:
    seq = []
    for j in range(1, len(trans)):
        st = trans[j - 1][1]
        dur = trans[j][0] - trans[j - 1][0]
        if st != 4:
            seq.append([st, dur])
    total = frames[gap_at]['tick'] - frames[gap_at - 1]['tick']
    frames[gap_at]['gap'] = dict(total=total, seq=seq)
    print('gap at record %d: total=%d ticks, seq=%s' % (gap_at, total, seq))
    print('pre-gap record %d: tick=%d lives=%02X dead=%02X y=%d' % (
        gap_at - 1, frames[gap_at - 1]['tick'], frames[gap_at - 1]['lives'],
        frames[gap_at - 1]['dead'], frames[gap_at - 1]['y']))

print('total frames:', len(frames))
f = frames[-1]
print('final: x=%d y=%d dead=%02X lives=%02X standby=%d active=%d ff0=%d uphase=%d' % (
    f['x'], f['y'], f['dead'], f['lives'], f['standby'], f['active'], f['ff0'], f['uphase']))
json.dump(frames, open(f'{OUT}/death2_raw.json', 'w'))
print('saved death2_raw.json (%d frames)' % len(frames))
p.stop(save=False)
