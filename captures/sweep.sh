#!/usr/bin/env bash
# The 18-capture gate. Prints one line per capture; a clean run prints only the
# documented traj4_ff0 exception's own numbers.
set -uo pipefail
cd /home/pmill/sml2-rs
BIN=./target/release/sml2-runtime
cargo build --release 2>&1 | grep -E "^(error|warning: unused)" || true
for f in traj4_ff0 traj5 traj6 traj7 traj8 traj9 traj10 traj11 traj12 traj13 \
         traj14 traj15 traj16 traj17 death1 death2 death3 crouch_pan1 screen_transition1; do
  out=$($BIN --level 0 --replay "captures/$f.json" 2>&1)
  line=$(printf '%s\n' "$out" | grep -E "^mismatches" | tail -1)
  notes=$(printf '%s\n' "$out" | grep -c "note:")
  if [ "$f" = "traj4_ff0" ]; then
    printf '%-20s %s  notes=%s\n' "$f" "$line" "$notes"
  elif [ "$notes" != "0" ] || [ -z "$line" ]; then
    printf 'REGRESSION %-20s %s  notes=%s\n' "$f" "$line" "$notes"
    printf '%s\n' "$out" | grep -E "note:|mismatch" | head -5
  else
    printf '%s\n' "$line" | grep -q "=0$" || { printf 'REGRESSION %-20s %s\n' "$f" "$line"; }
  fi
done
printf 'gate done\n'
