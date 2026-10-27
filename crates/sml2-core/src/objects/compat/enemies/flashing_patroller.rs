//! Slot conversion for ROM `$5966C` (mode A anim 80).
use super::{handler_0x5966c, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5966c(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5966c::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x5966C);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/flashing_patroller.rs"]
mod tests;
