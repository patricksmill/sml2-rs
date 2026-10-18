//! ROM `$5A11E` (mode B anim 38): a drifting projectile spawned by `$5A0D5`.
//!
//! State 0 arms contact 72, picks the direction from flip bit 5, lifts 15 px and
//! shifts 11 px sideways. States 1/2 drift one pixel every fourth frame while the
//! sprite climbs one step per move; when it reaches `$49` the state advances to
//! the faster states 3/4, which move two pixels every frame.
use crate::actor::{Actor, ContactKind, FrameTick};

const CONTACT: u8 = 72;
const SPRITE_LIMIT: u8 = 0x49;
const LIFT: u8 = 15;
const SIDE: u8 = 11;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<(), Error> {
    match actor.state {
        // $5A123: contact 72, direction, lift 15, then the 11-px side step.
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            actor.position.move_y(-i16::from(LIFT));
            let dx = if actor.state == 1 {
                -i16::from(SIDE)
            } else {
                i16::from(SIDE)
            };
            actor.position.move_x(dx);
            Ok(())
        }
        // $5A148: drift left every fourth frame.
        1 => {
            if !frame.every(4) {
                return Ok(());
            }
            actor.position.move_x(-1);
            accelerate(actor);
            Ok(())
        }
        // $5A154: drift right every fourth frame.
        2 => {
            if !frame.every(4) {
                return Ok(());
            }
            actor.position.move_x(1);
            accelerate(actor);
            Ok(())
        }
        // $5A16D: fast left.
        3 => {
            actor.position.move_x(-2);
            Ok(())
        }
        // $5A173: fast right.
        4 => {
            actor.position.move_x(2);
            Ok(())
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// `$5A15E`: the sprite climbs one step per drift move; `$49` switches to the
/// fast states (1 -> 3, 2 -> 4).
fn accelerate(actor: &mut Actor) {
    actor.sprite = actor.sprite.wrapping_add(1);
    if actor.sprite == SPRITE_LIMIT {
        actor.state = actor.state.wrapping_add(2);
    }
}
