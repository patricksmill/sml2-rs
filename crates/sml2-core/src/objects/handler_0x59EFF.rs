//! ROM `$59EFF` (mode B anim 33): waits until Mario is within 48 px, then bobs
//! eight pixels down and up on every fourth frame.
//!
//! State 0 arms flash 16 and contact 72. States 1/2 run only when `$FFF0 & 3`
//! is zero. A nonzero timer counts down; at zero, state 1 steps Y by +8 and
//! advances the sprite until 41, and state 2 steps Y by −8 until sprite 37.
//! Either end reloads an 8-frame timer and toggles with `state ^= 3`.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const REACH: u8 = 0x30;
const STEP: i16 = 8;
const LOW_SPRITE: u8 = 0x25;
const HIGH_SPRITE: u8 = 0x29;
const PAUSE: u8 = 8;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

pub(super) trait World {
    /// `$5B4B4`: eight-bit horizontal distance to Mario.
    fn mario_distance_x(&self, actor_x: u16) -> u8;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &impl World,
) -> Result<(), Error> {
    if actor.state == 0 {
        actor.flash = FlashFlags::from_bits(16);
        actor.contact_kind = ContactKind::from_raw(72);
        if world.mario_distance_x(actor.position.x) < REACH {
            actor.state = 1;
        }
        return Ok(());
    }
    if !frame.every(4) {
        return Ok(());
    }
    match actor.state {
        1 => bob(actor, STEP, HIGH_SPRITE, true),
        2 => bob(actor, -STEP, LOW_SPRITE, false),
        state => Err(Error::InvalidState(state)),
    }
}

fn bob(actor: &mut Actor, dy: i16, limit: u8, rising_sprite: bool) -> Result<(), Error> {
    if actor.timer != 0 {
        actor.timer -= 1;
        return Ok(());
    }
    actor.position.move_y(dy);
    actor.sprite = if rising_sprite {
        actor.sprite.wrapping_add(1)
    } else {
        actor.sprite.wrapping_sub(1)
    };
    if actor.sprite != limit {
        return Ok(());
    }
    actor.timer = PAUSE;
    actor.state ^= 3;
    Ok(())
}
