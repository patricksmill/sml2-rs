# Complete native Wario second form (anim23)

One handler: ROM `$AB8B` bank2 CPU `$6B8B`, COMPLETE brief
[`0x0AB8B.md`](../handlers/0x0AB8B.md). Port all states0..11 and hit prologue;
add exactly one boss dispatch arm. Normal def6 entry stays gated until final fight
ending is ready. Do not claim normal fight completion from handler dispatch.

Read PORTING/native guide and exact brief; bounded production API sections of
actor.rs, boss_damage.rs, boss_stun.rs, boss_spawn.rs, wario/phase2_exit.rs (skip their
tests), and existing wario/form1_integration.rs for real camera input test pattern.
Avoid whole boss.rs/mod.rs/objects tests/main.asm. Compiler-directed bounded
field/caller excerpts allowed. Target read budget15k; these APIs/source suffice.

Touch ONLY new objects/wario/form2.rs (native domain), wario/form2_adapter.rs, bounded
sibling tests/integration files; dispatch.rs one arm; mod.rs registrations.
Existing public ObjectSim facade remains stable. Do not edit shared helpers.
If a missing native contract blocks you, report it before reimplementing logic.

Ready APIs:
- Actor motion_step(&[u8],cap) -> Result<u8,MotionProfileError>, flash.animate_palette,
  rise_step(RiseProfile::{Full,Short}) and profile.cap(), animate_four_step_bob.
- Actor.face_toward_screen(target_x) -> Facing replaces full flip, equality right;
  screen.horizontal_distance_to returns signed wrapped magnitude (128 preserved).
- boss_stun::update_hit_stun(Actor,FrameTick,HitStunContext) typed saved context.
- Native damage and spawn helpers have facade adapters: legacy facade may call
  boss_damage_pass before Actor conversion, since it delegates to typed logic.
  Keep state matching/movement/flags/source timing in native domain function.
- Phase2 state8 may return explicit shared-exit action to adapter; adapter calls
  wario_phase2_exit after saving native Actor. It already contains typed exit
  behavior and tested camera/stream effects; do not duplicate it.
- Spawn state0 returns typed BossSpawnRequest/effect. Preserve current spawn_attr
  (source writes animation22 + sprite14 only); adapter may call shared native
  spawn facade after conversion. New child should match real boss-loop ordering.
- ObjectSim.player_screen_x is live derived boss-entry input; separate worker is
  finishing it. Other world fields: fff0, done, boss_hit_latch, boss_timer_reload,
  spawn_sprite, saved_frame_state, saved_type, remove_mode, boss_sprite, fade_timer,
  ending_door_active, timer_low, boss_delay, camera_pan_set, boss_transition,
  sfx0/sfx1/sfx2, facing, spawn_anim/spawn_attr. Snapshot descriptive native
  context fields, return explicit effects; no register names in domain logic.
- ObjectTables.boss_picker_profile supplies forward curve. FrameTick contextual
  source cadence. World facing compatibility writes FF(left)/1(right) only after
  the source face helper in state3; preserve otherwise.

Table is indexed state-1 for states1..11, targets in brief. State0 spawns child22
and enters7/timer32. State7 waits for child latch then decrements timer with wrap;
completion enters11/sprite114 and consumes delay. State11 is intro. State6 forces
fade low5; state8 shared exit. Wait-before-move states9/10 return even when a
nonzero timer decrements to zero. State9 uses SHORT rise/forward cap12, state3
FULL rise cap36, state2 and10 forward cap21. Preserve source write/compare order,
raw unknown persistent values, all screen wrapping, full flip assignments versus
XOR toggles. Reject unsupported states/data with diagnostic AB8B, no fabricated
profile movement. Source damage prologue saves sprite/state/kind, substitutes
sprite82/reload only for stomp latch, then contact NONE/state1.

Acceptance: direct native + independently ROM-derived exact bytes for all12
states, initialization, hit/recovery/defeat, terminal timer0/1/wrap, profiles,
left/right/equality/edge bounds, movement-before-proximity, timer/frame gates,
reset/removal. Real boss dispatch and dead freeze; state0 child creation + child
fall latch into parent state7/intro; camera impact -> camera -> next-frame player
guards; state8 actual event/stream handoff. Reuse existing shared tests but add
handler consumer paths. Full core/runtime tests; meaningful fail/restore mutations
including dispatch and new camera producer. Ask root before mutation/full suite.
No sweep/commit/queue/inventory edits. Keep modules bounded (<400lines each).
