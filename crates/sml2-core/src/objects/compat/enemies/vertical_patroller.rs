//! Slot conversion for ROM `$5A992` (mode B anim 80).
use super::{handler_0x5a992, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a992(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        if handler_0x5a992::update(&mut actor, frame).is_err() {
            st.b = actor.to_raw_slot();
            self.note_unported(0x5A992);
            return;
        }
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/vertical_patroller.rs"]
mod tests;
