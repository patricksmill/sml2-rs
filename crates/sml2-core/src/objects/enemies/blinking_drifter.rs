//! ROM `$59AAD` (anim 87): blinking sprite cadence and phased vertical drift.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 72;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
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
            let phase = actor.motion_leg & 3;
            let dy: i16 = match phase {
                0 | 1 => 1,
                _ => -1,
            };
            actor.position.move_y(dy);
            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            if actor.motion_phase == 12 {
                actor.motion_phase = 0;
                actor.motion_leg = actor.motion_leg.wrapping_add(1) & 3;
            }
            Ok(Effect::Moved { dx, dy })
        }
        state => Err(Error::InvalidState(state)),
    }
}
