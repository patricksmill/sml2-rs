# Ending sequence prerequisite contracts

Original completion still requires the entire state19 ending, credits, sound,
rendering and normal-play entry. Exact state dispatch is ROM6801A (20 entries).
0..3 each increment once/frame;4 calls29DD;5..19 include real sound sequencing.
Each frame runs one state then sprite cleanup52E7. Fresh whole6800F packet is
INCOMPLETE at computed audio dispatch/budget; do not supply invented contracts.

Verified native prerequisites:
- EndingEntry integrates scripted exit/fade/initializer through both live/replay
  heads. It explicitly returns SequencePending for downstream state19.
- EndingScene loads source6144 tilebytes/1024 mapIDs and exact initial display
  state. Extraction independently matches ROM45800/6948E.
- player::append_cells implements5297 (palette/priority OR, signed wrapping),
  consumed by existing player draw_cells and native ending pose composition.
- ending_sprites implements6845F/68492/684C5/684E0; paired routine falls through
  secondary, whose palette override is false and priority inherited. Burst
  requests slot1 ID24 on odd frames; request is not playback/acknowledgment.
- SpriteBatch implements visible-cell cleanup52E7, including equal-count next-Y
  clear. Scene load clears visible cells but preserves previous used count.

Remaining source graphics prerequisites:
- Transfer26D7: source home.asm5107..5161 copies until remaining low&3F==0,
  normally64bytes/frame; unaligned count first copies remainder. Active only
  clears at zero. State19 skips sound within transfer service.
- VBlank selects transfer BEFORE credits text/tile service; transfer suppresses
  credits service but not the shared fade decrement. Preserve frame ordering.
- Credits68572: four20-byte rows, destination9A20+(4-phase)*32, ASCII-minus48,
  space->FF, FF rowmarker fills20 FF, FE nextpointer sets done, phase-- and
  finalrow fade256. Tool currently cannot prove interrupt return/bank restore;
  resolve source-tool contract before dispatch, not fake native dependency.
- Raster68400 uses exact16-value wave [0,5,9,12,13,12,9,5,0,-5,-9,-12,-13,-12,-9,-5]
  selected by (((frame&3C)>>2)+scanline)&15, active below row16 to rasterY.
  Ending6809E/68267 split signed/unsigned background addressing at row111.
  Native rendering needs per-row projection and mutable map/tiles/palettes.

Sound dependency:
_UpdateSound10030 is entirely missing as a native driver. Normal path sequences
pulse/wave/noise slots then music; new music path differs, common tail clears
requests/mode. Driver mode/pause, music streams/data and waveform restore are
real behavior. Tiny effect-specific sequencing does not close this contract.
Ending stateC calls sound BEFORE burst writes request1, so preserve next-tick
request delivery. No no-op sound callback or immediate transfer completion.
Bounded source audit found exact candidate start/tick table declarations at
1007E/100AE (pulse IDs1..24),106FA/1072A(wave1..24),10E64/10E7A(noise1..11).
Declaration bytes match ROM, but exhaustive request bounds remain unproven:
main.asm5058/5146 take animation IDs,37481..37493 uses castle index-dependent
requests. Do not turn adjacent declaration lengths into exhaustive CFG edges.
First independently ready unit: pulse effect6, COMPLETE101F3/101F9 packets and
exact frames1020B..10216. Its native sequencer is a prerequisite, not a sound
output or ending completion claim.

Runtime caller contract:
Non-gameplay frame head already ticks fade once before ending dispatch. Do not
add a second decrement. State changes dispatch next frame. Present prior sprite
batch at VBlank, increment full wrapping frame byte and reset current drawing
batch, then dispatch one substate and cleanup. Existing ending native scene is
not yet loaded/rendered by normal runtime. Integration must consume real assets,
mutable scene, native audio effects and per-row display state with evidence.
