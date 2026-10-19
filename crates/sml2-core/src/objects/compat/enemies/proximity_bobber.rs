//! Slot conversion for ROM `$59EFF` (mode B anim 33).
use super::{handler_0x59eff, pipe_bobber, *};
use crate::actor::{Actor, FrameTick};

struct Inputs<'a> {
    sim: &'a ObjectSim,
}

impl handler_0x59eff::World for Inputs<'_> {
    fn mario_distance_x(&self, actor_x: u16) -> u8 {
        pipe_bobber::dist_mario_x(self.sim.mario_ref_x, actor_x)
    }
}

impl ObjectSim {
    pub(super) fn handler_0x59eff(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x59eff::update(&mut actor, frame, &Inputs { sim: self });
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x59EFF);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/proximity_bobber.rs"]
mod tests;
