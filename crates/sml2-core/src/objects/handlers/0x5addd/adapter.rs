//! Slot conversion for ROM `$5ADDD` (mode B anim 110).
use super::{handler_0x5addd, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5addd(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5addd::update(
            &mut actor,
            frame,
            self.mario_ref_x,
            handler_0x5addd::anim110(),
        );
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5addd::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5addd::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5ADDD),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
