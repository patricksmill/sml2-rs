//! Slot conversion for ROM `$5AEF4` (mode B anim 111).
use super::{handler_0x5addd, handler_0x5aef4, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5aef4(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5aef4::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5addd::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5addd::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5AEF4),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/short_curved_flyer.rs"]
mod tests;
