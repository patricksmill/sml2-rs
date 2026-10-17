# Shared native terrain probes for level2 object handlers

One prerequisite: common tile probes8358,8462,8510, with exact COMPLETE generated
packets tasks/handlers/0x08358.md,0x08462.md,0x08510.md. Existing58848 uses same
wall algorithm at reach7 (source843C/84EA), new589A6 at reach11. Source58848
packet may be read ONLY its8358/843C/84EA/8583/8590/859D/1F2F instructions to
verify reuse. Read PORTING/native guide, LevelLayout get/from_bytes, Actor
Position, existing58848 adapter and its tests. No whole engine/main.asm.

Touch ONLY new objects/terrain_probes.rs (+optional sibling tests), existing
handler_0x58848_adapter.rs to consume shared API, mod.rs registration ONLY
(coordinate root before registration; handler589A6 worker alsoownsregistrations).
Do not edit existing58848 native handler or tests. Keep raw slot conversion
solely Actor methods. No runtime/queue/inventory/captures/commits.

API agreed with dependent worker:
TerrainProbes::new(&LevelLayout,initial:Position)
.wall_blocked(actor_position:Position,right:bool,reach:u16)->bool
.grounded(actor_position:Position)->bool
.position()->Position (final compatibility probe projection)
.legacy_move_flags()->Option<u8> (None if no probe has run)

Use native Position arithmetic, descriptive ProbeContact kind/result internally,
source flags conversion in one explicit compatibility method, not a bytebacked
RAM/register model. Wall: X+/-reach,Y-11 thenY-5, blocked if EITHER tile<56;
clear only when BOTH>=56. Right legacy flags17 blocked/16clear,left34/32.
Ground: initialize resultgrounded; write only low probeY beforealignmenttest.
If actorY low&12!=0, preserve earlier probeY high and priorprobeX, false/flags128.
Else set whole probeY+X, grounded iff tile<72,flags136true/128false. Tile lookup
uses layout.get((x>>4)&255,(y>>4)&255).unwrap_or(0), existing sourcecontract.
Retain exact scratch projection only at this shared adapter seam; native handler
receives named clear/blocked outcomes. No duplicated probe arithmetic perenemy.

Existing58848 adapter Inputs owns TerrainProbes plus sim; delegate its World
methods (reach7), transfer flags when Some, retain exact no-probe flags/state,
and write projection into original staging onlyat final Actor boundary. New
589A6 adapter willreuse reach11. Root will review integration beforedispatch.

Tests exact point/flag values for bothreach7/11 bothsides, mixedtile55/56 order,
72groundthreshold, alignmentallow0..3, high-byte-retention e.gactorY0x104 after
wallY0xFF, coordinatewrap and no-probe outputs. Run focused shared+existing58848
regressions. Wait root before mutation/fullcore lane; other worker domain tests
may compile concurrently but willholdmodregistration. Source-sensitive wall,
reach,alignment/highbyte mutations mustfail thenrestore; root gates/commit.
