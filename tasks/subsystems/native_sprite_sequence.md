# Native sequential four-frame sprite cycle

One shared animation behavior, COMPLETE bank2 source briefs `$B3B9` and
`$B3C0` in tasks/handlers. Both gate then enter `$B3C5`; cadence2 or4.
Advance animation_frame wrapping. If new phase equals4, reset phase0 and
subtract3 from sprite wrapping; otherwise increment sprite wrapping.
Do not normalize other phases: phase255->0 still increments sprite, not subtract3.

Read PORTING/native guide, named briefs, actor.rs animation API. Touch ONLY
actor.rs registration (keep<400lines), new bounded actor_sprite_sequence.rs
with method+tests. Expose reusable Actor::animate_four_frame_cycle(FrameTick,
period:u64). Future anim27/28 call with2/4; no handlers in this task. Source
has no table lookup or extra validation. All other fields/flip stay untouched.

Test literal all phase/cadence/sprite wrapping cases, dirty slot preservation,
unknown/overflow phases, periods2/4; fail-and-restore sensitivity and full core.
Ask root for test lane (boss-clear correction currently owns it). No commits,
sweep, queue/inventory or existing handler edits.
