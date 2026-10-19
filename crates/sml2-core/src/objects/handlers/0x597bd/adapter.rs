use super::{handler_0x597bd, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x597bd(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let _ = handler_0x597bd::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
