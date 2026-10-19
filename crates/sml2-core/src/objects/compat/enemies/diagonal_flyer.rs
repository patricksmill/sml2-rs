//! Slot conversion for ROM `$59728` (mode A anim 82).
use super::{handler_0x59728, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x59728(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x59728::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x59728::Effect::Continue) => {}
            Ok(handler_0x59728::Effect::Sfx2) => self.sfx2 = 11,
            Err(_) => self.note_unported(0x59728),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/diagonal_flyer.rs"]
mod tests;
