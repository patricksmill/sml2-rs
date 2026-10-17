# Native anim32 buried enemy, ROM589A6

One handler task: COMPLETE tasks/handlers/0x589A6.md, bank16hex22, CPU49A6,
modeA anim32. Native handler589A6 + explicit adapter + sibling local tests,
mod registration and single dispatch arm only. Existing typed helpers:
Actor.face_toward, animate_two_frame_toggle(frame,2), motion_step(FALL_SPEED,21).
Shared terrain_probes prerequisite exposes wall_blocked(Position,right,reach11),
grounded(Position), final compatibility point/flags. Reuse instead of duplicate
probe math. Read scope PORTING/native guide, exact brief, Actor APIs, named
modules, staging FALL_SPEED literal ONLY, LevelLayout/ObjectTables APIs and
bounded normal object-loop fixture/consumer ranges. No full engine/source.

Source rules:
- Dormant0 writes contact32 everycall and wakesstate5 when SIGNED-WRAPPED
  lowbyte X distance magnitude<64; full worlddistance is not the source rule.
- States1/2 animate everyevenframe. Evenframes MOVE X1 then wallprobe, writing
  sfx2=1 if blocked (never reverse or undo thatstep). Oddframes groundcheck;
  if not grounded switchstate+2 WITHOUTfallstep thisframe.
- States3/4 evenframe same horizontalstep/probe/sound but no animation. Oddframe
  groundcheck: grounded resetsmotionphase0 and state-=2; otherwise fallprofile
  currentphase then equality-only advancementcap21 and signed nativeY+sample.
- State5 animates onebit every2frames and increments animation_frame everycall;
  ONLYnewcounter==32 resetscounter0 and enters6. Preserve counter255 wrap.
- State6 every8frames increments sprite; onlynewsprite==62 triggers shared
  facing state1/2,FULLflipoverwrite,contact77. If stood_on, carryMario up by
  actor X HIGH BYTE. Source58A69 loadsA8, but3D10usesB, and5B3E0 left B=Xhigh;
  dead A load must not be mistaken for the carryamount. Preserve0/1/2/255.

Carry adapter accumulates self.mario_y_adjust from earliercurrentframe or Mario
position; PlayerSim.apply_object_result consumes. Camera later derives deltas
from previousposition; no new rawWRAM scrollaccumulator field. Reuse existing
rising_platform producer/consumer fixture patterns. Faces also project actual
source global facing FF/0/1. Unsupported states/profiles diagnose, no clamp or
fabricated speeds. Preserve every other slot byte.

Test all6states+init, parityframes, thresholds63/64 and wrap, equalitycounter32,
sprite62 and wraps, motioncap21, mixedwall/ground thresholds and alignment,
source scratch, existingflagpreservation when noprobe, coordinateborrow carry,
full32 byte fixtures, realanim32 ObjectSim.step/deathgate and PlayerSim consumer.
Required sensitive mutants then restored fullcore; no sweep until level2package
(58848/589A6/58A6F/58B43). Root reviewscoverage/inventory and commits onlyscope.
