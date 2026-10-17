# Native shared sprite alternation

One shared animation behavior family: bank2 ROM `$B2EF/$B302` and `$B321`,
COMPLETE briefs `tasks/handlers/0x0B2EF.md`, `0x0B321.md`. Gate every eighth
frame and XOR sprite1 or31 respectively. Native method may generalize cadence
period but source call sites use8. Source modifies no other persistent actor data.

Read PORTING/native guide, named briefs and actor.rs. Touch ONLY actor.rs
small native methods (prefer existing actor_screen/actor_motion untouched),
actor_tests.rs for tests OR new bounded actor_animation.rs sibling and own tests
with registration in actor.rs. No existing handler or facade edits. Keep
actor.rs <400lines. Expose Actor::animate_two_frame_toggle(frame:FrameTick,
period:u64), and Actor::animate_sprite_blink(frame,period,sprite_mask:u8).
Two-frame method delegates to generic blink with sprite-mask1; generic blink
owns cadence and XOR. Caller-specific sprite mask31 remains literal source rule,
not a slot/flag offset. New Wario23 worker will call both, removing local helper.

Tests source literal sprites (including wrapping/high bits/31), all frame byte
cadences, untouched other fields and direct native state; sensitivity mutation
fail/restore then fullcore. Ask root before mutation/fullcore (handler worker
currently runs focused tests). No commit/sweep/queue/inventory changes.
