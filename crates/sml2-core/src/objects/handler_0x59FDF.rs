//! ROM `$59FDF` (mode B anim 35): arms contact 77, waits 24 frames, spawns anim
//! 36, then bobs until a 16-frame pause ends and faces Mario again.
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};
use std::cmp::Ordering;

const ARM_TIMER: u8 = 24;
const POST_SPAWN_TIMER: u8 = 16;
const REARM_TIMER: u8 = 112;
const CHILD_SPRITE: u8 = 62;
const CHILD_ANIM: u8 = 36;
const REARM_SPRITE: u8 = 59;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Waiting,
    Spawn {
        sprite: u8,
        anim: u8,
        flip: SpriteFlipFlags,
    },
    Refaced { facing: Ordering },
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(77);
        let facing = actor.face_toward(player_x);
        actor.timer = ARM_TIMER;
        return Ok(Effect::Initialized { facing });
    }
    match actor.state {
        // $59FF8: states 1 and 2 share the countdown, then spawn.
        1 | 2 => {
            if !timer_elapsed(actor) {
                return Ok(Effect::Waiting);
            }
            actor.timer = POST_SPAWN_TIMER;
            actor.state = 3;
            Ok(Effect::Spawn {
                sprite: CHILD_SPRITE,
                anim: CHILD_ANIM,
                flip: actor.sprite_flip,
            })
        }
        // $5A01C: bob, then reload facing when the post-spawn timer ends.
        3 => {
            let _ = actor.animate_four_step_bob_cadence(frame, 4);
            if !timer_elapsed(actor) {
                return Ok(Effect::Waiting);
            }
            actor.sprite = REARM_SPRITE;
            let facing = actor.face_toward(player_x);
            actor.timer = REARM_TIMER;
            Ok(Effect::Refaced { facing })
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// ROM countdown: a zero timer is already elapsed; any other value decrements
/// and elapses only when it hits zero.
fn timer_elapsed(actor: &mut Actor) -> bool {
    if actor.timer == 0 {
        return true;
    }
    actor.timer -= 1;
    actor.timer == 0
}
