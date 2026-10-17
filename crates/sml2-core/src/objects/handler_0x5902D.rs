//! Eight-way emergence followed by timed activation and player tracking.
use crate::actor::{
    Actor, ContactKind, FlashFlags, FrameTick, Position, SpriteFlipFlags, VerticalTrackingError,
};
use std::cmp::Ordering;

// Signed ROM data at $59082 and $5908A, respectively.
const EMERGENCE_Y: [i8; 8] = [4, -20, -32, 40, 28, 16, -44, -8];
const EMERGENCE_X: [i8; 8] = [0, -16, 24, -24, 32, -40, -16, 40];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    VerticalTracking(VerticalTrackingError),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Despawn,
}

pub(super) trait World {
    fn player_position(&self) -> Position;
    fn divider(&self) -> u8;
    fn set_facing(&mut self, relation: Ordering);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::NONE;
            actor.sprite_flip = SpriteFlipFlags::from_bits(0);
            actor.flash = FlashFlags::from_bits(16);
            let Some(direction) = actor.allocate_direction(world.divider()) else {
                return Ok(Effect::Despawn);
            };
            actor
                .position
                .move_y(i16::from(EMERGENCE_Y[usize::from(direction)]));
            actor
                .position
                .move_x(i16::from(EMERGENCE_X[usize::from(direction)]));
            actor.state = 3;
            actor.timer = 48;
        }
        1 | 2 => {
            actor.animate_two_frame_toggle(frame, 1);
            if frame.every(4) {
                let player = world.player_position();
                world.set_facing(player.y.cmp(&actor.position.y));
                let magnitude = actor
                    .track_player_y(player.y)
                    .map_err(Error::VerticalTracking)?;
                let horizontal_speed = i16::from(3u8.wrapping_sub(magnitude));
                actor.position.move_x(if actor.state == 1 {
                    -horizontal_speed
                } else {
                    horizontal_speed
                });
                world.set_facing(actor.face_toward(player.x));
            }
        }
        3 => {
            actor.animate_two_frame_toggle(frame, 8);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.state = 4;
                actor.contact_kind = ContactKind::from_raw(82);
                actor.sprite = 110;
                actor.timer = 24;
            }
        }
        4 => {
            actor.animate_two_frame_toggle(frame, 4);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                world.set_facing(actor.face_toward(world.player_position().x));
                actor.motion_leg = 8;
            }
        }
        state => return Err(Error::InvalidState(state)),
    }
    Ok(Effect::Continue)
}
