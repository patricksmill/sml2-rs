# Correct final Wario against generated source

One handler correction: ROM AD93 bank2. `tasks/handlers/0x0AD93.md` has been
regenerated to restore exact COMPLETE instructions (old handwritten summary
omitted persistent state and mislabeled powerup2 as Fire Mario).

Read PORTING/native guide, exact brief, production form3.rs/form3_adapter.rs,
Actor shared API sections, bounded relevant form3 tests/integration and objects
scratch clear. Touch ONLY those form3 files/tests, objects/mod.rs for a native
persistent saved-power field/default, scratch.rs clear/reset tests, and source
brief if documenting assumptions. Do not change runtime/scene gates yet.

Corrections: AF66 stores player powerup.wrapping_sub(2) in persistent AFD1 only
when walking enters jump. Jump states6/7 select high27 if SAVED byte0, low15
otherwise, including after player power changes mid-jump. Add native world
field/context/effect and page-clear/reset coverage (AFD1 belongs to AF page).
AF4E uses existing screen.horizontal_distance_to (signed wrapped magnitude),
not abs_diff. B339/B35A behavior already has source-verified Actor methods:
replace duplicated animation math with animate_four_step_bob / animate_ending_bob,
propagate typed errors, preserve equality-only caps and flip-derived reset lookup.
Use actor.face_toward_screen, sprite_flip.toggle_horizontal instead of duplicate
flag code. At ending removal clear Actor in domain; adapter converts through
Actor methods, no raw zero-array clear. Child sprite snapshot converts Actor,
not B_SPRITE indexing. Preserve all exact other source behavior.

Scene event note: existing scene_phase_set is NOT missing scene reset; real
object_events.rs consumer writes both phase and scene0. Read that bounded method
and test actual final phase3 consumer instead of inventing another flag. Current
Effects.scene_state may be removed as redundant with tests updated to assert
real combined event, or documented explicitly; retain normal source semantics.

Add regressions first and show existing code fails: power changed across jump
frames, literal saved-byte values0/1/2/3 powerups, wrapped distance24 vs255,
animation malformed phase equality/flip-index quirk, dirty-slot complete final
clear/count preservation, actual event->Camera scene0. Retain independent ROM
expectations; correct false old assertions explicitly. Full core/runtime,
meaningful sensitivity restore; ask root before mutations/fullsuite. No commit,
sweep/queue/inventory. Keep modules bounded (<400lines).
