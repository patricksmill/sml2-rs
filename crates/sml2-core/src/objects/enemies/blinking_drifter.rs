//! ROM `$59AAD` (anim 87): blinking sprite cadence and phased vertical drift.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 72;

/// ROM `$7488`, selected by `$5B445` with cap twelve: the accelerating legs.
const SPEED_UP_7488: [u8; 13] = [0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2];
/// ROM `$74A7`, selected by `$5B46C` with cap twelve: the decelerating legs.
const SLOW_DOWN_74A7: [u8; 13] = [3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Profile(crate::actor::MotionProfileError),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Moved { dx: i16, dy: i16 },
    Waiting,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.face_toward(player_x);
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            Ok(Effect::Initialized)
        }
        1 | 2 => {
            if frame.every(8) {
                actor.sprite ^= 1;
            }
            if !frame.every(2) {
                return Ok(Effect::Waiting);
            }
            let dx: i16 = if actor.state == 1 { -1 } else { 1 };
            actor.position.move_x(dx);
            // `$59AD6`: a 32px vertical bob in four eased legs of 16px each —
            // up slowing, down speeding, down slowing, up speeding.
            let (profile, up) = match actor.motion_leg {
                1 => (&SPEED_UP_7488, false),
                2 => (&SLOW_DOWN_74A7, false),
                3 => (&SPEED_UP_7488, true),
                _ => (&SLOW_DOWN_74A7, true),
            };
            let step = actor.motion_step(profile, 12).map_err(Error::Profile)?;
            let dy = if up {
                -i16::from(step)
            } else {
                i16::from(step)
            };
            actor.position.move_y(dy);
            // `$59AFF`: the leg ends when the phase reaches the cap.
            if actor.motion_phase == 12 {
                actor.motion_phase = 0;
                actor.motion_leg = actor.motion_leg.wrapping_add(1) & 3;
            }
            Ok(Effect::Moved { dx, dy })
        }
        state => Err(Error::InvalidState(state)),
    }
}
