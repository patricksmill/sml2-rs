#!/usr/bin/env bash
# The capture gate. Prints one line per capture; a clean run prints only the
# documented traj4_ff0 exception's own numbers.
# traj19 is the one capture that does not run at --level 0: it is the level-20
# (index 19) autoscroll capture, booted through captures/probe_level_select.py's
# node-61 route, so it replays at --level 19.
set -uo pipefail
cd /home/pmill/sml2-rs
BIN=${SML2_BIN:-./target/release/sml2-runtime}
cargo build --release 2>&1 | grep -E "^(error|warning: unused)" || true
for f in traj4_ff0 traj5 traj6 traj7 traj8 traj9 traj10 traj11 traj12 traj13 \
         traj14 traj15 traj16 traj17 traj19 death1 death2 death3 crouch_pan1 screen_transition1; do
  lvl=0; [ "$f" = "traj19" ] && lvl=19
  out=$($BIN --level "$lvl" --replay "captures/$f.json" 2>&1)
  line=$(printf '%s\n' "$out" | grep -E "^mismatches" | tail -1)
  notes=$(printf '%s\n' "$out" | grep -c "note:")
  if [ "$f" = "traj4_ff0" ]; then
    printf '%-20s %s  notes=%s\n' "$f" "$line" "$notes"
  elif [ "$notes" != "0" ] || [ -z "$line" ]; then
    printf 'REGRESSION %-20s %s  notes=%s\n' "$f" "$line" "$notes"
    printf '%s\n' "$out" | grep -E "note:|mismatch" | head -5
  else
    # EVERY field must be zero, not only the line's last one. The old test
    # (`grep -q "=0$"`) looked at the tail only, which is `coinshi=0` for most
    # captures — so a check that regressed mid-table was invisible here.
    bad=$(printf '%s\n' "$line" | sed 's/^mismatches://' | tr ' ' '\n' | grep -v '^$' | grep -v '=0$' | tr '\n' ' ')
    if [ -n "$bad" ]; then
      printf 'REGRESSION %-20s nonzero: %s\n' "$f" "$bad"
      printf '%-20s %s\n' "" "$line"
    fi
  fi
done
printf 'gate done\n'
