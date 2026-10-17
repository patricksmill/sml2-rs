# Native ending scene load prerequisite

One shared scene-load helper ROM `$29DD` (home, entered bank1A), COMPLETE
`tasks/handlers/0x029DD.md`. This is ending-sequence state4's complete helper;
full state19 controller remains INCOMPLETE until other prerequisites close.

Source assets: bank11(hex17), CPU5800, length1800hex => ROM45800..46FFF
native tile data (384 2bpp tiles). Background tilemap bank1A CPU548E length400
=> ROM6948E..6988D (32x32 IDs). No decompression. Source initializes scroll0/0,
palettes E1/D2/39, clears all40 OAM entries, signed BG tile addressing with
8x8 sprites/display BG+OBJ enabled/window disabled (LCDC C3), fade256,
credits sprite Y44 and increments ending_state. Hardware wait/bank plumbing
is eliminated; retain logical rendering state and asset IDs/data.

Read PORTING/native guide, named sourcebrief, bounded extractor main.rs module
list + baserom load/output integration points, existing core asset serialization
patterns/video tile helpers/ending_entry palette type. Touch ONLY new bounded
core ending_scene.rs (+ optional sibling tests), lib.rs export; new extractor
ending.rs (+local tests), extractor main.rs module registration + one emit call;
new generated extracted/ending assets (normally ignored; do not force-stage ROM
data). No runtime/sequence controller changes yet. Runtime must never read ROM.

Provide validated native EndingAssets and logical scene initialization with
source exact sizes/palettes/mode/reset and a clean rendering seam. Emit extracted
ending assets via existing extractor run; keep tiles data-based for texture packs.
Tests source literal offsets/lengths, malformed/truncated assets, dirty scene
reset and preserved unrelated state, actual extracted bytes independently checked
against ROM. Run core/extractor tests, fresh cargo run -p sml2-extract locally,
confirm existing assets unchanged, core/runtime gates and mutation sensitivity.
Do not claim pixel fidelity or sequence completion yet; loaded scene renderer/
sequence caller next. Ask root before mutation/fullsuite; test lane currentlyfree.
No commit/sweep/queue/inventory editing; root acceptance handles those.
