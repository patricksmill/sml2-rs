//! ROM `$596D9` (anim 81): faces Mario, spawns anim 82, then cools down.
use crate::actor::{Actor, ContactKind, SpriteFlipFlags};
use std::cmp::Ordering;

pub(super) const CONTACT: u8 = 77;
pub(super) const SPAWN_ANIM: u8 = 82;
pub(super) const SPAWN_SPRITE: u8 = 44;
pub(super) const SPAWN_SPRITE_SCRATCH: u8 = 43;
pub(super) const POST_SPAWN_TIMER: u8 = 16;
pub(super) const COOLDOWN_TIMER: u8 = 96;
pub(super) const FACE_SPRITE: u8 = 42;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Waiting,
    SpawnChild {
        anim: u8,
        sprite: u8,
        flip: SpriteFlipFlags,
    },
    Refaced { facing: Ordering },
}

pub(super) fn update(actor: &mut Actor, player_x: u16) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            let facing = actor.face_toward(player_x);
            Ok(Effect::Initialized { facing })
        }
        1 | 2 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(Effect::Waiting);
            }
            actor.sprite = SPAWN_SPRITE_SCRATCH;
            let flip = actor.sprite_flip;
            actor.sprite = SPAWN_SPRITE;
            actor.state = 3;
            actor.timer = POST_SPAWN_TIMER;
            Ok(Effect::SpawnChild {
                anim: SPAWN_ANIM,
                sprite: SPAWN_SPRITE,
                flip,
            })
        }
        3 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Waiting);
            }
            actor.sprite = FACE_SPRITE;
            let facing = actor.face_toward(player_x);
            actor.timer = COOLDOWN_TIMER;
            Ok(Effect::Refaced { facing })
        }
        state => Err(Error::InvalidState(state)),
    }
}
