# Porting policy (canonical)

**Effective 2026-10-15.** This file is the only agent-facing workflow spec. Everything else is reference or history.

| Doc | Role |
|-----|------|
| **PORTING.md** (this file) | How to work — one unit of work, verification, models, forbidden moves |
| **DESIGN.md** | Architecture and long-term constraints (texture packs, 60 Hz sim, seams) |
| **PORT_QUEUE.json** | **Pending work only** (query the live count). Done history: `PORT_QUEUE.done.json` (do not load) |
| **notes/** | Retired (see `notes/README.md`) — do not add files here |
| **captures/** | Frozen replay regression only — read-only |

Sibling ROM truth: `../sml2decomp/` + `baserom.gb` under that tree (`SML2_DECOMP_ROOT`).

Native implementation rules: [Idiomatic Rust porting guide](docs/idiomatic-rust-porting.md).
Handlers use typed actors and reusable domain behavior. Raw slot offsets and
register plumbing stay out of game logic; slot conversion belongs to
`Actor::from_raw_slot` / `Actor::to_raw_slot`. Existing byte-oriented handlers
must migrate in bounded, behavior-preserving steps. ROM byte assertions remain
the verification boundary, not the implementation style.

Full-game evidence and sequencing: [route inventory](docs/route-matrix.md).
Shared subsystem dependencies precede handlers; closure, played routes and
visual/audio/final integration are separate acceptance steps. Tool commands and
exit-status contracts are in [tools/README.md](tools/README.md).
Handler counts measure dispatch, not behavior or dynamic-spawn closure.

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
2. If no test exists, add ROM-derived tests in the native category's `tests/` directory in the same PR as the behavior.
3. Implement in a descriptive file under `objects/enemies/`, `platforms/`, `items/`, or `projectiles/`, with slot conversion under `objects/compat/` (bosses use their family directory); wire **one line** in `objects/dispatch.rs`.
4. `cargo test -p sml2-core handler_0x58201` (or the test name you chose).
5. Mark progress in `handler_coverage.json` (generated/manifest — see plan), not in a 2 KB `PORT_QUEUE` note.

---

## Code layout (objects)

Current module layout (keep the public facade stable):

```
crates/sml2-core/src/objects/
  mod.rs                 # ObjectSim, slot loop, module registrations, public API
  dispatch.rs            # ROM address → compatibility method
  staging.rs             # slot compatibility data
  helpers.rs             # shared engine helpers
  interaction.rs         # contact and projectile interactions
  enemies/
    boo.rs               # one named native behavior per file
    cannon_ant.rs
    hopping.rs
    tests/hopping.rs
  platforms/             # named platform files and tests/
  items/                 # named collectible and marker files and tests/
  projectiles/           # named projectile files and tests/
  effects/               # defeat, removal and other transient behavior
  spawning/              # shared dynamic spawning
  compat/
    enemies/hopping.rs   # slot conversion and world input/effect mapping
    platforms/           # platform adapters
    items/               # item adapters and legacy dispatch
    projectiles/         # projectile adapters
    effects/             # effect adapters
  bosses/
    shared/              # damage, stun, flash, spawn and reward helpers
    circuit/             # behavior.rs, adapter.rs, motion.rs, tests/
    door/                # mouth, children, side and central doors, tests/
    patrol/              # behavior.rs, adapter.rs, child/, tests/
    sweep/               # behavior.rs, adapter.rs, child/, drop_child/, tests/
    tree/                # behavior.rs, adapter.rs, tests/
    mario_zone/          # parts, controller, reward, tests/
    wario/               # forms, children, phase transitions, tests/
  tests/                 # cross-module engine regressions
```

Use a descriptive enemy or behavior name as the filename. Keep each enemy's
native behavior in its own file; share reusable movement and world operations.
Place platforms, items and projectiles in their corresponding categories.
Use source-derived behavior names when the canonical character identity is not
established. ROM addresses belong in source references, dispatch and verification,
not in domain filenames or directory names.

Slot adapters mirror the domain filename under `compat/`. Tests belong in the
category's `tests/` directory, such as `enemies/tests/hopping.rs`; a separate
adapter suite can use `hopping_adapter.rs`. Boss families may keep subdirectories
for their parts and phases. Module registrations use explicit paths to preserve
existing private access and the dispatch facade. Ordinary object modules register
in `objects/mod.rs`; Wario and Mario Zone keep their own registrations.

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

## Remaining work

Query pending IDs from `PORT_QUEUE.json`; do not copy a stale backlog here.
Refresh `handler_coverage.py` for declared dispatch and `anim_closure.py` for
dynamic reachability. Follow [ORCHESTRATION.md](ORCHESTRATION.md) for dispatch.

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
