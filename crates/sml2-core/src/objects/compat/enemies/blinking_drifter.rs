use super::{handler_0x59aad, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x59aad(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x59aad::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x59AAD);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/blinking_drifter.rs"]
mod tests;
