# Native final-Wario sprite bob prerequisite

One shared animation helper, COMPLETE bank2 ROM `$B35A` brief
`tasks/handlers/0x0B35A.md`, literal table `$B388..$B38B` = FF 01 FF 01.
Read PORTING/native guide, named brief, actor.rs native animation/flags API.
Touch ONLY actor.rs flag setter/module registration (keep below400lines), new
bounded actor_ending_animation.rs with native helper/tests. No other actor or
handler modules. Root already committed Actor sprite alternation methods.

Expose Actor::animate_ending_bob(FrameTick) -> Result<(), descriptive error>.
Every4 frames increment animation_frame wrapping; when NEW frame equals4 reset
it0 and clear horizontal flip. Crucial source quirk: on THAT reset the sprite
lookup index is the REMAINING FULL flip byte, not reset phase. Otherwise index
is new phase. Read signed table delta and add with sprite wrapping; finally
if animation frame3 set horizontal flip. Preserve other flip flags and every
unwritten actor field. Use SpriteFlipFlags::set_horizontal(bool) or equally clear
flag API (add it once), no raw slot indices/register names/scratch in logic.
Do not normalize unexpected flip flags. If computed index outside four literal
entries, return explicit error after source-prefix writes (phase/reset/flip),
leaving sprite unmodified rather than inventing a delta or emulating ROM memory.
This unsupported-data rejection is explicit; normal source states use flip0/32.

Independent ROM literal validation; tests all phases/cadences, all valid reset
lookup indices0..3 selected by preserved low flip bits, overflow phase255,
set/clear horizontal preserve other bits, signed sprite wrap, explicit errors,
entire slot preservation. Mutation fail/restore/fullcore; ask root for lane
(Wario23 currently owns it; rise-profile worker waits next). No commits/sweep/
queue/inventory changes. New helper is prerequisite, not final ending integration.
