//! ROM `$58BBA` (anim 41) / `$58BCB` (anim 40): a vertical moving platform.
//!
//! State 0 arms contact 32, faces Mario by Y (`$5B407`) and stores the
//! half-span (`$2C` for anim 41, `$14` for anim 40). States 1/2 move one pixel
//! up/down for `span` frames; states 3/4 then alternate for `2*span` frames
//! (`state ^= 7`). `$3D10`/`$3CDF` carry a rider (`$FFE7` bit 3) by the same
//! pixel.
use crate::actor::{Actor, ContactKind};
use std::cmp::Ordering;

/// The entry's half-span immediate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Kind {
    pub(super) span: u8,
}

/// ROM `$58BC6`: `ld a,44` (anim 41).
pub(super) const ANIM_41: Kind = Kind { span: 44 };

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Moved { carry_dy: Option<i16> },
}

pub(super) fn update(actor: &mut Actor, kind: Kind, player_y: u16) -> Result<Effect, Error> {
    match actor.state {
        // $58BBF: contact 32, $5B407 (face by Y), span.
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            let relation = player_y.cmp(&actor.position.y);
            actor.state = if relation == Ordering::Greater { 2 } else { 1 };
            actor.motion_leg = kind.span;
            Ok(Effect::Initialized { facing: relation })
        }
        // $58BE6: rise for `span` frames, then switch to the long descent.
        1 => {
            let (carry, done) = step(actor, -1, actor.motion_leg);
            if done {
                actor.state = 4;
            }
            Ok(Effect::Moved { carry_dy: carry })
        }
        // $58C00: descend for `span` frames, then switch to the long ascent.
        2 => {
            let (carry, done) = step(actor, 1, actor.motion_leg);
            if done {
                actor.state = 3;
            }
            Ok(Effect::Moved { carry_dy: carry })
        }
        // $58C1A: long ascent; `state ^= 7` flips 3 <-> 4.
        3 => {
            let cap = actor.motion_leg.wrapping_mul(2);
            let (carry, done) = step(actor, -1, cap);
            if done {
                actor.state ^= 7;
            }
            Ok(Effect::Moved { carry_dy: carry })
        }
        // $58C37: long descent, joining the same counter.
        4 => {
            let cap = actor.motion_leg.wrapping_mul(2);
            let (carry, done) = step(actor, 1, cap);
            if done {
                actor.state ^= 7;
            }
            Ok(Effect::Moved { carry_dy: carry })
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// One movement frame: move, carry a rider, advance the counter, and report
/// whether the counter reached its cap (which the caller resets and uses to
/// change state).
fn step(actor: &mut Actor, dy: i16, cap: u8) -> (Option<i16>, bool) {
    actor.position.move_y(dy);
    let carry = actor.interactions.stood_on().then_some(dy);
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    let done = actor.motion_phase == cap;
    if done {
        actor.motion_phase = 0;
    }
    (carry, done)
}
