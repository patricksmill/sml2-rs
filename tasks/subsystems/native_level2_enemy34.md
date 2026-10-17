# Native anim34 diagonal tracker, ROM58B43

Single COMPLETE tasks/handlers/0x58B43.md. Own new nativehandler_0x58B43.rs,
explicit adapter, optional siblingtests and one dispatcharm/modregistration.
Read PORTING/nativeguide, exactpacket, Actor APIs, staging FALL_SPEED literal
only, current589A6 adapter normalstepfixture and relevantObjectSim API ranges.
Use upcoming Actor.animate_sprite_delta_cycle(frame,period,deltas) from parallel
58A6F prerequisite; don'timplementduplicateanimation. It uses increment-phase,
equality-onlyreset attablelen, then signedwrappingdelta with explicitbadindex
error. Nativeconst [-2,1,1] source5B5D3 independentlyROMchecked. Domain context
FrameTick and MarioRefX/Y. No tilequeries; preserveoldprobes/flags in adapter.

CRITICAL source physical/control rules:
- State0 shared Actor.face_toward sets1/2,FULLflip0/32 and source globalfacing
  FF/0/1. contact77. Preserve allcounter/timer/positionbytes.
- Timerpositive calls three-delta animate on EVENframes, THEN decrementtimer;
  if timerstillnonzero returns. If timerreaches0 continues sameframe.
- Then UNCONDITIONAL three-delta animation on everyframe (period1), BEFORE odd
  frame earlyreturn. Thus an EVEN timer1 expiry animates TWICE; oddexpiry once.
- Movementonlyeven. FALL_SPEED currentphase then equality-onlyadvancecap21
  controls HORIZONTAL distance (NOTverticalgravity): X-=sample state1 or X+=
  sample state2. Vertical distance is ALWAYS1: downifmotion_leg==0,upotherwise.
- WhenNEWmotionphase==21, clearphase0, re-face MarioX using updatedposition,
  then verticaltargethelper5B41A overwrites globalfacing from unsigned fullY
  comparison. Setmotion_leg0(down) if playerRefY>updatedactorY OR equality
  with Ylow==1; otherwise1(up). Equality quirkybecause returnedA=playerYlow,
  followed DEC A. Do not simplify equality to a fixed side or use only global
  facing. Then timer40. Sprite/animationphase untouched by this retargettail.
- Unsupportedsourcestate/profile indices explicitlydiagnostic, not reset/clamp.

Tests whole32bytes everyprefix preservedincludingzeros/unknowns, bothdirections,
timer40/even/odd/expiry-doubleanimation, animationcounter255wrap-2 andstrict3
reset, spritewrap, motionsample21cap andtotalarc, verticalstepconstant1 and
positionwrap, targetfullX/Y plus equallowY0/1/2/255, finalglobalfacingY overrides
horizontal, no-probe flags/stpx/py unchanged, actual anim34 ObjectSim.step/dead
freeze. Meaningful source-mutants then restoredfullcore. Rootcontrols lane:
prepare UNREGISTERED modules while58A6F ownsactor/helper/tests; no compilations,
mod/dispatch changes or mutations untilroot releases. No commit/sweep/metadata.
