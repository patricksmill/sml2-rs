//! ROM `$5A721` (mode B anim 75): the `$5A30E` pacer with an eight-step bob.
//!
//! Every call advances `$7669` and sets vertical flip on phases 3 and 6.
//! State 0 then arms contact 72, faces Mario, and loads a 56-step leg with a
//! 112-step reload. States 1/2 move one pixel on even frames and flip when
//! the leg ends.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

const DELTAS: [i8; 8] = [-1, 1, 1, -1, -1, 1, 1, -1];
const FIRST_LEG: u8 = 56;
const RELOAD: u8 = 112;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Continue,
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
) -> Result<Effect, Error> {
    bob(actor);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(72);
        let facing = actor.face_toward(player_x);
        actor.set_travel_span(FIRST_LEG);
        actor.hit_reaction = RELOAD;
        return Ok(Effect::Initialized { facing });
    }
    if !frame.every(2) {
        return Ok(Effect::Continue);
    }
    match actor.state {
        1 => step(actor, -1),
        2 => step(actor, 1),
        state => Err(Error::InvalidState(state)),
    }
}

fn bob(actor: &mut Actor) {
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 8 {
        actor.animation_frame = 0;
    }
    let delta = DELTAS[usize::from(actor.animation_frame)];
    actor.sprite = actor.sprite.wrapping_add_signed(delta);
    match actor.animation_frame {
        3 => actor.sprite_flip.set_vertical(true),
        6 => actor.sprite_flip.set_vertical(false),
        _ => {}
    }
}

fn step(actor: &mut Actor, dx: i16) -> Result<Effect, Error> {
    actor.position.move_x(dx);
    let remaining = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(remaining);
    if remaining != 0 {
        return Ok(Effect::Continue);
    }
    actor.set_travel_span(actor.hit_reaction);
    actor.state ^= 3;
    actor.sprite_flip.toggle_horizontal();
    Ok(Effect::Continue)
}
