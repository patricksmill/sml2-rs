//! ROM `$5A739` (mode B anim 76): a ground crawler.
//!
//! Every call toggles the sprite every fourth frame (`$5B51A`). State 0 arms
//! contact 72 and faces Mario. States 1/2 step left or right and turn, with
//! `state ^= 3`, when the ground probe 16 pixels ahead is open or the Y
//! position is not 16-pixel aligned.
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
    /// `$8318` / `$8338`: true when the forward ground tile is solid.
    fn ground_present(&mut self, actor: &Actor, moving_left: bool) -> bool;
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
    if !world.ground_present(actor, left) {
        actor.sprite_flip.toggle_horizontal();
        actor.state ^= 3;
    }
    Ok(Effect::Continue)
}
