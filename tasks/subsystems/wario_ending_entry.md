# Native Wario ending entry control

One bounded subsystem: COMPLETE source briefs `$5E12` (bank1 phase3 scripted
input), `$03AF` (home state35 fade selection), `$2CAE` (home -> bank26 `$68000`
state18 initializer) in tasks/handlers. Source order: phase3 input overrides
held AND pressed before player/camera/objects. X low rounded even below160:
Right+B, no presses. X low160/161: Right+A, A pressed every frame. Above161:
Right+A, no presses; Y low>=224 requests state35. That frame STILL executes
ordinary player/camera/objects; state35 dispatch starts next frame.

State35: palettes from ROM3E1 at offset ((fade.low &C0)>>4), three bytes; test
full fade word for zero, when zero set fade255 and state18. Next frame state18
initializes ending_state0 and boss phase0 and enters state19 ($6800F). It does
not run state19 on its creation frame. Existing native ending sequence state19
body is separate pending ticket; expose explicit native Sequence state/event,
no ordinary level-exit substitute or fake successful ending.

Implement clean EndingEntry owner with typed state enum, typed held/press input
and palette effects. New bounded core ending_entry.rs (+ sibling tests), lib.rs
export; runtime main.rs ONLY local owner construction, bounded shared frame-head
helper, integration into BOTH live/replay loops and tests. Avoid enlarging giant
logic: native state code in core sibling. Read native guide/PORTING, named briefs,
PlayerKeys/PlayerSim step/tick_fade signature, bounded runtime frame head/player/
object pass locations. No whole giant runtime/player source loads.

Keep existing gameplay/death/pause timing until relevant owner changed. New
non-gameplay ending-entry frames must tick fade exactly once, since PlayerSim
step will not run; trigger frame uses current existing player tick. Cache frame
action so setting requested state cannot skip trigger-frame player/objects.
Phase3 ignores physical keys, including START/SELECT; preserve normal inputs
for other phases. Consume camera's authoritative boss_fight_phase; no replay
injection. Sequence state must be explicit pending downstream event; root next
ports $6800F body. Preserve renderer seams; palette bytes are authoritative
native state outputs for renderer, no pixel-completion claim in this ticket.

Tests independently derived literal raw input encodings/boundaries/high-byte
wrap, repeated A pressed at160/161, other phases preserve dirty input, source
palette bytes and high fade nonzero/zero, next-frame dispatch, actual shared
runtime frame head -> player and object step on trigger frame -> subsequent
ending entry, both loop call sites use same helper. Run fullcore/runtime and
meaningful fail/restore sensitivity; coordinate test lane with root (form3
source correction owns it now). No commits/sweep/queue/inventory edits.
If runtime integration needs another substantive source contract, report it.
