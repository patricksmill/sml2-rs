//! ROM `$58C52` (anim 56): a horizontal moving platform.
//!
//! State 0 arms contact 32, flash 16, faces Mario by X (`$5B3C6`) and stores the
//! half-span 36. States 1/2 run the short legs; states 3/4 alternate at
//! `2*span` (`state ^= 7`). `$3CF1`/`$3CC0` carry a rider (`$FFE7` bit 3) by
//! the same pixel. `$5B51A` toggles the sprite every fourth frame.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};
use std::cmp::Ordering;

/// ROM `$58C65`: `ld a,36`.
pub(super) const SPAN: u8 = 36;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Moved { carry_dx: Option<i16> },
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    actor.animate_two_frame_toggle(frame, 4); // $5B51A, before the dispatch
    match actor.state {
        // $58C5A: contact 32, flash 16, face by X, span.
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            actor.flash = FlashFlags::from_bits(16);
            let relation = actor.face_toward(player_x);
            actor.motion_leg = SPAN;
            Ok(Effect::Initialized { facing: relation })
        }
        // $58C85: short left leg, then the long right leg.
        1 => {
            let (carry, done) = step(actor, -1, actor.motion_leg);
            if done {
                actor.state = 4;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        // $58C9F: short right leg, then the long left leg.
        2 => {
            let (carry, done) = step(actor, 1, actor.motion_leg);
            if done {
                actor.state = 3;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        // $58CB9: long left leg; `state ^= 7` flips 3 <-> 4.
        3 => {
            let cap = actor.motion_leg.wrapping_mul(2);
            let (carry, done) = step(actor, -1, cap);
            if done {
                actor.state ^= 7;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        // $58CD6: long right leg, joining the same counter.
        4 => {
            let cap = actor.motion_leg.wrapping_mul(2);
            let (carry, done) = step(actor, 1, cap);
            if done {
                actor.state ^= 7;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn step(actor: &mut Actor, dx: i16, cap: u8) -> (Option<i16>, bool) {
    actor.position.move_x(dx);
    let carry = actor.interactions.stood_on().then_some(dx);
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    let done = actor.motion_phase == cap;
    if done {
        actor.motion_phase = 0;
    }
    (carry, done)
}
