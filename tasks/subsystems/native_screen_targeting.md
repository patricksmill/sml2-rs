# Native screen targeting geometry

One shared geometry subsystem needed by Wario23: COMPLETE bank2 briefs
[`$B27C`](../handlers/0x0B27C.md) (includes `$B296`) and
[`$B2DB`](../handlers/0x0B2DB.md).
Read PORTING/native guide, named briefs and actor.rs. Touch ONLY actor.rs for
small native APIs/registration, optional bounded actor_screen.rs sibling and
its tests. Do not change existing handler behavior yet.

Implement `ScreenPosition::horizontal_distance_to(target_x: u8) -> u8` using
source signed wrapped difference's magnitude, preserving 128 at signed minimum.
This is not unsigned coordinate absolute difference: 0 vs255 distance is1.
Implement `Actor::face_toward_screen(target_x: u8) -> Facing`: source compares
unsigned target to actor.screen.x; equality faces right. Set full sprite flip
value to horizontal32 for right or zero for left, clearing other old flip bits
as source `$B27C` does, return typed Facing. Source helper also selects state1
left/state2 right and world signed facing FF/1; keep those caller-specific state
and compatibility world writes explicit at future Wario23 adapter. Native
geometry must not encode those byte values or write unrelated actor fields.

Tests literal source edges: equal, 0/255, 127/128, both sides of wrap and signed
minimum; all positions roundtrip? direct independent ROM algorithm can be test
oracle but avoid implementation-mirroring tests. Whole byte preservation for
face operation and equality/old flip clearing. Focused/full core tests and
fail-and-restore sensitivity; ask root for mutation/full-suite window because
spawn helper is currently mutating own files. No commits/sweep/queue/inventory.
