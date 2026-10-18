# Porting tools

Run from the repository root. These tools use the standard library and locally
extracted assets; no emulator or network access is needed. Set
`SML2_DECOMP_ROOT` to override the sibling disassembly/ROM directory.

## Coverage

```bash
python3 tools/handler_coverage.py
```

Exit **0** means all declared level/boss targets have implementation arms,
**1** means pending handlers, and **2** means invalid inputs. Do not mask all
nonzero statuses with `|| true`. A failed input audit leaves the previous report
untouched; stop before reading it as a fresh measurement.

The generated, ignored `handler_coverage.json` retains `by_level` and
`unported_spawn_reachable` for existing consumers. It adds
`ported_spawn_reachable`, `level_tables`, and `boss_table`, each separating bodies,
ROM-verified bare-return no-ops, and pending targets. All 32 manifests are required;
invalid anim indices, missing profiles/sources, and invalid no-op arms fail.

Level IDs 0–9 use mode A; 10–31 use B, matching `$EC4B` and Rust. Dispatch arm
counts are implementation inventory, not test coverage. Boss-table inventory is
conservative: all declared boss entries are counted, without claiming every entry
is reachable. Dynamic spawn/state closure remains a separate source audit.

## Handler briefs

```bash
python3 tools/handler_task.py --addr 0x58201
python3 tools/handler_task.py --anim 4 --mode boss
python3 tools/handler_task.py --addr 0x58201 --max-states 800
```

ROM `0x58201` is bank `$16` (22 decimal), CPU `$4201`. The slicer uses exact ROM
instruction boundaries, direct branches/calls/returns, inline `rst $28` tables,
known immediate bank writes, and known HL/stack transfers. Source lines come from
`home.asm` and `main.asm`, anchored by address labels and optionally `sml2.sym`.
An unlabeled entry does not fall back to a nearby label's routine.

State-table `dw` rows may mix numeric words with named labels resolved from
explicit ROM-address labels or `sml2.sym`. Rows containing symbols must match
the encoded ROM words before they supply table bounds or advance the source
cursor. Unknown names, expressions and mismatched words invalidate the entire
contiguous declaration, including any preceding numeric rows; a truncated
prefix never becomes a ready dispatch table.

The status **COMPLETE** means traversal finished within this static model. It
requires source review of inferred table bounds and bank effects, and says
nothing about Rust parity. Unknown computed transfers, unknown bank writes,
unknown stack returns, missing source locations, and resource limits produce an
**INCOMPLETE** packet and exit **2**. Invalid inputs also exit 2 without a ready
packet. The tool does not infer arbitrary data-dependent targets or interrupts.

A reviewed, existing helper can bound a large slice:

```bash
python3 tools/handler_task.py --addr 0x58201 \
  --dependency '0x5B51A=crates/sml2-core/src/objects/helpers.rs::sprite_toggle'
```

The supervisor must confirm that ROM entry and Rust method have the same effects,
including preserving the selected bank. The tool verifies the file/method exists
and lists this assumption in the brief; it does not establish semantic equality.
If the dependency is unported, create its own task first. `--bank 0x16` specifies
the selected bank when slicing a home-bank entry. `--out-dir` allows scratch
briefs without editing tracked tasks. Regenerate symbols with the decomp's
established build instructions if source locations cannot be resolved.

For ending routines with verified VBlank entry frames and interrupt terminals,
`--entry-contract` scopes traversal to closed, byte-checked caller preconditions:

```bash
python3 tools/handler_task.py --addr 0x68572 --entry-contract ending-text-vblank
python3 tools/handler_task.py --addr 0x26D7 --entry-contract ending-transfer-vblank
```

These contracts check caller and body witnesses against ROM bytes and SourceIndex,
preserve the VBlank saved-register frame, and close interrupt control flow at `reti`
without following the opaque interrupted PC or inventing native helper bindings.
Generic traversal without `--entry-contract` continues to report `INCOMPLETE`.

For the reviewed mixed declaration used by anim A:53, opt in explicitly:

```bash
python3 tools/handler_task.py --addr 0x5902D --state-table-contract anim53-state0-4
python3 tools/dispatch_prompt.py --addr 0x5902D --state-table-contract anim53-state0-4
```

This contract scopes COMPLETE to entry FFDF in {0,1,2,3,4}, selected bank $16,
stable ROM, ordinary calls/returns, and no external staging mutation. It overrides
only dispatch 0x59079: four pointers occupy [0x5907A,0x59082); the remaining 16
bytes are two eight-byte displacement arrays. SourceIndex retains the entire
[0x5907A,0x59092) declaration. Generic slicing remains INCOMPLETE, including when
trailing offset pairs happen to look like valid pointers. No first-invalid-pointer
heuristic or Rust dependency binding supplies the bound.

`state_table_contracts.py` records immutable SHA256 witnesses for every byte in
the audited handler/helper instruction closure (including conservative removal
reward/spawn branches), home RST body, mixed declaration and movement data.
Source witnesses hash each exact instruction start, or every data byte, with the
complete SourceIndex file/line/instruction text; the module documents the stable
record encoding. Byte/source changes require renewed source review. Validation
rejects missing source, altered declaration extents, wrong entry/bank, helper
bindings and changed writers/calls/stores; traversal also rejects transfers
outside the checked closure. Resource budgets still apply. The packet prints
witnesses, assumptions and excluded paths: arbitrary FFDF>=5, other entry offsets,
global initialization, engine selection, interactions and saved-state restoration
are outside the claim. Native helper parity remains separate work.

## Dispatch prompts

```bash
python3 tools/dispatch_prompt.py --addr 0x58201
python3 tools/dispatch_prompt.py --addr 0x58201 \
  --module crates/sml2-core/src/objects/wario/form3.rs
python3 tools/dispatch_prompt.py --queue boss_arena_phase_cascade
```

Handler prompts always regenerate the brief and stop if it is incomplete. By
default they name a new `handler_0xADDR.rs` domain module, a bounded
`handler_0xADDR_adapter.rs` compatibility module, native tests with ROM-byte
assertions at the conversion boundary, one dispatch arm, and module registrations.
Prompts require the native guide and `Actor` API; raw slot/register logic belongs
outside domain handlers. Resolve missing native helpers first. Choose new modules
when a family or adapter is already too large. Family subfolders are supported;
Wario modules live in `objects/wario/`, and prompts register them in
`objects/wario/mod.rs`. Coverage searches family subfolders recursively. `--dependency` and `--max-states` pass through to slicing. Queue prompts
validate the named current files and include existing object integration tests
when appropriate. Successful stdout is the worker message; generator diagnostics
are written to stderr.

## Regression gate

```bash
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

The wrapper locates the repo relative to itself, then runs `replay_gate.py`.
It builds release successfully before replaying, checks runtime exit status,
requires one well-formed summary with required fields, rejects unexpected notes,
and checks every reported mismatch field. `traj4_ff0` must match the exact frozen
field/count baseline; new fields, changed counts, and notes fail. The gate exits
nonzero on any failure. `SML2_BIN` still overrides the replay binary; it does not
skip the build. Do not change the legacy expectation to bless a failing package.

## Tool checks

```bash
python3 -m unittest discover -s tools/tests -v
```

Fixtures cover mode boundaries, separate boss/no-op accounting, invalid and
missing inputs, exact unlabeled entries, far state branches, bank trampolines,
HL/stack transfers, incomplete slices, prompt rejection, and gate failure paths.
Use `cargo test -p sml2-core`, then the frozen sweep once at the package boundary.
