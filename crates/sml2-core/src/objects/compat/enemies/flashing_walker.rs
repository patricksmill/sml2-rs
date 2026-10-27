//! Adapter for ROM `$59FB9` (mode B anim 32): bob, then the `$58215` walker on even frames.
use super::{handler_0x59fb9, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x59fb9(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let effect = handler_0x59fb9::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match effect {
            handler_0x59fb9::Effect::Initialized { facing } => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            handler_0x59fb9::Effect::Motion => {
                if self.fff0 & 1 == 0 {
                    self.walker_state(st, layout, st.b[B_FSTATE]);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/flashing_walker.rs"]
mod tests;
