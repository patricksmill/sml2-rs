//! ROM `$5A816` (mode B anim 78): the cannon ant's projectile.
//!
//! State 0 arms type 73 and selects a course from flip bit 5: the diagonal
//! course (state 1/2) when flip bit 0 is set (it clears that bit), otherwise
//! the straight course (state 3/4). State 1/2 fly up one pixel per frame
//! while moving sideways; state 3/4 move two pixels sideways with no Y step.
use crate::actor::{Actor, ContactKind, SpriteFlipFlags};

const TYPE: u8 = 73;
const FLIP_BIT: u8 = 0x01;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

pub(super) fn update(actor: &mut Actor) -> Result<(), Error> {
    match actor.state {
        // $5A81B: type 73, $3CB1 picks state from flip bit 5, then the
        // odd flip bit keeps the diagonal course; otherwise state += 2.
        0 => {
            actor.contact_kind = ContactKind::from_raw(TYPE);
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            if actor.sprite_flip.bits() & FLIP_BIT != 0 {
                actor.sprite_flip =
                    SpriteFlipFlags::from_bits(actor.sprite_flip.bits() & !FLIP_BIT);
            } else {
                actor.state = actor.state.wrapping_add(2);
            }
            Ok(())
        }
        // $5A83E: b=1, X -= 1, then b=1, Y -= 1.
        1 => {
            actor.position.move_x(-1);
            actor.position.move_y(-1);
            Ok(())
        }
        // $5A845: b=1, X += 1, then Y -= 1.
        2 => {
            actor.position.move_x(1);
            actor.position.move_y(-1);
            Ok(())
        }
        // $5A850: b=2, X -= 2.
        3 => {
            actor.position.move_x(-2);
            Ok(())
        }
        // $5A856: b=2, X += 2.
        4 => {
            actor.position.move_x(2);
            Ok(())
        }
        state => Err(Error::InvalidState(state)),
    }
}
