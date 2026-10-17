# First native pulse effect sequencer prerequisite

One bounded sound effect, slot0 ID6, required by ending state0F. Exact COMPLETE
packets tasks/handlers/0x101F3.md and0x101F9.md. Generic object-module wording in
source generator is not architecture authority: sound belongs under audio/.
Read PORTING/native guide, packets, /tmp/sml2-sound-dispatch-audit.md native unit
section and bounded main.asm9591..9605, ram.asm source annotations for referenced
named fields (rg first). Root independently confirms frame data at1020B..10216:
[00,BA,C0,B6,C6,08] then [00,BA,C0,39,C7,02]. No whole source/engine.

Touch ONLY new core audio/mod.rs, audio/pulse.rs (+local or sibling tests), and
lib.rs export. No dependency/network/runtime changes. Provide domain-typed
persistent effect state, source timer/step/active/duration/busy and referenced
music ownership/vibrato values; preserve unspecified fields and source wrapping.
Start copies request to active (does not consume request), timer/step reset,
claims channel ownership bit7 preserving all other flagbits, emits five ordered
pulse-control writes from first frame and sets timer8. Tick decrements wrapping;
nonzero means no command; at zero increments step wrapping; equality2 stops,
otherwise emits second frame and timer2. Stop clears active,timer,step,duration,
channel vibrato timer,busy; emits sweep0 and releases bit7, preserves lower flags.
Native helpers should encapsulate ownership flags and ordered pulse writes.

Use typed audio control events (sweep,duty/length,envelope,frequency low/control)
with actual source values and ordering retained. This is a sound sequencer and
output seam; do not fake acknowledgment/no-op audio tick, add a byte-backed RAM
bus, CPU registers, or runtime ROM reads. Native audio synthesis/output and full
UpdateSound request dispatch are explicit later dependencies, not completion
claims. Shared common start/stop/frame logic should be reusable for other effects.
Don't silently clamp timer/step or invent guards absent source; API contract can
require caller select proper entrypoint. Explicit source-derived request/active
and dirty-state tests must cover that contract.

Tests independent exact command bytes/order, dirty-state preservation, full
8+2 tick sequence and silence waits, timer0 wrap255, step255 wrap0, equality-only
stop including noncanonical step2 ->3, lower musicflags preservation and source
request remains untouched. Meaningful mutants timer/order/equality/cleanup must
fail restored. Focused tests allowed only AFTER root releases lane for lib
registration; prepare unregistered modules first while sprite-batch worker owns
lane. Ask root before mutations/fullsuite. No commit/sweep/inventory/queue edits.
