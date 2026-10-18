//! Slot conversion and rider displacement for ROM `$58C41` (anim 43).
use super::{handler_0x58c41, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x58c41(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x58c41::update(&mut actor, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x58c41::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x58c41::Effect::Moved { carry_dx: Some(dx) }) => {
                self.carry_mario_dx(dx);
            }
            Ok(_) => {}
            Err(_) => self.note_unported(0x58C41),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x58C41_tests.rs"]
mod tests;
