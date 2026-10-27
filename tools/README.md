# Porting tools

Run from the repository root with extracted assets and `../sml2decomp/` (or
`SML2_DECOMP_ROOT`). [PORTING.md](../PORTING.md) defines acceptance;
[ORCHESTRATION.md](../ORCHESTRATION.md) defines bounded worker dispatch.

## Inventories and gates

| Command | Claim | Exit status |
|---|---|---|
| `python3 tools/handler_coverage.py` | Source/declared Rust dispatch, including both level tables and bosses | 0 all implemented/no-op; 1 pending; 2 invalid/missing input |
| `python3 tools/anim_closure.py` | Per-level fixpoint over spawn/morph/rewritten anim IDs | 0 closed; nonzero failure; `--self-test` must detect range and missing-handler mutations |
| `python3 tools/anim_inventory.py --frames` | Reachable player-frame bounds, extracted mappings and source citations | 0 checked; 1 failing/uncited frame; 2 invalid input or stale/unbounded declaration |
| `python3 tools/smoke_levels.py` | 32 manifests/assets and idle/walk native execution | 0 runs; 1 failing level; 2 missing input/binary |
| `python3 tools/route_matrix.py --table` | Source entries/exits/secret routes/mechanics/bosses | 0 generated; 2 missing inputs |
| `bash captures/sweep.sh` | Frozen regression, with exactly the documented traj4_ff0 exception | Nonzero on unexpected mismatches; preserve pipeline exit status |

Coverage/closure are not behavior parity. Profile table A applies below level
ID `$0A`, B from `$0A`. Dynamic closure includes level roots, dynamic/boss spawns,
interactions and record rewrites; type-set edges over-approximate reachability.
Smoke fails on panic/nonzero status, notes, unported IDs, unapplied reloads or
invalid assets, but never proves an exit was reached. See the
[route evidence](../docs/route-matrix.md) for played versus source-only coverage.

Useful options: anim closure/inventory accept `--self-test`; anim inventory accepts
`--frames --json out.json`; smoke accepts `--levels 12,19 --frames 600 --json out.json`;
route matrix accepts `--levels 1..32 --table --json out.json`. Build
`sml2-runtime` before smoke. `--json` output is local diagnostics, not a new
capture fixture.

## Handler briefs

```sh
python3 tools/handler_task.py --addr 0x58201
python3 tools/handler_task.py --anim 4 --mode boss
python3 tools/handler_task.py --addr 0x58201 --max-states 800
```

0x58201 is bank `$16` (22 decimal), CPU `$4201`. The slicer follows exact ROM
instruction starts, direct control flow, inline RST tables, known bank writes
and known HL/stack transfers. It resolves unlabeled entries exactly; it never
substitutes the nearest routine. Mixed symbolic/numeric table declarations must
match ROM words in full; unknown names/expressions or truncation invalidate them.

COMPLETE/exit 0 means bounded traversal finished, not Rust parity. Review inferred
state/table bounds and bank effects. Unknown computed transfers, stack returns,
bank writes, missing source or resource limits produce INCOMPLETE/exit 2. Invalid
inputs also exit 2, without a ready brief. Interrupts and arbitrary computed targets
are not inferred. Regenerate source symbols using the decomp's build instructions
if source locations cannot resolve; `--out-dir` writes scratch briefs.

```sh
python3 tools/handler_task.py --addr 0x58201 \
  --dependency '0x5B51A=crates/sml2-core/src/objects/helpers.rs::sprite_toggle'
```

A dependency is a reviewed equality assumption about an existing native helper,
including bank preservation. The tool checks existence, not semantic equality;
port an absent helper first. `--bank` supplies the bank for a home entry.

Reviewed specialized entry contracts:

```sh
python3 tools/handler_task.py --addr 0x68572 --entry-contract ending-text-vblank
python3 tools/handler_task.py --addr 0x26D7 --entry-contract ending-transfer-vblank
python3 tools/handler_task.py --addr 0x5902D --state-table-contract anim53-state0-4
```

Use `--help`/contract modules for supported names and assumptions. Contract
witnesses hash ROM bytes and exact SourceIndex instruction/data records. Wrong
entry/bank/helper bindings, changed declarations/writers/control flow or stale
source invalidate them and require renewed review. Resource limits still apply.
A restricted caller/interrupt/state contract never proves excluded entry states,
engine initialization, interaction behavior or native helper parity.

The generic state-bounded walk is attempted only after plain traversal is
incomplete and retained only when complete. Unbounded writers, unguarded indexes
and out-of-declaration bounds cannot bless an RST table. Special contracts remain
the stronger explicit byte/source-witnessed evidence.

## Worker prompts

```sh
python3 tools/dispatch_prompt.py --addr 0x58848
python3 tools/dispatch_prompt.py --addr 0xAD93 \
  --module crates/sml2-core/src/objects/bosses/wario/form3.rs
python3 tools/dispatch_prompt.py --queue QUEUE_ID --file path/to/file.rs
```

Successful stdout is the worker message; diagnostics go to stderr. Handler
prompts regenerate the brief and reject incomplete slices. Existing dispatch
entries resolve their named native module; new entries require `--module`.
Adapters mirror native filenames under `compat/`; tests belong to the category.
Prompts name bounded files, one dispatch arm and registrations. Dependencies and
state budgets pass through. Queue prompts require explicit, repeated `--file`
arguments and reject missing files or paths outside the repository; no retired
ticket-to-file table is maintained.

## Checks and playback

```sh
python3 -m unittest discover -s tools/tests
cargo test -p sml2-core
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

Tool tests cover status accounting, source/symbol/table validation, contract
mutations, incomplete briefs, dispatch rejection and regression-gate failures.
TAS import, comparison and manual recording usage is in
[TAS playback](../docs/tas-playback.md). Literal inputs are not synchronization
or full-game acceptance evidence. Do not create new trajectory captures.
