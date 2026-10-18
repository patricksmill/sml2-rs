//! ROM `$5A5C7` (mode B anim 69): a parent-tracking child.
//!
//! State 0 arms contact 77, state 1 and a 24-px lift. State 1 cycles the four
//! sprite deltas every fourth frame, copies the parent slot's five source bytes
//! and lifts 24 px again. `$5B4F5` also writes the source-gone byte, but this
//! entry never reads it.
use crate::actor::{Actor, AnimationFrameError, ContactKind, FrameTick, Position};

const CONTACT: u8 = 77;
const LIFT: u8 = 24;
const SPRITE_DELTAS: [i8; 4] = [-2, 1, -1, 2];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Animation(AnimationFrameError),
    UnresolvedSource,
}
impl From<AnimationFrameError> for Error {
    fn from(error: AnimationFrameError) -> Self {
        Self::Animation(error)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Source {
    pub position: Position,
    pub marker: u8,
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    source: Option<Source>,
) -> Result<Effect, Error> {
    match actor.state {
        // $5A5CC
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 1;
            actor.position.move_y(-i16::from(LIFT));
            Ok(Effect::Continue)
        }
        // $5A5E3: follow the parent slot and lift again.
        1 => {
            let Some(source) = source else {
                return Err(Error::UnresolvedSource);
            };
            actor.animate_sprite_delta_cycle(frame, 4, &SPRITE_DELTAS)?;
            actor.position = source.position;
            actor.marker = source.marker;
            actor.position.move_y(-i16::from(LIFT));
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}
