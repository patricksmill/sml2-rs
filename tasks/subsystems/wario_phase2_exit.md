# Wario second-form exit prerequisite

One shared state target: ROM `$AC55` bank 2, COMPLETE
[`source brief`](../handlers/0x0AC55.md). This precedes the full anim23 handler.
Read native guide/PORTING, `actor.rs`, `wario_phase1.rs` as consumer reference,
and the brief. Allowed bounded reads of CameraSim event consumer and existing
stream-scanner tests if needed. Do not load whole object engine or DESIGN.

Touch ONLY new `objects/wario_phase2.rs`, optional sibling test/adapter files,
and `objects/mod.rs` registration. Existing Actor animation helper API:
`animate_four_step_bob(FrameTick) -> Result<(), AnimationFrameError>` where
error has `frame: u8`; worker is completing this prerequisite now.

Use typed Actor logic and explicit effects/adapter. Every frame forces fade
low byte 5, advances four-step sprite cycle BEFORE palette flash, then moves
screen X by 2. Compare NEW wrapped X to 192; on exit clear actor, set done1
without decrementing count, boss phase2, scene0, sprite map1, reseed stream
cursor and install record X2248 ($08C8), Y176 ($00B0), marker0, animation27
normal /29 easy (easy bit0 only). Retain six-left-sentinel/right-end abstraction
from phase1 and its real CameraSim result consumer. Sprite/hitbox metadata may
be derived using mode-B table, like phase1. Preserve no-exit stream state.

Tests: independently derived exact ROM-byte outputs; frame cadence/order,
boundary/wrap, full actor clear, both easy-mode selections including other bits,
fade high byte preservation, unchanged count, actual event consumption and
replacement record scanned as level object; reset/reload restores original stream.
Native behavior tests plus facade/real consumer tests. Source selected helper
contracts established by existing source-verified methods; reuse typed animation
and flash behavior, do not duplicate byte helper bodies. Full core/runtime tests,
fail-and-restore sensitivity required; coordinate mutation window with root.
No sweep/commit/inventory/queue editing. Fight remains gated pending anim23.
