# Macro Zone scenery

Scope confirmed: extend scenery to Macro Zone, continuing clockwise from Tree Zone. The user has painted all four levels, the map, and the boss arena; all six snapshots are bundled and approved following the user's in-game verification for 0.3.0.

## Scope

- Cover Macro Zone 1–4, including pipe rooms and other areas visible during play.
- Cosmetic scenery only. Preserve terrain, collision, enemies, pipes, routes, and gameplay.
- Continue each level's existing visual language using its own tiles rather than redesigning it.
- Use the companion Scenery Editor. The user paints and approves composition; the assistant inspects coverage, integrates approved exports, and verifies in-game behavior.
- Include Macro Zone's map and boss arena surroundings, as requested after painting. Level rebuilding remains outside this work package.

## Work sequence

1. Paint Macro Zone 1 in the editor. Review the entire level, including interiors, in enlarged native and DX views. Completion: exposed edges are filled, repeating patterns join cleanly, actors and hazards remain readable, and the user approves the treatment.
2. Use the approved Macro Zone 1 treatment as the visual reference for Macro Zone 2–4. Paint and review each as a complete level, including its interior areas.
3. Integrate each approved export through the existing bundled-scenery registry. Follow [Scenery Editor](../scenery-editor.md) and [Approved scenery](../../assets/scenery/README.md) for export and verification procedures. Local drafts alone are not release assets.

## Acceptance

- All four main Macro Zone levels and their playable interior areas have been reviewed for scenery coverage.
- Approved scenery is bundled and visible in normal game play, not only editor playtests.
- Enlarged native and DX views have been visually checked for seams, abrupt edges, and gameplay readability.
- Gameplay is unchanged; no expanded-level recipes are introduced.
- Integration checks are executed and their actual results recorded when exports are integrated.

## Status

Visual review is complete, confirmed by the maintainer for 0.3.0. All six
targets are bundled. Further painting requires a fresh export, rebuild, and
visual approval; the existing approved scenes are not awaiting review.

## 0.3.0 integration verification

- `cargo test --workspace --locked --offline`: 2,289 passed, 24 ignored.
- `cargo test --locked --offline -p sml2-desktop --no-default-features --lib`:
  51 passed, 1 ignored.
- `cargo fmt --all -- --check`: passed.
- `bash captures/sweep.sh`: exited successfully with `gate done`; the gate
  still reports replay mismatches, so this is not an emulator-exactness claim.
- Runtime level 20 ran for 120 headless frames with `unported=[]`; the
  bundled map/session also ran for 120 headless frames successfully.
- Strict all-target workspace Clippy did not pass: existing test lint debt
  spans core, extraction, and DX. This preparation does not resolve that debt.
