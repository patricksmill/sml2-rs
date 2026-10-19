//! Slot conversion and rider carry for ROM `$5A683` (mode B anim 73).
use super::{handler_0x5a683, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a683(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5a683::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a683::Effect::Carried) => {
                self.carry_mario_right(1);
            }
            Ok(handler_0x5a683::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A683),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
