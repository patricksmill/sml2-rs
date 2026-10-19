//! Native Wario drop child (anim 28, ROM `$0B0C2`).

use crate::actor::{Actor, FrameTick, MotionProfileError, RiseProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropChildEffects {
    pub sound0: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropChildError {
    UnsupportedState(u8),
    InvalidMotionProfile(MotionProfileError),
}

impl From<MotionProfileError> for DropChildError {
    fn from(err: MotionProfileError) -> Self {
        Self::InvalidMotionProfile(err)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DropChildContext<'a> {
    pub frame: FrameTick,
    pub player_screen_x: u8,
    pub picker_profile: &'a [u8],
}

pub fn update_drop_child(
    actor: &mut Actor,
    context: DropChildContext<'_>,
) -> Result<DropChildEffects, DropChildError> {
    let mut effects = DropChildEffects::default();

    match actor.state {
        0 => {
            // ROM $B0CF: call $3CB1
            // Check bit 5 of sprite_flip (horizontal flip).
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            Ok(effects)
        }
        1 | 2 => {
            // ROM $B0D3 (state 1): ld b, 2; sub from screen.x
            // ROM $B0DA (state 2): ld b, 2; add to screen.x
            if actor.state == 1 {
                actor.screen.x = actor.screen.x.wrapping_sub(2);
            } else {
                actor.screen.x = actor.screen.x.wrapping_add(2);
            }

            // ROM $B0DF: call $B3C0
            actor.animate_four_frame_cycle(context.frame, 4);

            // ROM $B0E2: ldh a, [$FFF0]; and $03; ret nz
            if context.frame.0 & 3 != 0 {
                return Ok(effects);
            }

            // ROM $B0E7: call $B2DB (horizontal distance to player)
            let dist_x = context.player_screen_x.abs_diff(actor.screen.x);
            if dist_x < 8 {
                // ROM $B10F: ld a, 3; ldh [$FFDF], a; xor a; ldh [$FFE0], a; ldh [$FFE8], a; ld a, 6; ld [sSfxRequest0], a; ret
                actor.state = 3;
                actor.motion_phase = 0;
                actor.motion_leg = 0;
                effects.sound0 = Some(6);
                return Ok(effects);
            }

            // ROM $B0EE: ldh a, [$FFE8]; and a; jr nz, $B0FB
            if actor.motion_leg == 0 {
                // ROM $B0F2: call $B1EB (RiseProfile::Low, cap 15)
                // ROM $B0F5: call $B270 (sub from screen.y)
                let step = actor.rise_step(RiseProfile::Low)?;
                actor.screen.move_y(-i16::from(step));
            } else {
                // ROM $B0FB: call $B187 (boss_picker_profile, cap 15)
                // ROM $B0FE: call $B276 (add to screen.y)
                let step = actor.motion_step(context.picker_profile, 15)?;
                actor.screen.move_y(i16::from(step));
            }

            // ROM $B101: ldh a, [$FFE0]; cp c; ret nz (c is 15 for both legs)
            if actor.motion_phase == 15 {
                actor.motion_phase = 0;
                actor.motion_leg ^= 1;
            }

            Ok(effects)
        }
        3 => {
            // ROM $B11E: call $B3C0
            actor.animate_four_frame_cycle(context.frame, 4);

            // ROM $B121: ldh a, [$FFE8]; and a; jr nz, $B138
            if actor.motion_leg == 0 {
                // ROM $B125: call $B1E4 (RiseProfile::Short, cap 12)
                // ROM $B128: call $B270 (sub from screen.y)
                let step = actor.rise_step(RiseProfile::Short)?;
                actor.screen.move_y(-i16::from(step));

                // ROM $B12B: ldh a, [$FFE0]; cp c; ret nz (c is 12)
                if actor.motion_phase == 12 {
                    actor.motion_phase = 0;
                    actor.motion_leg = 1;
                }
            } else {
                // ROM $B138: ldh a, [$FFD9]; cp $90; jr nc, $B145
                if actor.screen.y >= 0x90 {
                    // ROM $B145: ld a, 108; ldh [$FFDB], a; ld a, 4; ldh [$FFDF], a; ret
                    actor.sprite = 108;
                    actor.state = 4;
                } else {
                    // ROM $B13D: call $B18F (boss_picker_profile, cap 21)
                    // ROM $B140: call $B276 (add to screen.y)
                    let step = actor.motion_step(context.picker_profile, 21)?;
                    actor.screen.move_y(i16::from(step));
                }
            }

            Ok(effects)
        }
        4 => {
            // ROM $B14D: ret (inert on ground)
            Ok(effects)
        }
        other => Err(DropChildError::UnsupportedState(other)),
    }
}
