# Agent orchestration

[PORTING.md](PORTING.md) is binding. Supervisors select/review bounded work;
workers implement. Never assign a monolith or multiple handlers to one worker.
Schedule shared subsystem dependencies before their handlers, then closure and
route/integration work from the [completion inventory](docs/route-matrix.md).
Tool contracts: [tools/README.md](tools/README.md).

## Roles and scope

| Role | Work / model |
|---|---|
| Supervisor | Read policy, pending queue and coverage; select one ticket and accept/reject its diff |
| Slicer | Deterministic scripts only; generate source slice/inventories |
| Porter | Strong coding model (V4 Pro/Argon/Sonnet class); one complete slice, named native module/adapter, tests and registrations |
| Escalation porter | After two failed fixes on the same handler, Opus/Sonnet tier with the same scope |
| Integrator | Script/small model: tests and once-per-package frozen sweep |

Flash/small models do not own logic ports. Porter context target <=15k tokens:
brief plus <=4 named Rust files and errors. Supervisor context target <=8k.
Do not attach the whole `objects/` tree, `main.asm`, DESIGN.md, archived queue, captures,
notes or multiple handler bodies. Do not bulk-transpile assembly into Rust.

## Dispatch loop

1. Refresh `python3 tools/handler_coverage.py`: 0 checked, 1 pending, 2 invalid
   inputs (stop and repair). Inspect spawn-reachable, level-table and boss-table
   inventories; empty spawn coverage is not full-game completion.
2. Select one pending queue item or handler. Generate its worker message:

   ```sh
   python3 tools/dispatch_prompt.py --addr 0x58201
   python3 tools/dispatch_prompt.py --queue QUEUE_ID --file path/to/file.rs
   ```

3. Handler generation must exit 0 with `Slice status: COMPLETE`. Repair source/
   dependencies on exit 2. New handlers need a descriptive `--module`; generator
   stdout alone is the worker user message. Existing helpers must match source
   before being declared dependencies.
4. Scope one native behavior, mirrored compatibility adapter, category tests,
   one dispatch arm and module registrations. Queue tickets similarly name exact
   allowed files and source references. Typed actor rules and shared domain
   behavior follow [the native guide](docs/idiomatic-rust-porting.md).
5. Review diff scope, independent expected-byte assertions and sensitivity to
   reverting new behavior. Require focused tests and `cargo test -p sml2-core`.
   Keep queue `status` as a verification claim, not a writing-progress marker.
6. Refresh handler coverage to confirm the implemented target; reject invalid
   inputs. Close a verified queue item with `python3 tools/queue_done.py ID`.
   Do not append note journals. Commit only this step's files.
7. At the work-package boundary, after tests pass, run once:

   ```sh
   set -o pipefail
   bash captures/sweep.sh 2>&1 | tail -n 25
   ```

Source-byte tests establish behavior; replay proves regression, not pixels.
Reject overbroad diffs, incomplete slices, raw-slot domain logic or new captures.

## Live inventory and integrations

```sh
python3 -c 'import json; print([e["id"] for e in json.load(open("PORT_QUEUE.json"))["queue"]])'
```

Copy-paste system prompts: [Hermes](hermes/README.md). Cursor rules are in
`.cursor/rules/`; do not duplicate the workflow into session journals or plans.
