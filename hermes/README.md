# Hermes prompts for sml2-rs

Copy these into Hermes agent roles (system + user). Repo truth stays in `ORCHESTRATION.md` / `PORTING.md`.

| File | Role |
|------|------|
| [`supervisor.system.md`](supervisor.system.md) | Supervisor system prompt |
| [`worker-handler.system.md`](worker-handler.system.md) | Porter system prompt (handlers) |
| User message | Generate with `python3 tools/dispatch_prompt.py --addr 0x58201` |

## Suggested Hermes setup

1. **Supervisor agent** — model: strong reasoning; system = `supervisor.system.md`; tools: shell.
2. **Porter agent** — model: V4 Pro / Argon / Sonnet-class; system = `worker-handler.system.md`; **no** repo-wide search; context = output of `dispatch_prompt.py` only.
3. **Integrator** (optional) — flash OK; runs `cargo test` + `captures/sweep.sh` when supervisor requests a package gate.

## Quick dispatch

```bash
python3 tools/handler_coverage.py
python3 tools/dispatch_prompt.py --addr 0x58201   # paste stdout into Porter user message
```

For queue items (non-handler):

```bash
python3 tools/dispatch_prompt.py --queue camera_pan_timer
```

Coverage exits 1 for pending work and 2 for invalid inputs (stop and repair).
Dispatch regenerates the brief and rejects incomplete slices. See `tools/README.md`.
