# Randomness (`rDIV`, `rTIMA`)

Audited 2026-10-21 against `baserom.gb`. The game has no RNG routine: every
random choice reads the hardware divider `rDIV` (`$FF04`), sometimes mixed
with `$FF97` and the shared accumulator `sMapRandAccum` (`$A7B7`). The ROM
has 25 DIV reads in code: 13 are `ldh a,[rDIV]` (`F0 04`), 10 are
`ld a,[$FF04]` (`FA 04 FF`, the decomp's `ldh_a_n $04`), and 2 are
`ld a,[c]` with `c = 4`. The other 111 `F0 04` byte pairs in the ROM fall in
graphics, level data and sprite lists. One routine also reads `rTIMA`. No
code writes DIV.

A native port can't reproduce the console's random values, because they
depend on CPU time. It must make the same choices from the same bits, at the
same rates. Each read below has a unit test that a sensitivity mutation of
the port's branch fails. Addresses are ROM offsets.

## Reads

| ROM | Use | Port | Test |
|---|---|---|---|
| `$979D` | door boss: `& 3` into `sDoorPhase` when the mouth spawns its child | `bosses/door/adapter.rs` | `handler_0x96fe_mouth_spawns_then_returns_to_the_intro` |
| `$97FE` | door boss: `& 3 != 0` flips which side the door appears on | `bosses/door/adapter.rs` | `handler_0x96fe_intro_waits_out_the_fade_then_places_the_door` |
| `$9F75` | patrol boss: `& 3 != 0` skips the rise | `bosses/patrol/behavior.rs` | `patrol_boss_rise_cadence_distance_divider_and_selected_child_busy_gates` |
| `$A220`, `$A275` | sweep boss: `& 3` volleys at the end of each arc (left, right) | `bosses/sweep/behavior.rs` (one path) | `handler_0xa0b9_all_arc_samples_wrap_and_exact_endpoints` |
| `$A414` | circuit boss: `& $3F == 0` adds sound 3 | `bosses/circuit/behavior.rs` | `handler_0xa372_live_animation_and_sounds_use_old_defeat_byte_all_cadences` |
| `$AB1D` | Wario's moving child: bit 0 picks the respawn edge | `bosses/wario/moving_child_adapter.rs` | `handler_0x0aaa5_real_divider_parity_selects_exact_respawn_bytes` |
| `$58A9E`, `$58AEF` | resting walker: `& $3F == 0` on even frames stops to rest (each direction) | `enemies/resting_walker.rs` | `handler_0x58a6f_divider_mask_animation_boundaries_and_post_move_wall` |
| `$5913B` | emerging tracker: `& 7` is the direction scan's start | `actor_direction_allocation.rs` | `direction_allocation_rom_branch_witnesses_and_divider_aliases` |
| `$3C13E`, `$3C15D` (`ld a,[c]`) | map bookkeeping: XORs into the key latch every frame and the random latch every 40th | `map_sim.rs` `bookkeeping` | `the_bookkeeping_counts_frames_and_ticks_at_40` |
| `$3CF09` | hub: `& $72 == 0` arms the sparkles | `map_sim.rs` `hub_decor_tick` | `sparkles_arm_from_the_divider` |
| `$3CEDF`, `$3CEF2` | hub: bits 1-3 and 5-7 place the two sparkles | `map_sim.rs` `set_sparkles` | `sparkle_positions_come_from_separate_divider_bits` |
| `$3DB4E` | Space: blink roll through `sMapRandAccum` | `map/animation_space.rs` | `space_blink_random_mixes_swapped_div_accumulator_and_global_frame` |
| `$3DBA8` | Space: the drifter's respawn coordinate | `map/animation_space.rs` | `space_drifter_wraps_its_coordinate_and_only_respawns_on_counter_wrap` |
| `$3DBCE`, `$3DBFC`, `$3DC2A` | Space: `& $1E` picks each star's position | `map/animation_space.rs` | `space_stars_use_rom_frames_positions_and_staggered_eight_frame_gates` |
| `$3DE2D` | Turtle: gate patch through `sMapRandAccum` | `map/animation_turtle.rs` | `turtle_gate_patch_random_and_secret_growth_rom_bytes` |
| `$3E00F` | Wario castle: `& $7E == 0` strikes lightning | `map/animation_wario_castle.rs` | `wario_castle_lightning_position_and_three_stage_palette_flash` |
| `$3E17E` (+ `rTIMA`) | Wario castle: the wanderer's next direction, retried until it changes | `map/animation_wario_castle.rs` `choose_movement` | `wario_castle_blink_gate_accumulator_wrap_and_random_rejection` |
| `$60482`, `$604C2` | bell room B: rung picks through `sMapRandAccum`, retried until valid | `castle_room.rs` `bell_b_pick` | `bell_b_repicks_both_rungs_from_the_shared_shuffle` |

## The divider clock

The port advances DIV by one console frame, 70,224 cycles (274.31 DIV
counts), per game tick, in every host: desktop, CLI, headless runs, replays
and the TAS bridge. Over its 4,096-frame period every DIV byte comes up 16
times, so a check such as `& $3F == 0` fires on 1 frame in 64, as it does on
the console. Native play seeds the origin from the wall clock at launch.
Replays and tests start at 0 (`VirtualDividerClock`,
`the_clock_visits_every_divider_value_equally`).

Before this audit, native play sampled wall time and replays advanced 1/60 s
per tick. At exactly 60 Hz, DIV moves 273.07 counts per tick and only ever
takes 15 values. Every masked check then fired either about four times too
often or never, depending on the origin. A steady 60 Hz display left the
desktop close to that case too.

## Residuals

- **One sample per frame.** The port feeds each frame one DIV byte. On the
  console, reads later in the frame see a later DIV, a few counts on per
  object update. So in the port, two objects that read in the same frame read
  the same value: two resting walkers stop on the same frame, and two trackers
  emerging together start their direction scan at the same point. Each one's
  rate is right; only their correlation differs. The two retry loops
  (`$3E17E`, `$604xx`) advance the value per try, as the console does.
- **The values themselves.** These depend on CPU time and on the console's DIV
  phase at power-on. Both are outside a native port
  ([frame-clock.md](frame-clock.md)).
