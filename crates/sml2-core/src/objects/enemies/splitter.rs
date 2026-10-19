//! ROM `$5A8E4` (mode B anim 54): the large stomped enemy that sheds a
//! child and walks/falls until it is respawned.
//!
//! State 0 faces Mario, flips away from him, arms type 133 and the anim 55 /
//! sprite 90 child at the pre-lift position, then lifts 16 px and requests
//! sound 18. States 1/2 walk 3 px on even frames and switch to the fall states
//! when the 16-px ground probe reports air; states 3/4 keep walking and fall on
//! odd frames until the probe reports ground.
use super::falling_dropper::step_fall_speed;
use crate::actor::{Actor, ContactKind, SpriteFlipFlags};

const TYPE: u8 = 133;
const CHILD_ANIM: u8 = 55;
const CHILD_SPRITE: u8 = 90;
const LIFT: u8 = 16;
const SFX1: u8 = 18;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    SpawnChild {
        anim: u8,
        sprite: u8,
        flip: SpriteFlipFlags,
        lift: u8,
        sfx1: u8,
    },
}

pub(super) trait World {
    /// `$3B54`/`$8381`: the 16-px-below probe. True when it reports ground.
    fn probe_ground(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    player_x: u16,
    frame_even: bool,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        // $5A8E9: face Mario, turn away, arm the child, lift, sound.
        0 => {
            actor.face_toward(player_x);
            actor.sprite_flip.toggle_horizontal();
            actor.contact_kind = ContactKind::from_raw(TYPE);
            Ok(Effect::SpawnChild {
                anim: CHILD_ANIM,
                sprite: CHILD_SPRITE,
                flip: actor.sprite_flip,
                lift: LIFT,
                sfx1: SFX1,
            })
        }
        // $5A939: walk right; air switches to state 3.
        1 => {
            if frame_even {
                actor.position.move_x(3);
            } else if !world.probe_ground(actor) {
                actor.state = actor.state.wrapping_add(2);
            }
            Ok(Effect::Continue)
        }
        // $5A92D: walk left; air switches to state 4.
        2 => {
            if frame_even {
                actor.position.move_x(-3);
            } else if !world.probe_ground(actor) {
                actor.state = actor.state.wrapping_add(2);
            }
            Ok(Effect::Continue)
        }
        // $5A945: walk right; odd frames fall and land back into state 1.
        3 => {
            if frame_even {
                actor.position.move_x(3);
            } else {
                let speed = step_fall_speed(actor);
                actor.position.move_y(i16::from(speed));
                if world.probe_ground(actor) {
                    actor.state = actor.state.wrapping_sub(2);
                }
            }
            Ok(Effect::Continue)
        }
        // $5A94D: walk left; odd frames fall and land back into state 2.
        4 => {
            if frame_even {
                actor.position.move_x(-3);
            } else {
                let speed = step_fall_speed(actor);
                actor.position.move_y(i16::from(speed));
                if world.probe_ground(actor) {
                    actor.state = actor.state.wrapping_sub(2);
                }
            }
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}
