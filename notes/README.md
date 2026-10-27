# notes/ (retired)

Session journals and delegation specs lived here until **2026-10-15**. They were removed on purpose: agents treated them as required reading and duplicated stale Hermes output.

**Use instead:**

- [`PORTING.md`](../PORTING.md) — how to work
- `tasks/handlers/` — per-handler ASM slices, generated locally by `tools/handler_task.py` (not committed)
- [`../sml2decomp/`](../../sml2decomp/) + `baserom.gb` — truth
- Rust module headers + `#[test]` — verified semantics

Do not recreate long-form research files here. If something is not in tests or code comments, port it or delete the claim.

`PORT_QUEUE.json` `note` fields were removed 2026-10-15; use `title` + `asm_refs` only.
