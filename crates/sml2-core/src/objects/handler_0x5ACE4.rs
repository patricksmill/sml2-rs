//! ROM `$5ACE4` (mode B anims 104 and 105): a flashing mine.
//!
//! State 0 arms a 32-frame timer, flash 16, and state 2. State 2 marks the
//! object done while timer bits 1 or 2 are set, then counts down. At zero it
//! arms contact 80, state 1, and sound 6. State 1 bobs every fourth frame,
//! slides two pixels left, and respawns once its screen X reaches `$B0`.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Flicker,
    Launch,
    Respawn,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.timer = 32;
            actor.state = 2;
            actor.flash = FlashFlags::from_bits(16);
            Ok(Effect::Continue)
        }
        1 => {
            bob(actor, frame);
            actor.position.move_x(-2);
            if actor.screen.x >= 0xB0 {
                Ok(Effect::Respawn)
            } else {
                Ok(Effect::Continue)
            }
        }
        2 => {
            let flicker = actor.timer & 0x06 != 0;
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(if flicker {
                    Effect::Flicker
                } else {
                    Effect::Continue
                });
            }
            actor.state = 1;
            actor.contact_kind = ContactKind::from_raw(80);
            Ok(Effect::Launch)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn bob(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(4) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 3 {
        actor.animation_frame = 0;
    }
    let delta = DELTAS[usize::from(actor.animation_frame)];
    actor.sprite = actor.sprite.wrapping_add_signed(delta);
}
