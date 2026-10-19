//! Slot conversion for ROM `$5AA71` (mode B anims 86 and 87).
use super::{handler_0x5aa71, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5aa71(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        if handler_0x5aa71::update(&mut actor, self.mario_ref_x).is_err() {
            st.b = actor.to_raw_slot();
            self.note_unported(0x5AA71);
            return;
        }
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
