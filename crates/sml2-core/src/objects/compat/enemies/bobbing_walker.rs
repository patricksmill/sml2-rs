//! Slot conversion and shared walker body for ROM `$5A667` (mode B anim 72).
use super::{handler_0x5a667, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a667(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5a667::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a667::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a667::Effect::Walk) => {
                let state = st.b[B_FSTATE];
                self.walker_state(st, layout, state);
            }
            Err(_) => self.note_unported(0x5A667),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/bobbing_walker.rs"]
mod tests;
