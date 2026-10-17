# Shared native metasprite append prerequisite

Source COMPLETE tasks/handlers/0x05297.md. Scope player.rs and new sibling
player_mapping_tests.rs only. Read PORTING.md, native guide, this brief,
source packet, object.rs MappingEntry/player_mappings definitions and actor.rs
ScreenPosition (bounded ranges only). No whole engine/runtime/main.asm.

Extract the existing draw_cells geometry into a native shared append API:
append frame cells to an existing Vec<Cell>, explicit ScreenPosition base and
native CellStyle (palette override and priority), preserving original attr bits,
signed offset wrap, ordering, empty mappings and explicit missing-frame result.
Callers own legal sprite batch capacity; do not simulate OAM pointer/registers.
Use append for concatenated poses; keep draw_cells public signature and delegate
its existing world-to-screen geometry and player style to the shared method.
No raw HRAM state, bank switching, CPU registers or slot operations in logic.
Reuse existing extracted player_mappings, which include ending IDs E5..EF;
root verifies their exact ROM records independently. No asset format change.

Tests independently assert complete Cell bytes/order, pre-existing appended
prefix, every attr byte with all four styles, signed wrapping, missing/empty
frames, real legacy draw_cells consumer and native screen input. Use standalone
ObjectTables fixture from existing extracted objects, preserving test isolation.
Keep production player.rs under400 lines. Root coordinates mutation/fullsuite
lane: focused tests allowed now; request before mutations/fullsuite. Sensitivity
must kill priority/palette/wrap/append or facade delegation changes. No commit,
sweep, inventory, queue or captures changes; root handles acceptance.
