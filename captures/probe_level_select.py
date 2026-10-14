"""Level entry route: drive the console into an ARBITRARY level from its own map.

Every other `*_capture*.py` in this directory can only reach level 1 (the hub's own
story path). Six backlog capture items need a level other than level 1 - autoscroll
(levels 20/26/31), the scene latch (level 3/20), the boss arena, the `$1413` wall
push-out tiles, a `$23`-bit5 walker, a fire flower. This module is the route they use.

MEASURED ROUTE (2026-10-14, PyBoy, two targets, exact on both):

    boot to the overworld map, state $FF9B = $0C   (the prefix every capture script has)
    poke sMapScreenId ($A68B) := any NONZERO value   (the $3E1D7 entry gate needs screen != 0)
    poke sMapNode     ($A840) := the target node from extracted/map/nodes.bin
    tap A

The console then runs its own entry path - state $0C -> $0B -> $2 -> $3 -> $4 - setting
`sCurLevel = $4218[sMapNode]` at `$3C1A2` (main.asm:18544; NOT `$3D0D7`, which is a castle
node's animation) and loading the real level header. First state-4 frame:

    node  0 -> sCurLevel  0   Mario (34,468)  camera (80,448)   level01 header: 34,468 / 80,448
    node 61 -> sCurLevel 19   Mario (63,212)  camera (96,192)   level20 header: 63,212 / 96,192

`node_for_level(idx)` reads the node index out of extracted/map/nodes.bin ($4218[node] =
level index; $FE = a pure path node). Level 25 (Wario's Castle) has NO map node and cannot
be reached this way; it needs the castle path.

Caveat: the map state is POKED, not driven, so the frames before entry are a poked console,
not a reachable game state. Start recording at state 4 (as the other capture scripts do) and
the captured level is the console's own - the pokes only choose which level, they do not
touch it.

Run as a script to re-verify the route against the manifests:
    /home/pmill/.venvs/sml2-pyboy/bin/python captures/probe_level_select.py [node ...]
"""
import os, sys, pathlib
os.environ['SDL_VIDEODRIVER'] = 'dummy'
os.environ['SDL_AUDIODRIVER'] = 'dummy'

ROOT = os.environ.get('SML2_DECOMP_ROOT', os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'sml2decomp')))
NODES_BIN = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)), '..', 'extracted', 'map', 'nodes.bin'))

STATE = 0xFF9B
S_MAP_SCREEN_ID = 0xA68B
S_MAP_NODE = 0xA840
S_CUR_LEVEL = 0xA269
# Mario Y/X lo+hi, camera Y/X lo+hi (verified: $FFC0 = Mario Y lo, $FFC2 = Mario X lo,
# $FFC8 = cam Y lo, $FFCA = cam X lo)
MARIO_Y, MARIO_X, CAM_Y, CAM_X = 0xFFC0, 0xFFC2, 0xFFC8, 0xFFCA


def node_for_level(level_index):
    """The map node whose $4218 entry is this 0-based level index, or None."""
    table = pathlib.Path(NODES_BIN).read_bytes()[:130]
    for i, v in enumerate(table):
        if v == level_index:
            return i
    return None


def boot_to_map(p, tick, tap, rd):
    """The standard title -> file select -> hub prefix, ending on the overworld map."""
    for f in range(3000):
        tick()
        if f > 300 and rd(STATE) == 1:
            break
    tap('start')
    for _ in range(900):
        tick()
        if rd(0xA277) == 1 and rd(0xA278) == 0:
            break
    p.memory[0xA000 + 67] = 5            # fresh-file counters, as every capture script does
    p.memory[0xA000 + 69] = 5
    tap('start')
    for _ in range(1500):
        tick()
        if rd(STATE) == 0x0C:
            break
    for _ in range(200):
        if rd(0xA277) == 1 and rd(0xA278) == 0:
            break
        tick()


def enter_level(p, tick, tap, rd, node, screen=1, max_ticks=1200, first_frame=None):
    """Poke the map onto `node` and press A; return True once the level is live (state 4).

    `first_frame`, if given, is filled with the register values of the FIRST state-4 frame -
    the frame whose Mario/camera match the level header exactly (60 ticks later the camera has
    already moved on an autoscroll level).
    """
    p.memory[S_MAP_SCREEN_ID] = screen
    p.memory[S_MAP_NODE] = node
    at4 = None
    for i in range(max_ticks):
        tick()
        if i == 40 or i == 200:
            tap('a', hold=8, gap=4)
        if rd(STATE) == 4:
            if at4 is None:
                at4 = i
                if first_frame is not None:
                    first_frame.update(tick=i, curlevel=rd(S_CUR_LEVEL),
                                       mario_y=rd(MARIO_Y) | (rd(MARIO_Y + 1) << 8),
                                       mario_x=rd(MARIO_X) | (rd(MARIO_X + 1) << 8),
                                       cam_y=rd(CAM_Y) | (rd(CAM_Y + 1) << 8),
                                       cam_x=rd(CAM_X) | (rd(CAM_X + 1) << 8))
            if i - at4 >= 60:
                return True
    return False


def _verify(nodes):
    from pyboy import PyBoy

    for node in nodes:
        p = PyBoy(f'{ROOT}/baserom.gb', window='null')
        p.set_emulation_speed(0)

        def rd(a):
            return p.memory[a]

        def tick(n=1):
            for _ in range(n):
                p.tick()

        def tap(btn, hold=6, gap=4):
            p.button_press(btn); tick(hold); p.button_release(btn); tick(gap)

        boot_to_map(p, tick, tap, rd)     # the map is poked per node, so each needs its own boot
        print(f"map: state={rd(STATE):02X} screen={rd(S_MAP_SCREEN_ID):02X} node={rd(S_MAP_NODE):02X}")
        first = {}
        ok = enter_level(p, tick, tap, rd, node, first_frame=first)
        print(f"node {node:3d}: entered={ok} curlevel={first['curlevel']:02X} "
              f"mario=({first['mario_x']},{first['mario_y']}) "
              f"cam=({first['cam_x']},{first['cam_y']})   [first state-4 frame]")
        p.stop(save=False)


if __name__ == '__main__':
    _verify([int(a) for a in sys.argv[1:]] or [0, 61])
