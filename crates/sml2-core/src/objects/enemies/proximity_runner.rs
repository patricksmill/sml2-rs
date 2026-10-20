//! ROM `$59774` (mode A anim 84): a runner that waits, glancing both ways,
//! until Mario comes within 48 pixels, then runs at him and turns at walls.
//!
//! Any state other than 1-3 arms contact 77 and waits in state 3. State 3
//! flips its facing every eighth frame (`$5B678`) and faces Mario
//! (`$5B3C6`) once the `$5B4B4` X distance drops below 48. States 1 and 2
//! advance the `$5B585` sprite cycle on even frames, probe the wall ahead
//! (`$3C58` / `$3BF4`) and either turn (`$59D2D`) or run two pixels.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

pub(super) const CONTACT: u8 = 77;
pub(super) const WAIT_DISTANCE: u8 = 0x30;
const RUN_SPEED: u16 = 2;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    Animation,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Armed,
    Waiting,
    FacedMario { facing: Ordering },
    Turned,
    Ran,
}

pub trait World {
    /// `$3C58`: true when the high left-wall probe is blocked.
    fn left_blocked(&mut self, actor: &Actor) -> bool;
    /// `$3BF4`: true when the high right-wall probe is blocked.
    fn right_blocked(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    dist_x: u8,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        // $59795 / $597A9
        state @ (1 | 2) => {
            actor
                .animate_four_step_bob_cadence(frame, 2)
                .map_err(|_| Error::Animation)?;
            let left = state == 1;
            let blocked = if left {
                world.left_blocked(actor)
            } else {
                world.right_blocked(actor)
            };
            if blocked {
                // $59D2D
                actor.sprite_flip.toggle_horizontal();
                actor.state ^= 3;
                return Ok(Effect::Turned);
            }
            actor.position.x = if left {
                actor.position.x.wrapping_sub(RUN_SPEED)
            } else {
                actor.position.x.wrapping_add(RUN_SPEED)
            };
            Ok(Effect::Ran)
        }
        // $59788
        3 => {
            if frame.every(8) {
                actor.sprite_flip.toggle_horizontal();
            }
            if dist_x >= WAIT_DISTANCE {
                return Ok(Effect::Waiting);
            }
            let facing = actor.face_toward(player_x);
            Ok(Effect::FacedMario { facing })
        }
        // $5977F
        _ => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 3;
            Ok(Effect::Armed)
        }
    }
}
