# Complete native rising platform

Source COMPLETE tasks/handlers/0x58E12.md, ROM58E12 bank16hex22, modeA47,
modeB47/97. Continue pre-existing uncommitted domain implementation.
Scope objects/rising_platform.rs, sibling native/integration tests and adapter,
objects/mod.rs registration, dispatch.rs one arm, Actor travel-span accessors.
Remove unfinished adapter/carry additions in >400line walkers.rs; keep its
committed content unchanged. No raw offset/register handling in native logic.

Source contract: init contact32,state3. Rider activation sets state1 and both
remaining/limit64 with no movement. Rise while riding: if remaining>0 decrement,
move platform Y-1, carry Mario Y-1. Step off: state2,delay16, no movement.
Return while unoccupied: countdown reaches0 and moves in the SAME call; compare
remaining to limit by EQUALITY (not >=), then wrapping increment and Y+1.
Reboarding sets state1,timer0 without movement. Unsupported states diagnose
explicitly rather than silently initialize. Preserve every unwritten slot byte.

Native carry stages Mario Y (accumulating earlier same-frame carries); existing
PlayerSim::apply_object_result consumes it. Camera derives vertical deltas from
its previous post-player Y latch, so object carry after camera is consumed next
camera pass. Verify this actual producer/consumer path instead of adding raw
scroll accumulator emulation.

Tests: retain original lifecycle fixtures, independent full-slot bytes including
reserved fields, equality-only return above limit, timer expiry, position/count
wrap, no carry on descent/wait, unsupported state, all3 source table profiles,
real ObjectSim::step and death freeze, dispatch->player->camera carry. Verify
source-sensitive mutations, full core/runtime suites and frozen sweep. Refresh
handler coverage; pending handler aggregate stays pending. Focused commit.
