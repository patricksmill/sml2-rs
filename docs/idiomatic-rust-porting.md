# Native Rust porting guide

The target is a clean native Rust game with original-game behavioral parity.
Translate game rules, rather than CPU instructions. Aim for routines four to
five times shorter than their assembly through native types and shared behavior.
Keep exact source constants, collision geometry, frame cadence, update ordering,
and observable wrapping and signed comparisons.

## Domain logic

Handlers use typed actors, positions, states, directions and interaction flags.
No slot indexing, `B_*` offsets, register names, HRAM addresses or simulated
carry/borrow plumbing belong in handler logic. Combine world-coordinate byte
pairs into `u16`; use signed displacement helpers. Use enums or state indices
with `match`, descriptive interaction methods and named animation operations.
Temporary calculations are local Rust values, never emulated HRAM scratch.

Shared source routines become reusable `Actor` or world methods: face toward
Mario, animate a two-frame toggle or four-step bob, probe a wall, follow a
gravity curve, and despawn with an explicit mode. Preserve the source contract
before reusing a helper; similar CPU addresses in different banks are distinct.
Keep lookup tables as exact constants or extracted data.

## Slot compatibility boundary

`Actor::from_raw_slot` and `Actor::to_raw_slot` contain the slot-byte conversion.
Game logic operates on named fields. The boundary preserves every persistent
slot value, including reserved fields and unknown enum/flag values, so replay
can still compare all 32 bytes. Do not give the native actor a raw-byte backing
store that handlers manipulate indirectly. Existing engine adapters may copy a
slot into/out of these conversion methods while handlers migrate in bounded
steps. Transient staging/probe scratch is not actor state.

Native world positions use `u16`. Screen projection and other observable byte
wraps still follow the source; represent that distinction with explicit types
and movement helpers instead of instruction-level arithmetic in handlers.

Groundwalker is the pilot for behavior-owned actor types. Its native
`GroundWalker` owns a state enum, `remaining_pixels`, `pause_ticks`, and the
position and presentation fields it uses. Its world interface accepts only a
position for wall probes. The compatibility adapter decodes through `Actor`,
maps the named fields into the native type, then writes them back to the
original `Actor`; unrelated and stale bytes never enter the native behavior.
Unknown state values survive conversion and retain the existing initialization
fallback on update. Shared sprite bobbing operates on its sprite and
animation fields without requiring a generic actor.

This pilot preserves existing behavior: initialization changes only the
horizontal flip bit. The source `$5B3C6` overwrites all flip bits; reconciling
that inherited difference requires a separate behavior correction.

## Verification and migration

Follow the source briefs, bounded tickets, ownership and verification gates in
`PORTING.md`. Each migration keeps the ROM-derived byte assertions and real
callers/consumers working. Add tests for coordinate conversion, signed movement,
flags/directions, reserved-value preservation, and relevant branch boundaries.
Sensitivity checks must reject behavior changes. Run the full core suite and
the frozen replay gate at the documented package boundary.

New logic follows this guide immediately. Migrate existing byte-oriented
handlers as bounded tasks; this is tracked implementation work, not a claim
that the existing engine is already fully typed. Preserve renderer/asset seams
and the public replay facade throughout.
