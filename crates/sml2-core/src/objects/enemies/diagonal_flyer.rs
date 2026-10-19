//! ROM `$59728` (mode A anim 82): a diagonal flyer.
//!
//! Any state other than 1/2 arms contact 77, faces Mario and requests sfx2 11.
//! States 1/2 cycle the sprite and move one step up and two sideways every
//! other frame.
use crate::actor::{Actor, AnimationFrameError, ContactKind, FrameTick};

const CONTACT: u8 = 77;
const SPRITE_DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
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
    Sfx2,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    match actor.state {
        1 => {
            actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            actor.position.move_x(-2);
            actor.position.move_y(-1);
            Ok(Effect::Continue)
        }
        2 => {
            actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            actor.position.move_x(2);
            actor.position.move_y(-1);
            Ok(Effect::Continue)
        }
        // $59730: contact 77, $3CB1 direction, sfx2 11.
        _ => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            Ok(Effect::Sfx2)
        }
    }
}
