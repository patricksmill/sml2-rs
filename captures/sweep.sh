#!/usr/bin/env bash
# Run the frozen regression corpus; preserve the Python gate's failure status.
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
exec python3 "$ROOT/tools/replay_gate.py" "$@"
