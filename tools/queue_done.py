#!/usr/bin/env python3
"""Move a pending PORT_QUEUE item to PORT_QUEUE.done.json.

Usage: python3 tools/queue_done.py <item_id>
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIVE = ROOT / "PORT_QUEUE.json"
DONE = ROOT / "PORT_QUEUE.done.json"


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {sys.argv[0]} <item_id>")
    item_id = sys.argv[1]

    live = json.loads(LIVE.read_text())
    done_doc = json.loads(DONE.read_text()) if DONE.is_file() else {"archived": "2026-10-15", "queue": []}

    found = None
    rest = []
    for e in live["queue"]:
        if e["id"] == item_id:
            found = e
        else:
            rest.append(e)
    if found is None:
        raise SystemExit(f"id not in pending queue: {item_id}")

    found["status"] = "done"
    done_doc.setdefault("queue", []).append(found)
    live["queue"] = rest
    n_pending = len(rest)
    live["milestone_note"] = (
        f"{n_pending} pending in PORT_QUEUE.json; work per PORTING.md / ORCHESTRATION.md."
    )

    for path, doc in ((LIVE, live), (DONE, done_doc)):
        text = json.dumps(doc, indent=1) + "\n"
        json.loads(text)
        path.write_text(text)

    print(f"archived {item_id}; {n_pending} pending")


if __name__ == "__main__":
    main()
