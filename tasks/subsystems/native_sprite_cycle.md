# Native four-step boss animation

One shared helper: ROM `$B339` bank 2, COMPLETE
[`source brief`](../handlers/0x0B339.md). ROM `$B356..$B359` literal deltas
are `FE 01 FF 02` (signed -2, +1, -1, +2). Read-before/index order is NOT the
motion helper's order: gate every fourth frame, increment animation frame,
reset only on equality with four, then apply indexed signed delta to sprite.

Permitted files: `actor.rs` for a method (or bounded sibling actor animation
module plus lib registration), local actor tests, and
`objects/boss_sprite_cycle.rs` to replace its helper with an Actor conversion
adapter while preserving old ROM-byte tests. No unrelated Actor layout/API
changes. Read native guide, PORTING, named brief and these files only.

Provide `Actor::animate_four_step_bob(FrameTick)` as reusable domain behavior.
Preserve source wrapping and reachable indices; explicitly report invalid table
indices rather than invent a delta. Existing facade may retain its established
panic on invalid source-unreachable indices; do not change its signature or old
tests. Keep conversion inside the Actor conversion API. Cadence, all four phases,
both carry directions, unrelated fields and malformed-index behavior require
native tests plus existing ROM-byte tests. Full core gate and fail-and-restore
sensitivity check required. Root coordinates mutation window and sweep/commit.
