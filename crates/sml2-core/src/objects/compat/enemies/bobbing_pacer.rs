//! Slot conversion for ROM `$5A721` (mode B anim 75).
use super::{handler_0x5a721, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a721(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5a721::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a721::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a721::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A721),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/bobbing_pacer.rs"]
mod tests;
