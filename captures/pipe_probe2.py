#!/usr/bin/env python3
"""Focused probe: pipe travel + camera vertical vars.
Prints every frame: y, camy, screenY($A23B), transition($A20E), scrollFlags($A223),
edge($FF98), sceneState($A28B), dir($A221), camyHi."""
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

p.memory[0xFFC2] = 1144 & 0xFF; p.memory[0xFFC3] = 1144 >> 8
p.memory[0xFFC0] = 400 & 0xFF; p.memory[0xFFC1] = 400 >> 8
tick(2)
for i in range(240):
    tick()
    if rd(0xA214):
        break
print('landed f%d y=%d screenY=%02X edge=%02X scene=%02X' % (
    i, rd(0xFFC0) | (rd(0xFFC1) << 8), rd(0xA23B), rd(0xFF98), rd(0xA28B)))
p.button_press('down')
for i in range(90):
    tick()
    print('f%3d y=%5d camy=%4d camyHi=%d scrY=%02X tr=%02X flags=%02X edge=%02X scene=%02X dir=%02X' % (
        i, rd(0xFFC0) | (rd(0xFFC1) << 8), rd(0xFFC8) | ((rd(0xFFC9) & 0xF) << 8),
        rd(0xFFC9) & 0xF, rd(0xA23B), rd(0xA20E), rd(0xA223), rd(0xFF98), rd(0xA28B), rd(0xA221)))
p.button_release('down')
p.stop(save=False)
