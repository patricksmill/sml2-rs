use super::{handler_0x597bd, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x597bd(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x597bd::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x597BD);
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/table_driven.rs"]
mod tests;
