# Systematic Rust port completion workflow

This is the completion roadmap for the original Super Mario Land 2 game.
`PORTING.md` remains the canonical policy for task size, verification, and
capture restrictions. `ORCHESTRATION.md` supplies dispatch templates. Use this
roadmap to decide what to port next and what evidence closes a milestone.

## 1. Define completion and track it honestly

The target is a native Rust game that starts at the title screen, manages save
files, runs every level and alternate route, completes every boss and minigame,
plays music and effects, reaches the ending, and can resume saved progress.
Extract original assets and tables locally from the user's ROM. Preserve the
renderer and asset seams in `DESIGN.md`.

Track three independent claims for every feature: implemented, behavior verified,
and integrated into normal play. A dispatch arm, a passing replay, or an empty
queue alone does not establish all three. A real ROM no-op counts as implemented
only when its dispatch and surrounding gates are verified.

Maintain a small feature inventory under `docs/` with one row per coherent
feature: stable ID, ROM references including bank, Rust entry point, dependencies,
verification test names, normal-play entry path, and remaining gaps. Keep the
current queue pending-only; promote discovered gaps into focused queue items as
their milestone becomes active. Keep completed evidence in the existing archive.
Do not mark a pending item done just because similar code or an old commit exists.

Use the ROM bytes and bank-aware disassembly as the authority. Historical notes,
queue descriptions, generated slices, and Rust comments are leads to confirm.
Byte-identical assembly establishes the reference build, not semantic completeness
of the Rust port.

## 2. Workbench contracts

These tooling prerequisites are implemented; commands, limits, and exit statuses
are documented in [`tools/README.md`](../../tools/README.md). Keep these contracts
when extending the tools:

1. Make `tools/handler_coverage.py` read `objects/dispatch.rs`, and update the
   prompt generator and subsystem file lists to the actual module tree. Current
   handler families live directly under `objects/`; there is no `handlers/`
   directory. Use the existing layout; create a focused sibling module when a
   family file is already too large for another handler.
2. Reconcile the coverage tool's mode selection against ASM and
   `ObjectSim::configure`. The script previously used `level_id > 10`; Rust uses
   `level_id >= 0x0A`. Pin the boundary at level IDs 9, 10, and 11 after confirming
   the source. Derive bank/CPU/ROM addresses from the bank mapping; store both
   forms in task briefs. Treat `$16` as hexadecimal bank 22, not decimal bank 16.
3. Make handler coverage distinguish level-table and boss-table dispatch,
   implemented bodies and verified no-ops. Report the intersection of implemented
   addresses with reachable addresses; a raw total of match arms is not a
   completion percentage. Fail on invalid anim indices, missing profiles, missing
   assets, or unreadable Rust sources rather than silently omitting entries.
4. Extend task slicing to include complete basic blocks, all state-table targets,
   tail jumps, and called helpers required by the task. A 90-line excerpt is a
   starting hint, not a complete routine. Resolve bank-switch trampolines and
   `push`/`ret` control flow explicitly. If a dependency exceeds the task budget,
   issue a prerequisite helper ticket; do not guess the missing behavior.
5. Make the regression gate fail on build failure, runtime failure, missing
   summaries, unexpected notes, or nonzero mismatches. Compare the documented
   `traj4_ff0` exception against its established baseline, including its notes;
   do not exempt arbitrary changes to that capture. The previous shell script
   masked build errors and ended successfully after printing regressions. The
   replacement must preserve failures, including when its output is piped through
   `tail` by a caller using `set -o pipefail`.

Validate these repairs with small deterministic fixtures: mode boundaries,
banked address conversion, a known handler and no-op, an invalid profile, a boss
entry, and an intentionally failed gate. Scripts must provide trustworthy inputs
and measurements before porters use them.

## 3. Work in dependency order

| Milestone | Work order | Exit evidence |
|---|---|---|
| A. Workbench | Repair tools above; record current core test and frozen replay baseline | Accurate coverage, usable task briefs, and a gate that rejects failures |
| B. Shared gameplay | Audit pending camera, moon-physics, scene, map-loader, and transition items; close dependencies that block ordinary gameplay | Unit tests cover each producer and consumer; normal play derives its own inputs |
| C. Objects and bosses | Shared helpers first, then reachable level handlers, dynamic spawn chains, all boss definitions, entry/clear/ending transitions | Reachable behavior closure has no unimplemented paths; verified dispatch and normal entry/exit for every fight |
| D. Complete game flow | Boot/title, file select and save management, overworld presentation, remaining screens, pause/game over, minigames, final castle and ending/credits | Every game-state transition and user action has an implemented route and acceptance scenario |
| E. Presentation and audio | Finish missing visual behavior, then sound driver sequencing and audio output | Native-size visual checks and deterministic audio-driver checks pass through normal game flow |
| F. Full integration | Run the feature inventory and level/route matrix; reconcile every remaining diagnostic | All original-game features are verified and reachable through normal play |

Audit each pending queue item against current code before implementing it. Reuse
existing tests and close the actual remaining gap. The queue contains an
aggregate handler item; subdivide it into address-sized tasks without treating
the aggregate title's old count as live truth.

Within a milestone, choose ready tasks in this order: prerequisite shared helper,
game-flow blocker, handler shared by many levels, then local behavior. Among
otherwise equal handler tasks, prefer the one that brings a level closest to
complete coverage; break ties by number of levels using it, then ROM address.
Recompute the ordering after each package rather than freezing today's level
order. Keep packages to at most five related tickets, each independently accepted.

Do not wait for every level handler before fixing a camera or transition defect
that blocks the same package. Finish a shared helper and its tests first, then
reuse it across handlers. Keep unrelated fixes in separate tickets.

Milestones D and E are deliberately later than the active mechanics milestone.
Texture packs, interpolation, DX compatibility, and other enhancements remain
separate work after original-game completion.

## 4. Inventory reachable behavior, including generated behavior

Start with all game-state dispatch entries, both level anim tables, boss
dispatch/definition tables, all level spawn streams, scene opcodes, map node
actions and loaders, and minigame/title/menu routines. Classify code, data, and
hardware effects; do not count every ROM byte as a behavior to translate.

Build the initial handler list from spawn data, then expand it to a fixed point:
add anim changes, child spawns, defeated/carried/kicked variants, pickups,
projectiles, boss phases, and scene-created objects until no new target appears.
Repeat this expansion when implementing a routine discovers another target.
Account for every table entry as implemented, verified no-op, unreachable with
source evidence, or pending. Spawn-only coverage is a useful lower bound.

For each gameplay system, inspect both its writers and readers. Record who owns
the state, its update order, reset behavior, and whether a timer advances during
travel, death, scenes, fades, and reloads. Match the source's actual logic-frame
and interrupt timing; do not infer timing from a uniform once-per-frame rule.

Normal play must compute its own camera flags, scene latches, timers, and object
results. Replay initialization may seed a documented starting snapshot, but
per-frame console values must not substitute for the system under verification.
Test the play and replay boundary with the same locally derived state.

## 5. Repeat this bounded task loop

1. **Select:** choose one handler address, one shared helper, one game-state
   transition, or one bounded subsystem behavior. State its dependencies and
   acceptance condition. Split a large queue item into child tasks before work.
2. **Prepare:** create a brief containing exact bank/CPU/ROM references, complete
   required ASM slices, state offsets, table bytes, frame gates, call targets,
   named Rust files, and test cases. Keep porter context around 15k tokens.
3. **Translate:** preserve wrapping arithmetic, signed comparisons, carry/borrow
   decisions, byte layout, update order, and side effects. Express the routine
   through existing Rust helpers and types. Preserve byte-addressable staging
   where it is part of the engine contract. Keep CPU emulation and bulk ASM
   transpilation out of the runtime architecture.
4. **Verify locally:** construct state in memory and assert independently derived
   output bytes and writes. Exercise all meaningful states and branches, including
   initialization, boundaries, timer/phase gates, and completion/removal. Add a
   dispatch test and a producer-to-consumer test where integration is required.
5. **Check sensitivity:** for newly covered branches, temporarily revert the
   relevant behavior and confirm the test fails; restore the implementation.
   Never leave the deliberate failure in the final diff.
6. **Review:** compare the Rust against the slice, check named-file scope, and
   reject placeholders, swallowed unknowns, hardcoded capture values, missing
   state arms, and changes to unrelated game behavior. Run the full core suite.
7. **Integrate:** connect the feature to its normal caller and verify its return
   path. Refresh measured coverage. Close only the ticket whose acceptance
   condition is proved; archive concise evidence and retain unresolved child work.

After two failed attempts caused by an unresolved source/control-flow question,
stop retrying the same patch and isolate that question. Expand the exact slice
or escalate that bounded task per `PORTING.md`. A compile error with a clear fix
does not justify exploring the whole repository.

Keep each accepted change reviewable. Save progress with a focused commit when
committing is part of the session's instructions; preserve unrelated local work.
If multiple workers are used, give each a distinct implementation file and keep
dispatch registration/integration under one owner. Otherwise run the same loop
sequentially; multiple agents are optional.

## 6. Apply verification at the right boundary

| Boundary | Required scenarios |
|---|---|
| Routine | Each reachable state; thresholds on both sides; wrapping/borrow; odd/even phase; timer zero and expiry |
| System seam | Writer followed by actual reader; entry/exit/reset; carried state across death, travel, fades, reload, and save where relevant |
| Level/route | Source-derived layout, spawn and property data; entry, exit, secret route, required mechanic, and boss if present |
| Screen/game state | Enter, input, cancel/confirm, completion, return destination, and persistent effects |
| Persistence | Source-derived save representation/checksum; new/load/update/delete; invalid data and interrupted writes through atomic host-file replacement |
| Audio | Command decoding, channel ownership, note/envelope timing, music/SFX priority, silence/reset; deterministic register/event traces, then device output |
| Rendering | Native viewport pixels, camera and scroll, OAM order, palettes, tile animation, HUD/window, scene/map transitions |

Use hand-derived ROM/table expectations rather than recomputing expected values
with the function being tested. Prefer short deterministic in-memory cases over
long replay routes for new logic. Preserve the frozen capture corpus; follow
`PORTING.md` for the narrowly allowed diagnostic memory poke. When a comparison
needs new external evidence, record the missing evidence as a blocker rather
than inventing a new capture workflow.

Run the targeted tests while editing and `cargo test -p sml2-core` before accepting
each ticket. At a package boundary, run the release build and frozen sweep once;
check the actual output and process failures. For extractor, runtime, or audio
changes, also run the relevant crate checks and focused tests for those layers.
Replay gates establish sampled state parity; use the existing documented visual
comparison procedure for pixel claims. Do not claim audio accuracy from silence
or from merely forwarding sound-request bytes.

At each milestone boundary, review a matrix covering all 32 level manifests and
all source-defined entrances, routes, boss definitions, minigames, and game
states. Deterministic direct placement helps isolate mechanics; normal boot and
transition scenarios separately prove that those mechanics are connected.

## 7. Require this completion handoff

For every package, report the ticket IDs and ROM references, implementation
entry points, test commands/results, coverage before/after, normal-play route,
and unresolved dependencies. Keep the handoff short and link evidence rather
than adding session narratives to queue items.

Declare the original-game port complete only after the inventory has no pending
reachable behavior, every matrix scenario passes, no unexpected unported
diagnostic is emitted, the frozen regression baseline holds, presentation and
audio evidence are recorded, and a fresh local asset extraction/build can run
the game from boot through final completion and save/reload. Retain the inventory
and tests as regression protection for later enhancements.
