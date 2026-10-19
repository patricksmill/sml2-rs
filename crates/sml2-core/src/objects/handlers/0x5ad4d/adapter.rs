//! Slot conversion for ROM `$5AD4D` (mode B anim 107).
use super::{handler_0x5ad4d, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5ad4d(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5ad4d::update(&mut actor, frame, self.easy_mode);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ad4d::Effect::Continue) => {}
            Ok(handler_0x5ad4d::Effect::Carried { dx }) => {
                self.carry_mario_dx(dx);
            }
            Err(_) => self.note_unported(0x5AD4D),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
