#!/usr/bin/env python3
"""Probe: drop Mario onto the level-1 pipe at x=1144 (cols 71-72, mouth $08/$09),
then hold DOWN and watch the travel — direction, Y, transition vars, level id.
Tells us where the pipe travel ends and what the transition does."""
import os
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'
from pyboy import PyBoy

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'sml2decomp')))
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
print('level start: mx=%d my=%d' % (rd(0xFFC2) | (rd(0xFFC3) << 8), rd(0xFFC0) | (rd(0xFFC1) << 8)))

# teleport above the pipe mouth and let him fall onto it
p.memory[0xFFC2] = 1144 & 0xFF; p.memory[0xFFC3] = 1144 >> 8
p.memory[0xFFC0] = 400 & 0xFF; p.memory[0xFFC1] = 400 >> 8
# also give the camera a nudge so the scroll is consistent (camx follows Mario)
tick(2)
for i in range(240):
    tick()
    mx = rd(0xFFC2) | (rd(0xFFC3) << 8)
    my = rd(0xFFC0) | (rd(0xFFC1) << 8)
    if rd(0xA214):  # on ground
        break
print('landed at f%d mx=%d my=%d ground=%d tile=%02X pipeDir=%02X align=%02X' % (
    i, mx, my, rd(0xA214), rd(0xA20D), rd(0xA221), rd(0xA283)))
# hold DOWN for 200 frames, print every 4th frame
p.button_press('down')
for i in range(200):
    tick()
    if i % 4 == 0 or i > 100:
        print('f%3d y=%5d dir=%02X hvel=%02X vvel=%02X dead=%02X lvl=%02X sc=%02X %02X %02X trans=%02X' % (
            i, rd(0xFFC0) | (rd(0xFFC1) << 8), rd(0xA221), rd(0xA200), rd(0xA201),
            rd(0xA224), rd(0xFF9B), rd(0xA2A7), rd(0xA2A8), rd(0xA2A9), rd(0xA26F)))
    if i > 100 and rd(0xFF9B) != 4:
        print('  level changed at f%d -> lvl=%02X' % (i, rd(0xFF9B)))
        for j in range(60):
            tick()
            if j % 6 == 0:
                print('   +%2d lvl=%02X mx=%d my=%d y=%d' % (j, rd(0xFF9B),
                      rd(0xFFC2) | (rd(0xFFC3) << 8), rd(0xFFC0) | (rd(0xFFC1) << 8),
                      rd(0xFFC0) | (rd(0xFFC1) << 8)))
        break
p.button_release('down')
p.stop(save=False)
