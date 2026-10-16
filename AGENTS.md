# AGENTS.md — sml2-rs

Native Rust port of Super Mario Land 2, verified against `../sml2decomp/` and `baserom.gb`.

## Read this first

| Doc | Use |
|-----|-----|
| [`ORCHESTRATION.md`](ORCHESTRATION.md) | **Supervisor loop**, worker prompts, model roles, accept/reject |
| [`PORTING.md`](PORTING.md) | Verification, one-ticket rules, forbidden moves |
| [`docs/plans/2026-10-15-rust-completion-workflow.md`](docs/plans/2026-10-15-rust-completion-workflow.md) | Full-game completion order, tooling prerequisites, task loop, milestone gates |
| [`DESIGN.md`](DESIGN.md) | Architecture, renderer seams, standing goals |
| [`PORT_QUEUE.json`](PORT_QUEUE.json) | **Pending items only** — `python3 -c 'import json;print(json.load(open("PORT_QUEUE.json"))["queue"])'` |
| [`PORT_QUEUE.done.json`](PORT_QUEUE.done.json) | Archived completed items (ignore unless auditing history) |
| [`docs/plans/2026-10-15-port-finish-reset.md`](docs/plans/2026-10-15-port-finish-reset.md) | Execution plan (split `objects`, handler factory, queue closeout) |
| [`captures/README.md`](captures/README.md) | Frozen replay corpus (read-only) |
| [`hermes/README.md`](hermes/README.md) | Copy-paste Hermes supervisor/Porter system prompts |

## Quick gate

```bash
cargo test -p sml2-core
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25   # once per work package, after tests pass
```

## Large files

Do not load into context: `captures/*.json`, `PORT_QUEUE.done.json`, whole `DESIGN.md` for a handler task.

```bash
python3 -c 'import json;print([e["id"] for e in json.load(open("PORT_QUEUE.json"))["queue"]])'
```

## Queue hygiene

- `status` = verification claim, not “lines written”.
- Capture-based gates are void; use `PORTING.md` unit tests.
- Do not append session journals to queue `note` fields — omit them.

## Commits

Create a focused commit after each completed, verified work step. Stage only
files belonging to that step and preserve unrelated local changes.
