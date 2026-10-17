# Native Wario trajectory child (anim27)

One handler ROM `$AFE0` bank2 CPU `$6FE0`, COMPLETE
`tasks/handlers/0x0AFE0.md`. Parent final form is pending; do not enable arena.
Read PORTING/native guide, named brief, actor.rs APIs, actor_sprite_sequence.rs
production method, and bounded existing wario/form2_integration.rs caller pattern.
Touch ONLY new wario/trajectory_child.rs and trajectory_child_adapter.rs plus
bounded sibling tests, wario/mod.rs registrations, dispatch.rs one arm. Shared
helpers/actor API/queue/inventory untouched. Ask if missing native prerequisite.

Every handler invocation first calls Actor::animate_four_frame_cycle(frame,2).
Then state0 sets contact WARIO_CHILD, state1 if flipH clear/state2 if set,
sprite104, animation_frame0, and screenY-16. Direction is chosen from sprite
flip, not travel_direction. States1/2 read current phase of horizontal profile
ROM `$B039..$B07D` (69 bytes). Last byte128 is restart sentinel: phase=58 and
return without movement/Y/advance. Other cells subtract/add screenX by distance
according to state. Compare NEW X unsigned>=176; counted remove immediately
before Y lookup/phase increment. Otherwise add SIGNED vertical profile sample
from `$B07E..$B0C1` (68 cells), increment phase wrapping. Use const native tables
(single copy) transcribed independently from ROM; no emulator/table byte plumbing.
No `step_picker` because this source does not increment until AFTER movement.
Unknown states/invalid phases report source diagnostic AFE0 after any animation
prefix; do not invent zero steps. Native returns explicit counted-removal effect;
adapter preserves source decrement wrapping, done1 and clears complete Actor.

Table expectations must be independently ROM-derived. Verify every trajectory
pair, restart/frame cadence, left/right/zero-distance/wrap/offscreen boundaries,
initialization preserving unwritten fields, whole removal bytes/count underflow,
invalid states/phase, boss table dispatch and dead freeze, actual shared-spawn
helper creates child27 then boss driver runs it. Parent anim26 pending separately.
Full core/runtime, meaningful fail-and-restore mutations (dispatch and trajectory/
update-order boundaries). Ask root before mutation/full-suite lane. No sweep/
commit/queue/inventory changes. Bounded modules<400lines; test files may split.
