//! ROM `$5B177` (mode B anim 91): a bobbing flyer.
//!
//! Every call blinks the sprite on every fourth frame. State 0 arms contact
//! 77, flash 16, and faces Mario. States 1 and 2 move two pixels on even
//! frames, then rise on `$749E` or fall on `$7488`, each sample plus one.
//! Phase 21 clears the phase and flips the vertical direction.
use super::staging::{FALL_SPEED, HOVER_SPEED};
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};
use std::cmp::Ordering;

const PHASE_CAP: u8 = 21;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    actor.animate_two_frame_toggle(frame, 4);
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(77);
            actor.flash = FlashFlags::from_bits(16);
            let facing = actor.face_toward(player_x);
            Ok(Effect::Initialized { facing })
        }
        1 => {
            cruise(actor, frame, -2);
            Ok(Effect::Continue)
        }
        2 => {
            cruise(actor, frame, 2);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn cruise(actor: &mut Actor, frame: FrameTick, dx: i16) {
    if !frame.every(2) {
        return;
    }
    actor.position.move_x(dx);
    let rising = actor.hit_reaction == 0;
    let table: &[u8] = if rising { &HOVER_SPEED } else { &FALL_SPEED };
    let step = i16::from(sample(actor, table).wrapping_add(1));
    actor.position.move_y(if rising { -step } else { step });
    if actor.motion_phase != PHASE_CAP {
        return;
    }
    actor.motion_phase = 0;
    actor.hit_reaction ^= 1;
}

fn sample(actor: &mut Actor, table: &[u8]) -> u8 {
    let step = table
        .get(usize::from(actor.motion_phase))
        .copied()
        .unwrap_or(0);
    if actor.motion_phase != PHASE_CAP {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    step
}
