//! Slot conversion and rider carry for ROM `$5A236` (mode B anim 62).
use super::{handler_0x5a236, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a236(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5a236::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a236::Effect::Carried) => {
                self.carry_mario_right(1);
            }
            Ok(handler_0x5a236::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A236),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
