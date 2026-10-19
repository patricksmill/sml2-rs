//! ROM `$5A969` (mode B anim 55): the child shed by the anim-54 splitter.
//!
//! State 0 only promotes to state 1. State 1 follows the parent slot's
//! position (the spawn record points at the parent slot base), wobbles the
//! sprite every even frame, drops 20 px below the parent, and removes itself
//! once the parent slot is no longer active.
use crate::actor::{Actor, Position};

const DROP: u8 = 20;
const SPRITE_DELTAS: [i8; 4] = [-2, 1, -1, 2];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    UnresolvedSource,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Remove,
}

/// Projection of the parent slot's live position/activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Source {
    pub position: Position,
    pub marker: u8,
    pub gone: bool,
}

pub(super) fn update(
    actor: &mut Actor,
    frame_even: bool,
    source: Option<Source>,
) -> Result<Effect, Error> {
    match actor.state {
        // $5A96E: just promote.
        0 => {
            actor.state = 1;
            Ok(Effect::Continue)
        }
        // $5A979: wobble, re-read the parent, drop, then remove when gone.
        1 => {
            let Some(source) = source else {
                return Err(Error::UnresolvedSource);
            };
            if frame_even {
                actor.animation_frame = actor.animation_frame.wrapping_add(1);
                if actor.animation_frame == 4 {
                    actor.animation_frame = 0;
                }
                actor.sprite = actor
                    .sprite
                    .wrapping_add_signed(SPRITE_DELTAS[usize::from(actor.animation_frame)]);
            }
            actor.position = source.position;
            actor.marker = source.marker;
            actor.position.move_y(i16::from(DROP));
            Ok(if source.gone {
                Effect::Remove
            } else {
                Effect::Continue
            })
        }
        state => Err(Error::InvalidState(state)),
    }
}
