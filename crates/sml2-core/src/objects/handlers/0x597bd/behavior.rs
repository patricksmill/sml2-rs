//! ROM `$597BD` (anim 85): table-driven vertical platform (`$5938` family).
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 72;
pub(super) const TABLE_Y: [u8; 21] = [
    128, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1,
];
pub(super) const TABLE_X: [u8; 21] = [
    128, 0, 0, 0, 1, 0, 1, 1, 1, 1, 2, 1, 1, 2, 2, 1, 2, 2, 2, 2, 2,
];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Moved { dx: i16, dy: i16 },
    Waiting,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Effect {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        actor.face_toward(player_x);
        actor.hit_reaction = 1;
        actor.motion_leg = 1;
        return Effect::Initialized;
    }
    if !frame.every(2) {
        return Effect::Waiting;
    }
    let idx = usize::from(actor.hit_reaction.min(20));
    let dy = i16::from(TABLE_Y[idx].min(3));
    let dx = i16::from(TABLE_X[idx].min(3));
    actor
        .position
        .move_y(if actor.state & 1 == 0 { dy } else { -dy });
    actor
        .position
        .move_x(if actor.state & 2 == 0 { dx } else { -dx });
    actor.motion_leg = actor.motion_leg.wrapping_add(1);
    if actor.motion_leg >= 0x15 {
        actor.motion_leg = 1;
        actor.state = actor.state.wrapping_add(1);
        if actor.state > 8 {
            actor.state = 1;
        }
    }
    Effect::Moved { dx, dy }
}
