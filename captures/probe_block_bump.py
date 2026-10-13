"""Block-bump probe: what does the console do when Mario bonks a `$01` block?

Level 1's early section puts Mario on the row-31 floor (y=468) with the row-28
tiles as blocks above him: `$01` cells at (28,22)/(28,24) (x 352/384) are the
nearest ones, so a short walk east plus a jump reaches them. The bonked tile
goes through `$1DFE` (ceiling probe: hit on `t == $7E` or `t < $38`) → `$15CF`
(y snap, vvel 24) → `$15F4` → **`$1993`**, the classification this probe exists
to measure:

  tile `$7E`/`$02` → type 3 ; `$00` → type 2 (small) / 1 (big) ;
  `$01` → spin-bounce `$FF`, type 3, then `$2C77` (`$591F` = +1 coin) ;
  `$03` → type 6 (small) / 5 (big) ; `$04`-`$37` → nothing.

`$1993`'s `$19F1` tail does `$2C6C` = the slot alloc (`$5960`: find a free
`sBlockAnimSlots` ($AA80, 8 x $10: +0 type, +1 timer, +2/+3 Y, +4/+5 X), store
type, timer := 1, the block's rounded position) and sets `sSpinJumpBounce`
($A220) = `$FF`. The per-frame update is `$5997` (called from the frame loop as
`$2C82`, i.e. AFTER the `$2D0A` frame selection, so its `$FFC6` write wins):
timer++, then per type: type 3 (`$5B06`) draws `$FFC6 = 66` with the `$5BB4`
bob, at timer 2 writes tile 24 + blanks the 2x2 BG tiles, at timer 8 writes
`$F8-$FB` (the "used block" art) and frees the slot. Types 2/6 draw 66 too
(`$5B8A`); type 1/5 draw 101/102 (`$5A07`); type 4 draws the `$5AF9` cycle
`[$6D,$6E,$6F,$6E]`.

Note the alloc's timer is 1 and the update increments before dispatching, so
the first *drawn* frame is timer 3 and the slot dies at timer 8 — a 5-frame
`$FFC6 = 66` window, which is what the capture sees.

This probe drives the level-0 start, walks east to x ~340, settles, jumps twice
(second jump: is the block still solid after the bump?), and logs the bump
state per logic frame. It writes `$OUT/probe_block_bump.json` (the per-tick log)
and prints the window. Nothing here writes a capture for the suite.
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

def bslots():
    return [rd(0xAA80 + i) for i in range(0x10)]      # slot 0 raw 16 bytes

def log(keys):
    return dict(
        x=rd(0xFFC2) | (rd(0xFFC3) << 8), y=rd(0xFFC0) | (rd(0xFFC1) << 8),
        hvel=rd(0xA200), vvel=rd(0xA201), ground=rd(0xA214) != 0,
        frame=rd(0xFFC6), ffbb=rd(0xFFBB), ffc4=rd(0xFFC4), ffc5=rd(0xFFC5),
        ffc6=rd(0xFFC6), scrolly=rd(0xA2B0), scrollx=rd(0xA2B1),
        ftile=rd(0xA20D),                                # sCollisionTile
        coinlo=rd(0xA262), coinho=rd(0xA263),
        spin=rd(0xA220),                                 # sSpinJumpBounce
        btype=rd(0xA23E), btimer=rd(0xA23F),             # sBlockAnimType/Timer
        bylo=rd(0xA240), byhi=rd(0xA241), bxlo=rd(0xA242), bxhi=rd(0xA243),
        slot0=bslots(),
        cell28_22=rd(0xB000 + 28 * 256 + 22),
        camx=rd(0xFFCA) | ((rd(0xFFCB) & 0x0F) << 8),
        camy=rd(0xFFC8) | ((rd(0xFFC9) & 0x0F) << 8),
        ff97=rd(0xFF97), dead=rd(0xA224),
        right='right' in keys, a='a' in keys, b='b' in keys)

LOG = []
CUR = set()
HITS = []

def bonk_hook(tag):
    def cb(_ctx):
        px = rd(0xFFB9) | (rd(0xFFBA) << 8)
        py = rd(0xFFB7) | (rd(0xFFB8) << 8)
        HITS.append((tag, rd(0xFFC2) | (rd(0xFFC3) << 8), rd(0xFFC0) | (rd(0xFFC1) << 8),
                     px, py, rd(0xA20D),
                     rd(0xB000 + (((py - 16) >> 4) & 0xFF) * 256 + (((px - 8) >> 4) & 0xFF))))
    return cb

# $15CF = the ceiling probe returned carry (the bonk entry); $1993 = the
# classification. Both are hookable code addresses in bank 0.
p.hook_register(0, 0x15CF, bonk_hook('bonk'), None)
p.hook_register(0, 0x1993, bonk_hook('1993'), None)

def hook(_ctx):
    # Sampled at the frame START ($05E4), i.e. the convention a capture uses:
    # the values are the PREVIOUS frame's end state, so a block bump's
    # `$FFC6 = 66` draw shows up here one frame after it happened.
    LOG.append(log(CUR))

p.hook_register(0, 0x05E4, hook, None)

def run(keys, n, tag=''):
    for k in keys: p.button_press(k)
    CUR.clear(); CUR.update(keys)
    for _ in range(n):
        tick()
    for k in keys: p.button_release(k)
    CUR.clear()
    f = LOG[-1]
    print(f'  [{tag}] n={len(LOG)} mx={f["x"]} my={f["y"]} ground={f["ground"]} '
          f'vvel={f["vvel"]} frame={f["frame"]} coin={f["coinlo"]:02X} '
          f'btype={f["btype"]} btimer={f["btimer"]} spin={f["spin"]:02X} dead={f["dead"]} '
          f'slot0={[f"{b:02X}" for b in f["slot0"]]}')

# walk east at floor level (row 31) to just west of the $01 block at x=352.
# The ceiling probe is (x+16, y+28) and the $01 cells sit at cols 22 (x 352-367)
# and 24 (x 384-399): col 22 wants x+16 in [352,367] -> x in [336,351], and the
# settle costs a few px of coast, so stop at 338 and nudge back if we overshoot.
run(['right'], 60, 'walk-1')
while LOG[-1]['x'] < 338:
    run(['right'], 1)
run([], 20, 'settle-1')
for _ in range(20):
    if LOG[-1]['x'] > 347:
        run(['left'], 1)
        run([], 2)
    else:
        break
f = LOG[-1]
print('  at block: mx=%d my=%d ground=%d probe-x=%d col=%d cell=%02X' % (
    f['x'], f['y'], f['ground'], f['x'] + 16, (f['x'] + 16 - 8) >> 4,
    rd(0xB000 + 28 * 256 + ((f['x'] + 16 - 8) >> 4))))

# jump 1: bonk the block
run(['a'], 30, 'jump-1')
run([], 40, 'after-jump-1')
# jump 2: is the block still solid (the tile write is BG-only)?
run(['a'], 30, 'jump-2')
run([], 40, 'after-jump-2')

json.dump(LOG, open(f'{OUT}/probe_block_bump.json', 'w'))
print('log frames:', len(LOG))

# --- report the bump window -------------------------------------------------
print('\nframe  x    y   vvel frame ffbb btype btimer spin coin slot0[0] slot0[1] ffc4 ffc5 cell(28,22)')
lo = max(0, len(LOG) - 130)
for i in range(lo, len(LOG)):
    d = LOG[i]
    print(f'{i:>5} {d["x"]:>4} {d["y"]:>4} {d["vvel"]:>4} {d["frame"]:>5} {d["ffbb"]:>4} '
          f'{d["btype"]:>5} {d["btimer"]:>6} {d["spin"]:>4} {d["coinlo"]:02X}   '
          f'{d["slot0"][0]:02X}     {d["slot0"][1]:02X}    {d["ffc4"]:>4} {d["ffc5"]:>4} '
          f'{d["cell28_22"]:02X}')
print('\n=== $15CF/$1993 hits (tag, mario x, mario y, probe x, probe y, sCollisionTile, cell) ===')
for h in HITS:
    print('  ', h)

print('\n=== bump draw ($FFC4/5/6) vs the slot, per logged frame ===')
print('  i   ffc4 ffc5 ffc6 | slot0 type t Y     X     | predY predX | cam(y,x) scroll(y,x)')
for i, d in enumerate(LOG):
    if not d['slot0'][0]:      # $A23E is cleared by the slot walk; use slot 0
        continue
    s = d['slot0']
    sy = (s[3] << 8) | s[2]
    sx = (s[5] << 8) | s[4]
    print(f"  {i:>3} {d['ffc4']:>4X} {d['ffc5']:>4X} {d['ffc6']:>4X} | "
          f"{s[0]:>2} {s[1]:>2} {sy:>5} {sx:>5} | "
          f"{d['camx']:>5} {d['camy']:>5} {d['scrollx']:>3X} {d['scrolly']:>3X}")

print('\nframes with frame==66:', [i for i, d in enumerate(LOG) if d['frame'] == 66])
print('frames with btype!=0:', [(i, d['btype'], d['btimer']) for i, d in enumerate(LOG) if d['btype']])
print('coin changes:', [(i, d['coinlo']) for i, d in enumerate(LOG) if i and d['coinlo'] != LOG[i - 1]['coinlo']])
p.stop(save=False)
