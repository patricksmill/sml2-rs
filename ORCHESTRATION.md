# Agent orchestration

**For supervisors** (Hermes, Cursor multi-agent, or human dispatch). Workers implement; supervisors **never** port logic in a monolith.

Canonical port rules: [`PORTING.md`](PORTING.md). Execution phases: [`docs/plans/2026-10-15-port-finish-reset.md`](docs/plans/2026-10-15-port-finish-reset.md).

For full-game completion, use the dependency ordering, tooling prerequisites,
and milestone gates in
[`Systematic Rust port completion workflow`](docs/plans/2026-10-15-rust-completion-workflow.md).
Its dependency ordering takes precedence over the handler-first ordering below:
schedule shared subsystem blockers before handlers that depend on them. Tool commands and failure statuses are documented in [`tools/README.md`](tools/README.md).

---

## Roles

| Role | Reads | Does | Model |
|------|--------|------|--------|
| **Supervisor** | This file, `PORTING.md`, pending `PORT_QUEUE.json`, `handler_coverage.json` | Pick one ticket, run slicer, write worker prompt, accept/reject PR | Any; no ASM required |
| **Slicer** | — | `python3 tools/handler_task.py --addr 0x…` or `tools/handler_coverage.py` | **No LLM** (scripts) |
| **Porter** | One `tasks/handlers/*.md` + named Rust files only | Implement + `#[test]` | **Strong** (V4 Pro, Argon, Sonnet-class). **Not** flash/8B for logic |
| **Escalation porter** | Same bounded files + compiler errors | After **2** failed fix attempts on same handler | Opus / Sonnet, same scope |
| **Integrator** | — | `cargo test -p sml2-core`, then `bash captures/sweep.sh 2>&1 \| tail -n 25` once per package | Script or small model |

**Forbidden worker assignments:** whole `objects.rs`, multiple handlers, “read `main.asm`”, `PORT_QUEUE.done.json`, `captures/*.json`, `notes/`, bulk RGBDS→Rust transpile.

---

## Supervisor loop (handlers — bulk of remaining work)

```
1. python3 tools/handler_coverage.py   # refresh report; exit 1 = pending, exit 2 = STOP (invalid inputs)
2. Pick next ROM address from unported_spawn_reachable (prefer greedy level order in plan Phase 3)
3. python3 tools/handler_task.py --addr 0x{ADDR}
4. Dispatch porter with prompt template below
5. Require: cargo test -p sml2-core passes for new/changed tests
6. Optional: revert-one-line check on new branches (PORTING.md)
7. Repeat from 1; do NOT run sweep every handler — only per work package / Phase 5
```

Track `level_tables.unported` and `boss_table.unported` as well as spawn coverage.
Close required subsystem dependencies before their handlers, and audit dynamic
spawn chains per the completion roadmap. Read pending subsystem tickets from the
live queue; an empty spawn list alone does not complete the game.

---

## Supervisor loop (queue item)

```
1. Read one item from PORT_QUEUE.json (id, title, asm_refs)
2. List allowed files in the prompt (e.g. camera.rs + camera tests only)
3. Dispatch porter; gate = PORTING.md (unit tests, no captures)
4. On pass: move item to PORT_QUEUE.done.json, remove from PORT_QUEUE.json
   (python3 tools/queue_done.py <id> when available; else manual JSON edit with json.load)
5. Integrator runs sweep once if the item closed a multi-handler package
```

---

## Worker prompt templates

### Handler (default)

```text
You are the Porter. Rules: PORTING.md (binding).

Apply docs/idiomatic-rust-porting.md: domain handlers use Actor fields/enums
and shared methods. Raw slots are converted only at the compatibility boundary;
do not extend byte-indexed handler logic or recreate HRAM scratch.

Task: Port object handler ROM 0x{ADDR} using ONLY:
  - tasks/handlers/0x{ADDR}.md
  - one named handler module under crates/sml2-core/src/objects/
    (Wario-specific modules go under objects/bosses/wario/)
  - crates/sml2-core/src/actor.rs
  - one named compatibility adapter sibling module
Touch the named domain/adapter modules plus dispatch.rs (one arm) and the owning mod.rs
(registrations only). Resolve missing native shared behavior as a prerequisite.
Require a COMPLETE generated slice; place new tests in the named module.

Do not read objects.rs monolith, DESIGN.md, PORT_QUEUE, or main.asm wholesale.

Deliverables:
  1. native handler + compatibility adapter + one arm in dispatch.rs
  2. native behavior tests and ROM-derived slot bytes at the conversion boundary
  3. cargo test -p sml2-core {test_filter} must pass

If borrow-checker or cross-bank control flow blocks you after 2 attempts, stop and report ESCALATE.
```

### Subsystem (example: camera_pan_timer)

```text
You are the Porter. Rules: PORTING.md.

Task: Close PORT_QUEUE item "{id}" — {title}
asm_refs: {asm_refs}
Touch ONLY: {file_list}

Deliverables: unit test(s) + minimal Rust change. cargo test -p sml2-core.
No new captures.
```

---

## Context budget

| Worker turn | Target input size |
|-------------|-------------------|
| Porter | ≤ ~15k tokens: slice md + ≤4 Rust files + test output |
| Supervisor | ≤ ~8k: this file + queue ids + handler_coverage summary |

Refresh `handler_coverage.json` before batch planning; do not paste the full unported list into every worker prompt — send **one** `0x{ADDR}`.

---

## What “accept” means

- [ ] Diff touches only files named in the ticket
- [ ] New/changed logic has a unit test with byte assertions
- [ ] `cargo test -p sml2-core` green
- [ ] No new files under `captures/` or `notes/`
- [ ] For handlers: `handler_coverage.py` confirms the selected target is implemented in its applicable level/boss inventory (after refresh); exit 2 rejects the package

Reject and re-dispatch if the worker read the whole object engine or added queue `note` essays.

---

## Hermes / Cursor wiring

**Configured in-repo:**

| Target | File |
|--------|------|
| Hermes supervisor system prompt | [`hermes/supervisor.system.md`](hermes/supervisor.system.md) |
| Hermes Porter system prompt | [`hermes/worker-handler.system.md`](hermes/worker-handler.system.md) |
| Porter user message (generate) | `python3 tools/dispatch_prompt.py --addr 0x{ADDR}` or `--queue <id>` |
| Cursor always-on rule | [`.cursor/rules/sml2-orchestration.mdc`](.cursor/rules/sml2-orchestration.mdc) |
| Cursor Porter (objects glob) | [`.cursor/rules/sml2-objects-porter.mdc`](.cursor/rules/sml2-objects-porter.mdc) |

1. Paste supervisor / Porter **system** prompts from `hermes/` into Hermes agent settings.
2. Each Porter **user** message = stdout of `dispatch_prompt.py` only.
3. Do not attach `PORT_QUEUE.done.json`, `DESIGN.md`, or `captures/README.md` to workers.
4. Flash / small models: **Slicer + Integrator only** (run commands, parse test output).

---

## Pending backlog (snapshot)

Query live list:

```bash
python3 -c 'import json;print([e["id"] for e in json.load(open("PORT_QUEUE.json"))["queue"]])'
```

Handler progress:

```bash
python3 tools/handler_coverage.py  # 1 = pending work; 2 = invalid inputs, stop
python3 -c 'import json;d=json.load(open("handler_coverage.json"));print(len(d["unported_spawn_reachable"]), "unported")'
```
