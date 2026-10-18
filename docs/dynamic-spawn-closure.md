# Dynamic spawn and anim closure

**Status: measured 2026-10-18, closed.** Tool: [`tools/anim_closure.py`](../tools/anim_closure.py)
(`python3 tools/anim_closure.py` exits 0; `--self-test` proves the checker is
sensitive).

This closes the *reachability* half of `mechanics_all_levels_acceptance`:
every anim value an object can receive at runtime dispatches to an implemented
handler or a verified ROM no-op. It does **not** assert the behavior of those
handlers — that stays with their unit tests and the route matrix.

## Why declared dispatch is not enough

`handler_coverage.json` measures the two declared anim tables: 126/126
level-table handlers and 28/28 boss-table handlers are native. That is a
dispatch inventory, not a closure, because objects also receive anim values
from:

- the level spawn streams (949 records across 32 manifests),
- the dynamic spawner `$5B36D` (8 call sites),
- the boss spawner `$B224` (8 writes in bank 2),
- the interaction/morph arms in bank 2 (`$85AA`, `$8611`, `$86A0`),
- the spawn-record rewrite at `$86F9`,
- handler-local anim rewrites,
- scene scripts through the `$2x` boss-arena arm.

An anim value at or past the table length indexes nothing:
`ObjectTables::profile_for` returns `None`, the dispatcher sees handler 0, and
`ObjectSim` records `note_unported(0)` — the slot silently does nothing.

## Method

Per level, a fixpoint over anim values:

1. **Roots** — every `anim_index` in the manifest's spawn records.
2. **Handlers** — each produced anim's `handler` from the level's table
   (`anim_profiles_a`, 96 entries, level IDs 0–9; `anim_profiles_b`, 112
   entries, IDs 10–31; boundary `level_id >= $0A`, pinned by
   `tools/handler_coverage.py`).
3. **Dynamic spawns** — a `$5B36D`/helper writer adds its constant when an
   owning handler is running. Ownership is resolved through `call` targets, so
   a spawn inside a helper is attributed to the helper's callers.
4. **Morphs** — handler-local rewrites apply to every anim that dispatches to
   the owning handler; interaction arms apply to anims whose profile
   `type_sets` can satisfy the arm's `$FFE2` gate.
5. **Check** — `anim < table_len` and `table[anim].handler` implemented or a
   verified no-op.

The tool asserts that the `$FFD5` write sites it finds in `main.asm` are exactly
the 12 declared below, so a new writer in the disassembly fails the check rather
than escaping the audit.

## Result

| producer | sites | anim values | landing |
|---|---:|---|---|
| level spawn records | 949 records | 1 … 111 | all in table, all native |
| `$5B36D` dynamic spawns | 8 calls | 14, 36, 38, 55, 69, 78, 82, 89 | all native |
| `$5B31D` money-bag helper | 1 | 26 | native |
| `$B224` boss spawns | 8 writes | 4, 5, 9, 13, 22, 25, 27, 28 | all boss handlers native |
| boss anim rewrites | 2 | 3, 4 | native |
| slot anim writers (`$FFD5`) | 12 | ±1, +2, 0, 16, 18, 1, 62, 3, 4 | see below |
| record rewrite `$86F9` | 1 | 18 | native |

Measured closure: **477 (level, anim) pairs, 106 distinct anim values, 0
out-of-range, 0 unported.** Level 25 (`Wario's Castle`, level ID 24, table B)
is the boundary case: it produces anims 96–111, the last entry of the 112-entry
table.

### The 12 `$FFD5` writers

| main.asm | kind | effect | gate |
|---|---|---|---|
| 3008 `$85DC` | interaction | anim − 1 | `$FFE2` bit1 set, bit0 set |
| 3032 `$85FC` | interaction | anim = 0 (defeat/clear) | several kill arms |
| 3057 `$8611` | interaction | anim + 2 | `$FFE2 == 6`, no key held |
| 3064 `$8633` | interaction | anim + 1 | `$FFE2` bit1 set, bit0 clear (or bit4 & !bit3) |
| 3125 `$8695` | interaction | anim = 16 | `$FFE2 == $89` |
| 3167 `$86DA` | interaction | anim = 16 | `$FFE2 == $85` |
| 3191 `$86E4`/`$86F9` | record + slot | anim = 18, record anim = 18, sprite 26 | `$FFE2` bit7 set, low 7 bits 0, anim `$11`, powerup ≠ 0 |
| 4047 `$9443` | boss-local | anim = 3 | boss code |
| 4675 `$98F8` | boss-local | anim = 4 | boss code |
| 27551 `$58380` | handler-local | anim = 1 | handler `$5834F` |
| 29719 `$5918B` | handler-local | anim − 1 | screen X ≥ `$A0`, then `$5B2BF` |
| 30053 `$593A5` | handler-local | anim = 62 | handler `$59376` |

The carry/kick helpers `$84C8`/`$841A` (called from the `$8611` arms) write only
`sMoveFlags` and run probes; they do not touch the anim byte. Defeat and the
`$7x`-type bounces reach the anim through the rows above, not through a separate
writer.

### `$86F9` (the record rewrite)

`main.asm:3190-3191` (inside the bank-2 `$86E4` arm) walks the slot's record
pointer (`$FFD6/$FFD7`, record+2) by three bytes and writes 18 — the record's
anim byte at record+5 (`ram.asm:867`) — then writes the same 18 to `$FFD5` and
sets sprite 26. Only anim 17 with a powerup and a bit-7 type takes this arm.

### Scene-created spawns

The `$2x` scene opcode arms the arena; the port's `$0EB0` branch
(`crates/sml2-runtime/src/main.rs`, `cam.scene_load_pending`) then calls
`CameraSim::begin_scene_load` and `ObjectSim::spawn_boss(def_index)`.
`boss_def_index` uses the ROM's per-level ladder (`$05→0 … $17→5`, else 6) and
`boss_def_portable` accepts definitions 1–8; a definition past the 9-entry
table reports `$8DFD` through the unported list. Scene opcodes without an
implementation are surfaced through `SceneVm::unported()` and fail the headless
smoke gate.

The doc comment above `lookup_boss_spawn_def` still says "nothing spawns a
boss"; that is stale relative to the runtime `$0EB0` branch and should be
corrected when the boss-arena route is verified.

**Open gap found by this audit: definition 0 has no arena entry.** Level 6
(level ID `$05`) carries scene `$0B` in its 48-byte scene map — the arena intro
script at `$4184` that `scene.rs`'s test already runs. `boss_def_index($05, 0)`
returns 0, but `boss_def_portable` accepts only 1–8, so the runtime takes
`take_blocked_scene_load` and reports `$0EB0` as unported. The gate is
deliberate (the same pattern used before definitions 1–5 were verified), and
every boss handler is native, so this is a route/verification ticket rather
than a handler port. Tracked as `boss_def0_arena_entry`.

## Limits

- The handler table is fully native, so closure reduces to a **range/bounds**
  proof: any in-range anim is implemented. The tool checks exactly that, plus
  the per-level anim sets. It does not verify that a handler behaves like the
  ROM.
- Interaction arms are over-approximated: `type_sets` is a static scan of the
  handler's `$FFE2` immediates, and the arms are applied whenever a reachable
  anim's type can satisfy the gate. Extra edges only add in-range, native
  anims.
- Helper-owned spawns are attributed to the helper's enclosing handler when the
  helper is itself a table entry (`$5B28C` for the remove-mode path). Anim 26
  may therefore be reachable from more handlers than the closure shows; it is
  in range in every level either way.
- Boss-table closure is asserted per spawn write, not per fight; arena entry is
  route evidence (next ticket).

## Reproduce

```bash
cargo build -p sml2-runtime
python3 tools/anim_closure.py            # exit 0, prints the per-level table
python3 tools/anim_closure.py --self-test  # both mutations must be detected
python3 tools/smoke_levels.py            # all 32 levels, idle + walk passes
```
