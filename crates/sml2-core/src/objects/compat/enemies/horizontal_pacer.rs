//! Slot conversion for ROM `$5A30E` (mode B anim 65).
use super::{handler_0x5a30e, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a30e(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5a30e::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a30e::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a30e::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A30E),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/horizontal_pacer.rs"]
mod tests;
