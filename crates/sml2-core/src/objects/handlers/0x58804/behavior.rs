//! ROM `$58804` (mode B anim 24): proximity arms contact 136, sprite counts to
//! `$88`, then floats up until touched and removes via `$5B2D7`.
use crate::actor::{Actor, ContactKind, FrameTick};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Despawn,
}

pub(super) trait World {
    /// ROM `$5B4B4`: eight-bit horizontal distance to Mario.
    fn mario_distance_x(&self, actor_x: u16) -> u8;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &impl World,
) -> Result<Effect, Error> {
    match actor.state {
        // $5880F: advance into the proximity wait.
        0 => actor.state = 1,
        // $58814: within $30 px, arm contact 136 and enter the sprite ramp.
        1 => {
            if world.mario_distance_x(actor.position.x) < 0x30 {
                actor.state = 2;
                actor.contact_kind = ContactKind::from_raw(136);
            }
        }
        // $58823: every fourth `$FFF0` frame, count the sprite up to $88.
        2 => {
            if frame.every(4) {
                actor.sprite = actor.sprite.wrapping_add(1);
                if actor.sprite == 0x88 {
                    actor.state = 3;
                }
            }
        }
        // $58835: touched → remove; else `$5B511` + one-pixel rise (`$3D2D`).
        3 => {
            if actor.interactions.touched() {
                return Ok(Effect::Despawn);
            }
            actor.animate_two_frame_toggle(frame, 8);
            actor.position.move_y(-1);
        }
        state => return Err(Error::InvalidState(state)),
    }
    Ok(Effect::Continue)
}
