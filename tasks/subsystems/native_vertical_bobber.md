# Native migration of vertical bobber (anim58)

One handler ROM `$591F9` bank16hex22, COMPLETE regenerated brief
`tasks/handlers/0x591F9.md`. Latest handler is byte-oriented despite its commit
title; migrate behavior to native Actor/context/effects per user guide.

Read PORTING/native guide, named brief, actor.rs native APIs,
objects/handler_0x591f9.rs (352 lines incl old byte tests) and ONLY FALL_SPEED /
BOUNCE_SPEED literal definitions in staging.rs. Touch new bounded
objects/vertical_bobber.rs domain (+ optional sibling native tests), existing
handler_0x591f9.rs facade adapter + retain old ROM-byte tests, mod.rs registration
only. Keep dispatch arm/facade signature stable; no unrelated helpers/handlers.

Use Actor.animate_two_frame_toggle(frame,2), native u16 position.move_y(signed),
Actor.motion_step(profile,15), FrameTick.every(4). Initialize contact86/flash16,
then compare full native player_ref_y to actor.position.y. Source equality selects
up state1 but world-facing0; less selects state1/facingFF; greater selects
state2/facing1. Return typed Ordering/relation effect for adapter; no raw facing
bytes/register names/slot offsets in domain logic. Use shared source literal
BOUNCE_SPEED/FALL_SPEED once; no duplicated motion math/clamping/fabricated steps.
Old curve_step_15 clamps phase with min15, contrary to source; native method must
read current phase then equality-only advance or reject missing table entries.
Keep source unknown-state diagnostic after animation prefix. Capture adapter
world inputs once, Actor conversions only; scratch and every unwritten byte stay.

Retain all old valid ROM-byte tests. Add native phase/timer/frame/position wrap,
y relation boundary/equality, unknown state/profile source cases, exact source
full fall/bounce samples (including source valid phase16..21 fall tail), and actual
ObjectSim::step dispatch consumer, dead freeze if caller applies. Meaningful
sensitivity incl source clamp removal, movement/order, dispatch when testable.
Full core/runtime. Ask root before mutations/fullsuite (ending_scene owns lane).
No commit/sweep/queue/inventory, no captures changes; preserve5B36D untrackedfile.
