//! ROM `$5AFD7` (mode B anim 88): a hopping ant.
//!
//! State 0 arms contact 77, faces Mario, and sets a two-hop span. States 1 and
//! 2 walk one pixel every frame. On even frames they rise on `$7488` or fall
//! on `$74A4`. Reaching phase 15 reverses the vertical direction; the downward
//! turn also flips the walk. After two downward turns the ant opens its mouth,
//! spawns anim 89, and resumes the saved walk.
use super::staging::{BOUNCE_SPEED, FALL_SPEED};
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};
use std::cmp::Ordering;

const PHASE_CAP: u8 = 15;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Spawn,
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(77);
            let facing = actor.face_toward(player_x);
            actor.hit_reaction = 1;
            actor.set_travel_span(2);
            Ok(Effect::Initialized { facing })
        }
        1 => Ok(hop(actor, frame, -1)),
        2 => Ok(hop(actor, frame, 1)),
        3 => mouth(actor, frame),
        4 => {
            recover(actor);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn hop(actor: &mut Actor, frame: FrameTick, dx: i16) -> Effect {
    actor.position.move_x(dx);
    if !frame.every(2) {
        return Effect::Continue;
    }
    let rising = actor.hit_reaction != 0;
    let table: &[u8] = if rising { &FALL_SPEED } else { &BOUNCE_SPEED };
    let step = sample(actor, table);
    actor.position.move_y(if rising {
        -i16::from(step)
    } else {
        i16::from(step)
    });
    if actor.motion_phase != PHASE_CAP {
        return Effect::Continue;
    }
    actor.motion_phase = 0;
    actor.hit_reaction ^= 1;
    if actor.hit_reaction == 0 {
        actor.sprite_flip = SpriteFlipFlags::from_bits(actor.sprite_flip.bits() ^ 0x20);
        actor.state ^= 3;
        return Effect::Continue;
    }
    let span = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(span);
    if span != 0 {
        return Effect::Continue;
    }
    actor.visibility_mask = actor.state;
    actor.state = 3;
    Effect::Continue
}

fn sample(actor: &mut Actor, table: &[u8]) -> u8 {
    let step = table
        .get(usize::from(actor.motion_phase))
        .copied()
        .unwrap_or(0);
    if actor.motion_phase != PHASE_CAP {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    step
}

fn mouth(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    if !frame.every(4) {
        return Ok(Effect::Continue);
    }
    actor.sprite = actor.sprite.wrapping_add(1);
    if actor.sprite != 0x70 {
        return Ok(Effect::Continue);
    }
    actor.sprite = 108;
    actor.state = 4;
    actor.timer = 8;
    Ok(Effect::Spawn)
}

fn recover(actor: &mut Actor) {
    actor.timer = actor.timer.wrapping_sub(1);
    if actor.timer != 0 {
        return;
    }
    actor.state = actor.visibility_mask;
    actor.set_travel_span(2);
}
