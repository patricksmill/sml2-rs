//! Slot conversion for ROM `$59487` (anim 67).
use super::{handler_0x59487, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x59487(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x59487::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x59487);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/resting_bobber.rs"]
mod tests;
