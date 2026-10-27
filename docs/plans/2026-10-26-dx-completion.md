# Complete SML2DX compatibility

Updated 2026-10-27 (steps 2-6). Use the exact supported ROM and source locations in the
[DX audit](../sml2dx-audit.md). Keep implementation status here and source facts
there; do not create more tracking documents.

## Work order

1. **Source audit — complete.** ROM identities, palette/movement contracts,
   content differences and importer requirements are documented in the audit.
   Two opt-in ROM tests reproduce the content and character-table inventories.
2. **RGB comparison tooling — complete (`b69e750`).** Extend the existing host/frame bridge and
   `tools/compare_tas.py` for DX RGB, explicit mode/character and matching state.
   Preserve grayscale callers. Test known colors, truncation and state alignment;
   do not mask wrong physics/fades with arbitrary frame offsets.
3. **Regional palette timing — complete (`451b824`).** All eight selectors landed in `291981a`.
   Verify staged coordinates, frozen/special-call frames, level-reset retention,
   direct OBJ writes and selector/arena ordering. Test thresholds and actual
   rendered colors for both characters; drawing must not advance state.
4. **Fades and scene effects — complete except VBlank job-queue ordering
   (`5a10a26`, `47169a2`, `f5cc37f`).** Implement palette job ordering, white blanking,
   castle black fade, map brighten/return and DX's 36-wait exit (vanilla 48).
   Add credits 7->8/8->9/9->10 palette/attribute changes and castle coin flash.
   Split by primitive then scene. Assert exact bytes/cadence/restoration and
   repeated-render independence; retain DMG-only paths without CGB interpolation.
5. **Gameplay selection and physics — complete except diagnostics and a
   save/restart retention test (`ff04d84`, `beecfec`, `6137fc8`).** Thread native mode/character through
   shared simulation and diagnostics. Apply gameplay/content changes at safe
   level-entry/reload boundaries; retain mode on save/restart and rendering
   fallback. Extract narrow H256/V76 curves, validate configuration and bump the
   persisted schema/cache identity. Port friction and displacement separately;
   test Luigi's exact differences and unchanged Mario/swim/moon paths.
6. **Geometry and object differences — implemented (`875e815`, `6467e18`);
   per-level block/coin, pipe and checkpoint checks on DX layouts move to step 7.** Give simulation, actor probes, rendering,
   spawns and room/arena reloads the same DX content. Start with each of the six
   collision-changing levels, then remaining layout/metatile changes. Test live
   block/coin mutations, pipes, checkpoints and reloads before removing render
   substitution. Include spawn 0B and separate parked-priority/flash-mask fixes.
7. **Full-game verification.** Cover both characters/forms, all 32 levels and
   pipe rooms, 15 map/minigame screens, bosses 0..8 and fixed scenes through credits.
   Check Wario 6->7->8 inheritance, water/decor animation and enlarged views.
   Finish both character routes from new file through six coins and credits,
   including save/reload. Keep missing independent/hardware evidence explicit.

## Execution and acceptance

Follow [PORTING.md](../../PORTING.md): one bounded ticket at a time, named files
and source range, meaningful source-derived tests, focused verified commit.
Existing vanilla route milestones remain separate. Use existing code/tests for
case inventories; update this checklist and the audit instead of duplicating them.

Run focused tests plus `cargo test -p sml2-core` for behavior work. At each work
package, run affected crate suites, then the frozen sweep once with pipefail:

```sh
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

Importer/integration closeout also runs the opt-in ROM commands in the audit.
No new captures or trajectory scripts; no committed ROMs/extracted graphics.
A passing sweep proves regression, source-byte tests prove their branches, and
independent RGB comparisons prove only the pictures/states actually compared.
Update player-facing compatibility wording when the corresponding work passes.
