# Native level2 handler58A6F (anim33)

One handler/source COMPLETE tasks/handlers/0x58A6F.md. Own new named native
handler_0x58A6F.rs + explicit adapter + optional siblingtests; mod/dispatchonly
registrations/one arm. Additional native prerequisite allowed: new focused
actor_sprite_deltas.rs (+tests) and actor.rs mod registration only, providing
Actor.animate_sprite_delta_cycle(frame,period,deltas:&[i8])->Result for source
increment-phase, equality-only reset at tablelen, currentnewphase table lookup,
wrapping sprite-add. Expose existing AnimationFrameError; no clamp/panic for
unsupported phase. Do NOT substitute existing animate_four_frame_cycle: that
method differs on phase255->0 (it increments sprite, source delta table uses-3).
Source5B5E6 table at5B603 independently ROMchecked [FD,01,01,01]=[-3,1,1,1].
Native helper captures reusable sequence math; consumer passes exactconst data.
Do not modify existing Actor animation semantics. Shared FlashFlags method
flash_palette_on_tick(frame,2) exactly implements5B68B. Actor.face_toward exists.
Shared terrain_probes wall_blocked(pos,right,7),projection/flags used inadapter.
Context supplies existing native ObjectSim.div and FrameTick; no hardwareemulator.

Bounded reads authorized: PORTING/nativeguide, exactpacket, Actor method/type
and module definitions, actor_sprite_sequence.rs for contractcontrast only,
terrain_probes API, previous589A6 adapter signature/currentengine fixture,
staging arraydefinitions if needed, LevelLayout/ObjectTables relevant APIs.
No whole engine/main.asm/queue/captures/DESIGN.

Source0 (and retarget tail) faceplayer, add2 tostate ->3/4, sprite68, clearflash,
contact77; NO other resets. States3/4: togglesprite everyevenframe and increment
travel counter everyevenframe; onlynewcounter40 resetscounter+animation_frame,
state-=2,timer36. Preserve above40 and wrap.
States1/2: paletteflash even BEFOREtimer; timerpositive countdown returns if
stillnonzero. On reaching0 sprite64/contact76, then continue. If timeralready0
those fields remain. Four-delta animation even; oddreturns beforemovement/DIV.
Even divider&63==0 retargets to sourceinitialtail (afteranimation), notmovement.
Else X1 movement THEN reach7 wallprobe; blocked flips state1/2 and horizontal
spriteflip. No ground/fall. Prefix/retarget ordering observable in slot bytes.

Tests exact32-byte fixtures allstates, dormant/retarget preservedfields and
facingglobal FF/0/1, even/odd animation/timer36->0 behavior, strict40countreset,
phase255->0 gives delta-3, spritewrap, DIV0/63/64/128/192/255, wallpostmovement,
probeoutputs/no-probe preservation, unsupportedstate/profilediagnostic,
realanim33ObjectSim.step/deathgate. Nativehelpertestdelays/cap and independent
literalROM outputs. Source-sensitive mutations thenrestorefullcore, no sweep
untillevel2package complete. Rootacceptance/metadata/commit.
