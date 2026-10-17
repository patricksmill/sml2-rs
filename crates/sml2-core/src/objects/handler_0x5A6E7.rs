//! ROM `$5A6E7` (mode B anim 74): a ceiling crawler.
//!
//! Every call toggles the sprite every fourth frame (`$5B51A`). State 0 arms
//! contact 72 and faces Mario. States 1/2 then step left or right and turn,
//! with `state ^= 3`, when the ceiling probe 18 pixels up and 16 pixels ahead
//! finds open air.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Continue,
}

pub(super) trait World {
    /// `$83EC` / `$8403`: true when the ceiling tile is still solid.
    fn ceiling_present(&mut self, actor: &Actor, moving_left: bool) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    actor.animate_two_frame_toggle(frame, 4);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(72);
        return Ok(Effect::Initialized {
            facing: actor.face_toward(player_x),
        });
    }
    let left = match actor.state {
        1 => true,
        2 => false,
        state => return Err(Error::InvalidState(state)),
    };
    actor.position.move_x(if left { -1 } else { 1 });
    if !world.ceiling_present(actor, left) {
        actor.sprite_flip.toggle_horizontal();
        actor.state ^= 3;
    }
    Ok(Effect::Continue)
}
