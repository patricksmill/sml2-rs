//! ROM `$59774` (anim 84): vertical shuttle between ceiling and floor probes.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 77;
pub(super) const WAIT_DISTANCE: u8 = 0x30;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Armed { state: u8 },
    Moved { dy: i16 },
    Waiting,
    Reface,
}

pub trait World {
    fn probe_ceiling(&mut self, actor: &Actor) -> bool;
    fn probe_floor(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    dist_x: u8,
    world: &mut impl World,
) -> Effect {
    let _ = actor.animate_four_step_bob_cadence(frame, 2);
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 3;
            Effect::Armed { state: 3 }
        }
        1 => {
            if !world.probe_ceiling(actor) {
                actor.position.move_y(-2);
                Effect::Moved { dy: -2 }
            } else {
                actor.sprite_flip.toggle_horizontal();
                actor.state ^= 3;
                Effect::Reface
            }
        }
        2 => {
            if !world.probe_floor(actor) {
                actor.position.move_y(2);
                Effect::Moved { dy: 2 }
            } else {
                actor.sprite_flip.toggle_horizontal();
                actor.state ^= 3;
                Effect::Reface
            }
        }
        3 => {
            if dist_x >= WAIT_DISTANCE {
                return Effect::Waiting;
            }
            actor.face_toward(player_x);
            Effect::Reface
        }
        _ => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 3;
            Effect::Armed { state: 3 }
        }
    }
}
