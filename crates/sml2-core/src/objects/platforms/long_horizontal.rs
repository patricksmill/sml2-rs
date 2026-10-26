//! ROM `$58C41` (anim 43): horizontal platform like `$58C52`, but the leg
//! length is 60 in `$FFE8` (`motion_leg`) and state 0 omits flash 16.
//!
//! States 1/2 run the short legs; 3/4 alternate at `2 * span` (`state ^= 7`).
//! `$3CF1`/`$3CC0` carry a stood-on rider by the same pixel.
use crate::actor::{Actor, ContactKind};
use std::cmp::Ordering;

/// ROM `$58C4D`: `ld a,60`.
pub(super) const SPAN: u8 = 60;

pub(super) use super::patrol_motion::Error;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Moved { carry_dx: Option<i16> },
}

pub(super) fn update(actor: &mut Actor, player_x: u16) -> Result<Effect, Error> {
    match actor.state {
        // $58C46: contact 32, face by X, span in $FFE8.
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            let relation = actor.face_toward(player_x);
            actor.motion_leg = SPAN;
            Ok(Effect::Initialized { facing: relation })
        }
        _ => super::patrol_motion::patrol(actor).map(|carry_dx| Effect::Moved { carry_dx }),
    }
}
