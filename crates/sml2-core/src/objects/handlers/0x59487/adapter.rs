//! Slot conversion for ROM `$59487` (anim 67).
use super::{handler_0x59487, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x59487(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x59487::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x59487);
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
