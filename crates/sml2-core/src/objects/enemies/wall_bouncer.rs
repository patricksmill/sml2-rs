//! ROM `$59B1F` (anim 92): `$5B51A` toggle and wall-bouncing horizontal drift.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Moved { dx: i16 },
    Waiting,
    Turned,
}

pub trait World {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Effect {
    actor.animate_two_frame_toggle(frame, 4);
    if actor.state == 0 {
        actor.face_toward(player_x);
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        return Effect::Initialized;
    }
    if !frame.every(2) {
        return Effect::Waiting;
    }
    if actor.state == 1 {
        actor.position.move_x(-2);
        if world.probe_left_wall(actor) {
            actor.state = 2;
            actor.sprite_flip.toggle_horizontal();
            return Effect::Turned;
        }
        Effect::Moved { dx: -2 }
    } else {
        actor.position.move_x(2);
        if world.probe_right_wall(actor) {
            actor.state = 1;
            actor.sprite_flip.toggle_horizontal();
            return Effect::Turned;
        }
        Effect::Moved { dx: 2 }
    }
}
