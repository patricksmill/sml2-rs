# Porting policy (canonical)

**Effective 2026-10-15.** This file is the only agent-facing workflow spec. Everything else is reference or history.

| Doc | Role |
|-----|------|
| **PORTING.md** (this file) | How to work — one unit of work, verification, models, forbidden moves |
| **DESIGN.md** | Architecture and long-term constraints (texture packs, 60 Hz sim, seams) |
| **PORT_QUEUE.json** | **Pending work only** (7 items). Done history: `PORT_QUEUE.done.json` (do not load) |
| **notes/** | Retired (see `notes/README.md`) — do not add files here |
| **captures/** | Frozen replay regression only — read-only |

Sibling ROM truth: `../sml2decomp/` + `baserom.gb` under that tree (`SML2_DECOMP_ROOT`).

Full-game sequencing and milestone acceptance:
[`Systematic Rust port completion workflow`](docs/plans/2026-10-15-rust-completion-workflow.md).
The seven-item mechanics queue is one milestone within that roadmap. The repaired tools and their exit-status contracts are documented in
[`tools/README.md`](tools/README.md). Handler counts measure declared dispatch,
not behavior verification or dynamic-spawn closure.

---

## What “done” means

1. Logic matches the disassembly slice (branch structure, slot/staging offsets, frame gates).
2. **`cargo test -p sml2-core`** includes at least one test that constructs state in memory, runs the code path, and asserts **bytes** (slot, staging, or documented WRAM fields).
3. New tests that cover previously unreachable code must **fail** if you revert the line under test.
4. After a **work package** (not every commit): `bash captures/sweep.sh 2>&1 | tail -n 25` once.

Enable `set -o pipefail` before piping gate output through `tail`; the pipeline
must preserve the gate's nonzero exit status.

**Forbidden:** new `captures/*.json`, trajectory capture scripts, steering Mario in PyBoy to “get a capture”, VBlank lag-frame debugging, reading whole `PORT_QUEUE.json` or multi-MB JSON into context.

**Allowed once:** &lt;10-frame PyBoy memory poke when ASM + ROM cannot settle an instruction.

---

## Unit of work (mandatory)

Never assign “port the object engine” or “fix DeepSeek’s mess in `objects.rs`”.

**One ticket = one of:**

| Ticket type | Deliverable | Max context |
|-------------|-------------|-------------|
| **Handler** | One bank-$16 (or boss) handler ROM address + `#[test]` | Slice in `tasks/handlers/*.md` + `objects/` submodule only |
| **Subsystem** | One queue item (e.g. `camera_pan_timer`) | Files listed in that item’s plan section |
| **Tooling** | Script that reduces ASM hunt tokens | No behavior change |

### Handler ticket checklist

1. Run `python3 tools/handler_task.py --addr 0x58201` (or `--anim 9` mode A) → writes `tasks/handlers/0x58201.md`. Require exit 0 and `Slice status: COMPLETE`; exit 2 requires source/dependency work. Bank `$16` is decimal 22.
2. If no test exists: add `objects::tests::handler_0x58201_*` in the same PR as the handler (or `objects/tests/`).
3. Implement in a named sibling module under `crates/sml2-core/src/objects/`; wire **one line** in `objects/dispatch.rs`.
4. `cargo test -p sml2-core handler_0x58201` (or the test name you chose).
5. Mark progress in `handler_coverage.json` (generated/manifest — see plan), not in a 2 KB `PORT_QUEUE` note.

---

## Code layout (objects)

Current module layout (keep the public facade stable):

```
crates/sml2-core/src/objects/
  mod.rs          # ObjectSim, slot loop, public API
  staging.rs      # $FFD0 offsets, copy in/out
  dispatch.rs     # match on handler ROM address → fn
  helpers.rs      # $5Bxxx / probes already shared
  walkers.rs      # existing handler family
  items.rs        # existing item/pose family
  boss.rs         # existing boss family
  interaction.rs  # contact and projectile interactions
  shell.rs        # carried shell behavior
  stream.rs       # spawn stream
  tests.rs        # existing cross-module tests; new handler tests stay local
```

**Rule:** No new handler bodies in a file over ~400 lines. Split before adding.

---

## Models and roles (orchestration)

Cheap / flash models are **not** allowed to own logic ports.

| Role | Task | Model tier |
|------|------|------------|
| **Slicer** | Run `handler_task.py`, queue JSON, `cargo test` parse | Script or smallest model |
| **Porter** | One handler + one test from `tasks/handlers/*.md` | Strong coding model (e.g. DeepSeek V4 **Pro**, Gemini Argon, Sonnet-class) |
| **Escalation** | `push hl`/`ret` tricks, cross-bank trampolines, borrow hell after **2** failed fix attempts | Opus / Sonnet, **same file only** |
| **Forbidden** | “Read all of `objects.rs` and port anims 13–14” | Any model |

**Context budget per porter turn:** aim &lt; 15k tokens input — slice markdown + one handler file + `staging.rs` + `helpers.rs` + test error output. Do not attach `main.asm`, full `PORT_QUEUE.json`, or `DESIGN.md`.

**No RGBDS→Rust bulk transpiler** as a gate. Deterministic tools may emit **stubs, tables, and ASM appendices** only; the porter writes Rust calling existing helpers.

---

## Remaining work (scope)

Queue milestone **mechanics_all_levels**. Pending items (7): `remaining_handler_classes` (bulk), `moon_physics_remaining_arms`, `map_zone_screen_loaders`, `camera_pan_timer`, `boss_arena_phase_cascade`, `boss_clear_ending_handoff`, `boss_anim_4_5_chain`.

Handler audit: run `python3 tools/handler_coverage.py` → `handler_coverage.json`. Orchestration (supervisor/worker prompts): [`ORCHESTRATION.md`](ORCHESTRATION.md).

---

## Rendering claims

Replay sweep does **not** validate pixels. For visual bugs use `DESIGN.md` console vs `sml2-runtime --out` diff procedure (keep cameras in the assertion).

---

## Editing `PORT_QUEUE.json`

Python `json.load` before write. Query with:

```bash
python3 -c 'import json;print([e["id"] for e in json.load(open("PORT_QUEUE.json"))["queue"]])'
```

When closing an item: `python3 tools/queue_done.py <id>` (moves to `PORT_QUEUE.done.json`). **Do not add `note` fields.**
