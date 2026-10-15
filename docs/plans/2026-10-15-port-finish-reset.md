# Port finish — policy reset execution plan

> **For agentic workers:** One checkbox = one PR or one focused session. Do not skip Phase 0.

**Goal:** Stop whole-repo handler thrash; finish mechanics via **one handler address + one test** per ticket, with tooling and module boundaries that enforce small context.

**Architecture:** Canonical policy in `PORTING.md`; `objects/` split; `tools/handler_task.py` produces task slices; strong model ports only from slices; queue notes stop growing.

**Tech stack:** Rust `sml2-core`, `sml2-extract` / `extracted/objects.json`, `../sml2decomp`, Python 3.

## Global constraints

- No new captures (`PORTING.md`).
- Verification: unit test bytes + sweep once per work package.
- Sibling ROM: `../sml2decomp/baserom.gb`.

---

## Phase 0 — Policy land (you are here)

- [x] Add `PORTING.md` as canonical workflow.
- [x] Slim `AGENTS.md` to point at `PORTING.md` only.
- [x] Bump `PORT_QUEUE.json` → `workflow_rules.set` = 2026-10-15, `policy` = `PORTING.md`.
- [x] **Human:** Hermes prompts in `hermes/`; Cursor rules in `.cursor/rules/`; `tools/dispatch_prompt.py`.

---

## Phase 1 — Tooling (no handler logic yet)

**Files:** `tools/handler_task.py`, `tools/handler_coverage.py`, `tasks/handlers/.gitkeep`, `handler_coverage.json` (generated).

### Task 1.1 — `handler_task.py`

- [ ] Read `extracted/objects.json` anim → handler CPU address (mode A/B; level &gt; 10 uses B per `sprite_map_index`).
- [ ] Map CPU → ROM offset (bank 16: `rom = cpu + 0x54000 - 0x4000`).
- [ ] Locate label / line range in `../sml2decomp/main.asm` (search for `Unknown*` / address comment or disasm line).
- [ ] Emit `tasks/handlers/0x{addr:05X}.md`: handler address, anims that reference it, ASM excerpt (~80 lines), list of `call`/`jp` targets, pointers to existing Rust helpers in `objects/helpers.rs` (grep names).
- [ ] CLI: `python3 tools/handler_task.py --addr 0x58201` and `--anim 9 --mode a`.

**Test:** Run script for a **ported** handler (e.g. `0x58201`); diff excerpt against known `handler_walker` source comment.

### Task 1.2 — `handler_coverage.py`

- [ ] Union handler addresses from all 32 level manifests (same method as queue audit).
- [ ] Compare to `dispatch.rs` match arms + `note_unported` set.
- [ ] Write `handler_coverage.json`: `{ "ported": [...], "unported": [...], "by_level": {...} }`.
- [ ] Exit non-zero if a spawn references an address missing from both lists.

**Test:** `python3 tools/handler_coverage.py` → 97 total, ~11 ported (numbers may drift slightly; fix script not audit).

---

## Phase 2 — Split `objects.rs` (behavior-neutral)

**Rule:** `cargo test -p sml2-core` must stay green after each sub-task.

### Task 2.1 — Scaffold modules

- [ ] Create `objects/mod.rs`, `staging.rs`, `dispatch.rs`, `helpers.rs`, `handlers/mod.rs`.
- [ ] Move `Staging` + offset constants → `staging.rs`.
- [ ] Move shared probe/helper fns → `helpers.rs`.
- [ ] Leave `ObjectEngine` + slot loop in `mod.rs`.
- [ ] `lib.rs`: `pub mod objects` unchanged path.

### Task 2.2 — Move ported handlers (batch by family)

- [ ] Walkers / fall / stun → `handlers/walkers.rs`
- [ ] Items / pickup / pose → `handlers/items.rs`
- [ ] Boss path → `handlers/boss.rs`
- [ ] `dispatch.rs`: single `match` calling into handlers.

### Task 2.3 — Delete old monolith

- [ ] Remove `objects.rs` file after `mod.rs` re-exports.
- [ ] Confirm `rg 'objects\.rs'` only hits docs/history.

---

## Phase 3 — Handler factory (the long tail)

Repeat until `handler_coverage.json` shows `unported: []`.

### Per-handler loop (one PR each preferred)

- [ ] `python3 tools/handler_task.py --addr 0xXXXXX`
- [ ] Porter prompt: *only* `tasks/handlers/0xXXXXX.md` + target `handlers/*.rs` + `helpers.rs` + `staging.rs` + test template.
- [ ] Add `#[test]` in `objects/tests/` or inline module.
- [ ] `cargo test -p sml2-core <test_name>`
- [ ] Revert-one-line check for new branches.
- [ ] Update `handler_coverage.json` via script.

**Order:** L26, L27, L30 (0 new handlers) smoke → then levels from greedy cover (L25, L5, L8, …).

**Escalation:** After 2 failed compile/fix cycles on same handler, escalate with **only** that handler’s files + errors — no repo-wide read.

---

## Phase 4 — Close non-handler pending queue items

One item = one plan subsection; same PORTING gates.

| ID | Suggested approach |
|----|-------------------|
| `moon_physics_remaining_arms` | `player_sim.rs` slices + tests; read `home.asm` / `tasks/handlers/` only |
| `map_zone_screen_loaders` | `map_sim.rs` / `map.rs`; unit tests |
| `camera_pan_timer` | `camera.rs`; unit tests |
| `boss_arena_phase_cascade` | `objects/handlers/boss.rs` + `scene.rs`; unit tests, no new capture |
| `boss_clear_ending_handoff` | `scene.rs` / `player_sim` handoff; unit tests |
| `boss_anim_4_5_chain` | Single handler ticket + poke only if ASM ambiguous |

- [ ] Each item: move to `PORT_QUEUE.done.json` when `status: done` (keep `PORT_QUEUE.json` pending-only).

---

## Phase 5 — Integration gate

- [ ] `cargo test -p sml2-core`
- [ ] `bash captures/sweep.sh 2>&1 | tail -n 25`
- [ ] `python3 tools/handler_coverage.py` (all spawned handlers ported)

---

## Orchestration

See **[`ORCHESTRATION.md`](../../ORCHESTRATION.md)** (supervisor loop, porter prompts, accept checklist, Hermes wiring).

---

## Out of scope (until mechanics milestone done)

Sound, title/file select, menus, texture packs, frame interpolation implementation.
