//! ROM `$5A0AA` (mode B anim 37): a patrolling walker that spawns anim 38 when
//! its travel span expires, pauses, then hops three pixels and resumes.
//!
//! State 0 arms contact 77, faces Mario, and loads span 48. Later calls count
//! the span down first. At zero the actor sets sprite 66, spawns anim 38, and
//! advances two states. Otherwise states 1/2 bob on even frames (`$5B585`) and
//! run the shared `$582FB`/`$5831A` patrol. States 3/4 wait out a 16-frame
//! timer, hop three pixels with the facing, reload span 88, and return to the
//! patrol pair.
use super::fallwalker::{self, FallWalkerWorld};
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};
use std::cmp::Ordering;

const FIRST_SPAN: u8 = 48;
const HOP_SPAN: u8 = 88;
const PAUSE: u8 = 16;
const HOP: i16 = 3;
const CHILD_ANIM: u8 = 38;
const CHILD_SPRITE: u8 = 69;
const OWN_SPRITE: u8 = 66;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized {
        facing: Ordering,
    },
    Spawn {
        sprite: u8,
        anim: u8,
        flip: SpriteFlipFlags,
    },
    Continue,
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl FallWalkerWorld,
) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(77);
        let facing = actor.face_toward(player_x);
        actor.set_travel_span(FIRST_SPAN);
        return Ok(Effect::Initialized { facing });
    }
    let span = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(span);
    if span == 0 {
        actor.sprite = OWN_SPRITE;
        actor.timer = PAUSE;
        actor.state = actor.state.wrapping_add(2);
        actor.animation_frame = 0;
        return Ok(Effect::Spawn {
            sprite: CHILD_SPRITE,
            anim: CHILD_ANIM,
            flip: actor.sprite_flip,
        });
    }
    if actor.state < 3 {
        let _ = actor.animate_four_step_bob_cadence(frame, 2);
    }
    match actor.state {
        1 | 2 => {
            let _ = fallwalker::patrol(actor, frame, world);
            Ok(Effect::Continue)
        }
        3 | 4 => {
            if !timer_elapsed(actor) {
                return Ok(Effect::Continue);
            }
            let dx = if actor.sprite_flip.bits() == 0 {
                HOP
            } else {
                -HOP
            };
            actor.position.move_x(dx);
            actor.set_travel_span(HOP_SPAN);
            actor.state = actor.state.wrapping_sub(2);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn timer_elapsed(actor: &mut Actor) -> bool {
    if actor.timer == 0 {
        return true;
    }
    actor.timer -= 1;
    actor.timer == 0
}
