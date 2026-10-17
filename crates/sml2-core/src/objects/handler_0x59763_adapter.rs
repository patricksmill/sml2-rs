//! Adapter for ROM `$59763` (anim 83): bob + walker `$58215` tail.
use super::{handler_0x59763, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x59763(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let effect = handler_0x59763::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match effect {
            handler_0x59763::Effect::Initialized { facing } => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            handler_0x59763::Effect::Motion => {
                if self.fff0 & 1 == 0 {
                    self.walker_state(st, layout, st.b[B_FSTATE]);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "handler_0x59763_tests.rs"]
mod tests;
