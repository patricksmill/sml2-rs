//! Alternating walking and stationary enemy, ROM $58A6F (animation 33).
use crate::actor::{Actor, AnimationFrameError, ContactKind, FrameTick, Position};

// ROM $5B603: phase is incremented before selecting the sprite displacement.
const WALK_SPRITE_DELTAS: [i8; 4] = [-3, 1, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState,
    Animation(AnimationFrameError),
}

pub(super) trait World {
    fn player_x(&self) -> u16;
    fn face_player(&mut self, position: Position);
    fn divider(&self) -> u8;
    fn wall_blocked(&mut self, position: Position, right: bool) -> bool;
}

fn retarget(actor: &mut Actor, world: &mut impl World) {
    world.face_player(actor.position);
    actor.face_toward(world.player_x());
    actor.state += 2;
    actor.sprite = 68;
    actor.flash.clear();
    actor.contact_kind = ContactKind::from_raw(77);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<(), Error> {
    match actor.state {
        0 => retarget(actor, world),
        1 | 2 => {
            actor.flash.flash_palette_on_tick(frame, 2);
            if actor.timer != 0 {
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(());
                }
                actor.sprite = 64;
                actor.contact_kind = ContactKind::from_raw(76);
            }
            actor
                .animate_sprite_delta_cycle(frame, 2, &WALK_SPRITE_DELTAS)
                .map_err(Error::Animation)?;
            if !frame.every(2) {
                return Ok(());
            }
            if world.divider() & 63 == 0 {
                retarget(actor, world);
            } else {
                let right = actor.state == 2;
                actor.position.move_x(if right { 1 } else { -1 });
                if world.wall_blocked(actor.position, right) {
                    actor.state = if right { 1 } else { 2 };
                    actor.sprite_flip.toggle_horizontal();
                }
            }
        }
        3 | 4 => {
            actor.animate_two_frame_toggle(frame, 2);
            if frame.every(2) {
                actor.travel_direction =
                    crate::actor::Facing::from_raw(actor.travel_direction.raw().wrapping_add(1));
                if actor.travel_direction.raw() == 40 {
                    actor.travel_direction = crate::actor::Facing::from_raw(0);
                    actor.animation_frame = 0;
                    actor.state -= 2;
                    actor.timer = 36;
                }
            }
        }
        _ => return Err(Error::InvalidState),
    }
    Ok(())
}
