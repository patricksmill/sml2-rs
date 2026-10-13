#!/usr/bin/env python3
"""Pair a $05E4-hooked death capture for replay.

The hook fires at the START of each logic frame, so raw record k holds the
state at the END of frame k-1 together with frame k's held keys. The sim's
frame i replays the console's logic frame i-1, so it needs THAT frame's keys
paired with the record-i state:

    out[0] = raw[0]                       (the seed)
    out[i] = raw[i] with keys := raw[i-1] (i >= 1)
"""
import json
import sys

# Per-frame INPUTS read by the console *during* logic frame k, so the sim's
# frame i (which replays frame i-1) takes them from record i-1:
#   - the held keys (the joypad read)
#   - `phase` ($FF97), the speed-dither phase
# NOT shifted: `edge` ($FF98) — the camera's own lookup at the frame's start
# (its position is the record-i state, so its edge is the record-i edge too).
KF = ('right', 'left', 'up', 'down', 'a', 'b', 'phase')

def pair(raw):
    out = [dict(raw[0])]
    for i in range(1, len(raw)):
        rec = dict(raw[i])
        for kf in KF:
            rec[kf] = raw[i - 1][kf]
        out.append(rec)
    return out

if __name__ == '__main__':
    src = sys.argv[1] if len(sys.argv) > 1 else 'death1_raw.json'
    dst = sys.argv[2] if len(sys.argv) > 2 else 'death1.json'
    raw = json.load(open(src))
    out = pair(raw)
    json.dump(out, open(dst, 'w'))
    print(f'paired {len(out)} frames: {src} -> {dst}')
    for i in (0, 1, 2, 139, 140, 141, 420, 421, 422):
        if i < len(out):
            f = out[i]
            print(f'  f{i}: x={f["x"]} y={f["y"]} hvel={f["hvel"]:02X} vvel={f["vvel"]:02X} '
                  f'dead={f["dead"]:02X} uphase={f["uphase"]} right={f["right"]}')
