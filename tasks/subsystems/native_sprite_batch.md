# Native sprite batch finalization prerequisite

Source COMPLETE tasks/handlers/0x052E7.md; mapping append source5297 is already
native player::append_cells. Read PORTING/native guide and named packet, Cell
in player.rs, ending_scene.rs and ending_scene/tests.rs only. Scope new
sprite_batch.rs (+sprite_batch/tests.rs), lib.rs registration, ending_scene.rs
and its existing tests to consume the typed batch. No runtime/objects edits.

Provide SpriteBatch owning forty typed Cell records and previous used cell
count, not emulated RAM or hardware cursors. write_frame(&[Cell]) copies current
frame then applies source cleanup: if new_len<old_len zero whole cells in
new_len..old_len; equality case clears only NEXT cell Y, preserving its x/tile/
attr. If count grows do not clear beyond new cells. Set previous count=newlen.
An equal full40 write has no additional visible cell to clear: do not emulate
unrendered memory past OAM. Reject >40 explicitly without mutation; callers
retain the original visible sprite budget. Default allzero count0. Provide
read-only cells and a clear_cells method that zeros all forty WITHOUT resetting
previous used count (source29DD clears OAM, not sOAMWriteEnd).

EndingScene should own SpriteBatch instead of a public raw cell array. Its load
uses clear_cells, preserving source metadata count. Adapt dirty-scene test via
native write_frame before load; verify counter preservation for later cleanup.
No rawbyte backing, HRAM/register simulation, or raw offsets in domain logic.
Do not invent source52E7 ability to clear all stale metadata on equal/grow paths.

Tests independent complete flattened cell bytes for growth/shrink/equality,
zero/40 bounds, rejection untouched, multi-frame append_cells->write_frame
consumer with real ending mappings, scene load then next frame. Use private
module fixtures to dirty unused records for equality branch, not a new public
raw constructor just for tests. Sensitivity equality-clear omission, growth
clearing and wrong shrink bound must fail. Focused tests now but request root
before mutation/fullsuite lane (composition worker owns lane). No commit/sweep/
inventory/queue/capture changes. Root handles acceptance and metadata.
