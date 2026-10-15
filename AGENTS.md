# AGENTS.md — sml2-rs

Native Rust port of Super Mario Land 2, byte-verified against the disassembly in
`../sml2decomp/`. `DESIGN.md` = verified semantics; `captures/README.md` = per-capture
status; `PORT_QUEUE.json` = the backlog (the durable record).

## Workflow: read ASM → port → unit-test → replay once

**Set 2026-10-14. This supersedes the old "capture first, replay-verify" loop.**

1. **NO NEW CAPTURES.** Do not create `captures/*.json`, do not write or run
   `trajectory_capture*.py`, do not script inputs to steer Mario to an object, tile or
   trigger, and do not debug VBlank lag-frame pairing. The existing corpus is frozen and
   is used read-only.

2. **PORT DIRECTLY FROM THE DISASSEMBLY, THEN UNIT-TEST.**
   - Read the exact assembly slice in `../sml2decomp/` (`main.asm`, `home.asm`). Grep the
     routine's callers and readers before deciding polarity or sub-frame placement.
   - Verify data tables against `baserom.gb` bytes when the listing is ambiguous.
   - Port the logic to Rust, mirroring the disassembly's branch structure.
   - Verify with fast, self-contained `#[test]` unit tests that **construct the state/slot
     directly in memory, run the handler, and assert the resulting state bytes**. No
     emulator, no JSON, no fixture files.
   - A unit test written because nothing else reaches the path must be shown to FAIL:
     revert the line under test, watch it go red, restore it.

3. **HARDWARE AMBIGUITY ONLY → THROWAWAY POKE.** If an instruction's semantics or a table's
   extent genuinely cannot be settled by reading, run a one-off **<10-frame** PyBoy
   memory-poke one-liner. Never save its output as a capture, never add a file to
   `captures/`.

4. **RUN THE EXISTING REPLAY GATE ONCE.** Only after a work package compiles and its unit
   tests pass, run the existing `--replay` regression suite ONCE with quiet/summary output:
   ```bash
   bash captures/sweep.sh 2>&1 | tail -n 25
   ```
   `sweep.sh` enumerates its captures by name — a new capture file is never picked up
   automatically. Do not run it per-iteration.

## Editing `PORT_QUEUE.json` (and DESIGN.md) from a shell script

Validate before you write, not after: shell heredocs silently turn `$0EB0` into `\$0EB0`,
and one stray backslash makes `PORT_QUEUE.json` unparseable while `cargo test` stays green
(the core tests never read it, only the queue tooling does). So a script that rewrites the
queue must `json.loads` the new text **before** `write_text`, and the write should keep only
genuinely valid JSON escapes (see the repair commit that fixed this twice). Cheap check:
`python3 -c "import json;print(len(json.load(open('PORT_QUEUE.json'))['queue']))"`.

## Verifying a rendering claim

The replay gate compares state bytes, so it cannot see a background/sprite
misalignment — a render bug passes it green (that is how this one survived 171
tests). When the claim is about the picture, take a console frame and diff it:

```bash
# console: boot to the level through the map route, then grab the screen
#   PyBoy, window='null'; the API is `p.screen.image` (there is no `screen_image()`)
#   from probe_level_select import boot_to_map, enter_level, node_for_level
# port: the same camera and Mario position
cargo run -q -p sml2-runtime -- --level N --camera CX CY --mario MX MY 0 --out /tmp/port.png
```

Compare shade indices row by row (map the console's 153 and the port's 170 onto one
shade), then slide one image against the other over ±16 rows: the best-fit shift names
the bug. Keep both frames' cameras in the assertion so the finding is a test, not a
screenshot.

## Reading large artifacts

`captures/*.json` are 1–20 MB, `extracted/*.json` several hundred KB, `PORT_QUEUE.json`
~135 KB. Never read any of them whole, and never re-read the queue every turn — one
wholesale read is re-billed on every later turn and forces a compression that breaks the
provider's prefix cache.

`jq` is **not installed** on this host. Query in place with Python or grep:

```bash
python3 -c 'import json;d=json.load(open("captures/traj17.json"));print(d[:40])'
python3 -c 'import json;print([e["id"] for e in json.load(open("PORT_QUEUE.json"))["queue"] if e["status"]!="done"])'
grep -n 'PATTERN' notes/design_notes.md | head -n 40
```

Whole-corpus work belongs in an offline analysis script, never in a tool result.

## Backlog rules

- An item's `status` is a claim about **verification**, not about lines written. A `done`
  whose gate named a capture that does not exist is unverified.
- A gate naming a **new capture** is void — re-gate it to unit tests before doing the work.
- Writer/consumer pairing: when you port a writer, grep the sim for the readers it makes
  live; when you add an entry point, grep for its call sites. A subsystem nothing calls is
  silent — make the dead case loud through the existing `note:` mechanism.
- Retired capture-only items are archived in `PORT_QUEUE.json.retired_capture_tasks`;
  the live rules are in `PORT_QUEUE.json.workflow_rules`.
