//! Adapter for ROM `$59FCE` (mode B anim 34): bob, then the `$58215` walker on even frames.
use super::{handler_0x59fce, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x59fce(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let effect = handler_0x59fce::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match effect {
            handler_0x59fce::Effect::Initialized { facing } => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            handler_0x59fce::Effect::Motion => {
                if self.fff0 & 1 == 0 {
                    self.walker_state(st, layout, st.b[B_FSTATE]);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "handler_0x59FCE_tests.rs"]
mod tests;
