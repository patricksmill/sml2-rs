//! Slot conversion and rider displacement for ROM `$58C52` (anim 56).
use super::{handler_0x58c52, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x58c52(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x58c52::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x58c52::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            // $3CF1/$3CC0: the rider moves with the platform's 16-bit X.
            Ok(handler_0x58c52::Effect::Moved { carry_dx: Some(dx) }) => {
                self.carry_mario_dx(dx);
            }
            Ok(_) => {}
            Err(_) => self.note_unported(0x58C52),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x58C52_tests.rs"]
mod tests;
