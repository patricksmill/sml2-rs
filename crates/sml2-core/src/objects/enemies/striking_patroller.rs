//! ROM `$5A179` (mode B anim 39): a patrolling enemy that strikes when Mario
//! comes within 32 pixels.
//!
//! States below 3 bob on even frames (`$5B585`). State 0 arms contact 77 and
//! faces Mario. States 1/2 run the shared patrol, then count a timer down to
//! the proximity check. State 3 lowers the sprite from 99 to 97 on every
//! fourth frame. State 4 waits out its timer, raises the sprite back to 99,
//! and faces Mario again with contact 77 and a 64-frame timer.
use super::fallwalker::{self, PatrolWorld};
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

const STRIKE_SPRITE: u8 = 0x63;
const CROUCH_SPRITE: u8 = 0x61;
const STRIKE_CONTACT: u8 = 76;
const PATROL_CONTACT: u8 = 77;
const REACH: u8 = 0x20;
const CROUCH_TIMER: u8 = 32;
const REARM_TIMER: u8 = 64;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Refaced { facing: Ordering },
    Continue,
}

pub(super) trait World: PatrolWorld {
    fn mario_distance_x(&self, actor_x: u16) -> u8;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    if actor.state < 3 {
        let _ = actor.animate_four_step_bob_cadence(frame, 2);
    }
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(PATROL_CONTACT);
        return Ok(Effect::Initialized {
            facing: actor.face_toward(player_x),
        });
    }
    match actor.state {
        1 | 2 => {
            let _ = fallwalker::patrol(actor, frame, world);
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(Effect::Continue);
            }
            if world.mario_distance_x(actor.position.x) >= REACH {
                return Ok(Effect::Continue);
            }
            actor.state = 3;
            actor.animation_frame = 0;
            actor.sprite = STRIKE_SPRITE;
            actor.contact_kind = ContactKind::from_raw(STRIKE_CONTACT);
            Ok(Effect::Continue)
        }
        3 => {
            if !frame.every(4) {
                return Ok(Effect::Continue);
            }
            actor.sprite = actor.sprite.wrapping_sub(1);
            if actor.sprite != CROUCH_SPRITE {
                return Ok(Effect::Continue);
            }
            actor.state = 4;
            actor.timer = CROUCH_TIMER;
            Ok(Effect::Continue)
        }
        4 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(Effect::Continue);
            }
            if !frame.every(4) {
                return Ok(Effect::Continue);
            }
            actor.sprite = actor.sprite.wrapping_add(1);
            if actor.sprite != STRIKE_SPRITE {
                return Ok(Effect::Continue);
            }
            let facing = actor.face_toward(player_x);
            actor.contact_kind = ContactKind::from_raw(PATROL_CONTACT);
            actor.timer = REARM_TIMER;
            Ok(Effect::Refaced { facing })
        }
        state => Err(Error::InvalidState(state)),
    }
}
