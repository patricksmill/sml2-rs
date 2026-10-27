//! ROM `$594FB` (mode A anim 69): a timed fuse that blinks, then bursts.
//!
//! State 0 arms state 1 and a 48-frame fuse. State 1 blinks the sprite bit and
//! the flash palette every other frame, then arms state 2 with contact 72,
//! sprite 130, flash 16 and sfx1 13. State 2 cycles the three sprite deltas
//! every eighth frame and removes itself at sprite `$84`.
use crate::actor::{Actor, AnimationFrameError, ContactKind, FlashFlags, FrameTick};

const FUSE: u8 = 48;
const BLAST_SPRITE: u8 = 130;
const BLAST_CONTACT: u8 = 72;
const LIMIT: u8 = 0x84;
const SPRITE_DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Animation(AnimationFrameError),
}
impl From<AnimationFrameError> for Error {
    fn from(error: AnimationFrameError) -> Self {
        Self::Animation(error)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Armed,
    Remove,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(0);
            actor.state = 1;
            actor.timer = FUSE;
            Ok(Effect::Continue)
        }
        // $59512: blink, count the fuse, then arm the blast.
        1 => {
            if frame.every(2) {
                actor.sprite ^= 1;
                actor.flash.toggle_palette(frame);
            }
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            actor.state = 2;
            actor.flash = FlashFlags::from_bits(16);
            actor.contact_kind = ContactKind::from_raw(BLAST_CONTACT);
            actor.sprite = BLAST_SPRITE;
            Ok(Effect::Armed)
        }
        // $59534: cycle to $84, then remove.
        2 => {
            actor.animate_sprite_delta_cycle(frame, 8, &SPRITE_DELTAS)?;
            if actor.sprite != LIMIT {
                return Ok(Effect::Continue);
            }
            Ok(Effect::Remove)
        }
        state => Err(Error::InvalidState(state)),
    }
}
