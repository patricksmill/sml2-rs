//! ROM `$5A2D0` (mode B anim 64): rises 16 pixels, faces Mario, then crawls.
//!
//! State 0 arms contact 77, state 5, a 16-step rise, and defeat mode 128.
//! Every later call runs the `$5B5AD` three-step sprite bob. State 5 rises one
//! pixel on even frames and, at the end, faces Mario and clears the defeat
//! mode. States 1–4 are the shared `$5864E`/`$5866D` crawler bodies.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

const DELTAS: [i8; 3] = [-2, 1, 1];
const RISE: u8 = 16;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Faced { facing: Ordering },
    Crawl,
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(77);
        actor.state = 5;
        actor.motion_leg = RISE;
        actor.defeat_mode = 128;
        return Ok(Effect::Initialized);
    }
    if (1..=5).contains(&actor.state) {
        bob(actor, frame);
    }
    match actor.state {
        1..=4 => Ok(Effect::Crawl),
        5 => {
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            actor.position.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg != 0 {
                return Ok(Effect::Continue);
            }
            let facing = actor.face_toward(player_x);
            actor.defeat_mode = 0;
            Ok(Effect::Faced { facing })
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn bob(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(4) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 3 {
        actor.animation_frame = 0;
    }
    let delta = DELTAS[usize::from(actor.animation_frame)];
    actor.sprite = actor.sprite.wrapping_add_signed(delta);
}
