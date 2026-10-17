# Native Wario child fall prerequisite

Target one shared helper: ROM `$AB4D` (bank 2), COMPLETE source brief
[`0x0AB4D.md`](../handlers/0x0AB4D.md), including counted removal `$B157`.

Permitted implementation files: new `objects/wario_child_fall.rs`, optional
sibling tests/adapter, existing `objects/wario_child22.rs` (replace only the
shared helper facade/body; leave anim22 initialization/tests intact), and
`objects/mod.rs` module registrations only. Read `actor.rs`, the native guide,
PORTING, and the named source brief. Keep other handlers and actor API unchanged.

Native update takes an Actor, FrameTick and primary screen Y, and returns an
explicit removal effect. Preserve even-frame cadence, increment-before-compare,
16-pixel row comparison with two-row wrapping offset, full persistent actor
clear, counted-removal underflow, delay latch and done writes. Compatibility
adapter converts both actors with Actor methods and applies effects; do not
write slot fields or clear raw arrays in native logic. Retain existing callers
and old ROM-byte tests for child22/25. No new dispatch arm is required.

Acceptance: direct native boundary/cadence/removal tests; existing child22/25
and dispatch tests unchanged; sensitivity mutations fail and restore; full core
tests. Run frozen sweep once at end of child-fall migration package.
