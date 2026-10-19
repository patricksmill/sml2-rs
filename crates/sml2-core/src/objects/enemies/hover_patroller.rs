//! ROM `$599A6` (anim 86): phased horizontal patrol with hover segments.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Armed { state: u8 },
    Moved { dy: i16 },
    Waiting,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Effect {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 5;
            Effect::Armed { state: 5 }
        }
        5 => {
            actor.position.move_y(2);
            actor.hit_reaction = actor.hit_reaction.wrapping_add(2);
            if actor.hit_reaction < 0x40 {
                return Effect::Moved { dy: 2 };
            }
            actor.hit_reaction = 9;
            actor.state = 6;
            Effect::Moved { dy: 2 }
        }
        6 => {
            actor.position.move_y(-3);
            actor.hit_reaction = actor.hit_reaction.wrapping_sub(3);
            if actor.hit_reaction != 0 {
                return Effect::Moved { dy: -3 };
            }
            actor.state = 7;
            actor.sprite = 151;
            Effect::Moved { dy: -3 }
        }
        7 | 8 => {
            if !frame.every(2) {
                return Effect::Waiting;
            }
            actor.position.move_y(if actor.state == 7 { 1 } else { -1 });
            Effect::Moved {
                dy: if actor.state == 7 { 1 } else { -1 },
            }
        }
        _ => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 5;
            actor.hit_reaction = 0;
            Effect::Armed { state: 5 }
        }
    }
}
