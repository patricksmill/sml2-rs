use super::{handler_0x59b1f, *};
use crate::actor::{Actor, FrameTick};

struct Walls<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x59b1f::World for Walls<'_> {
    fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x59b1f(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let mario_ref_x = self.mario_ref_x;
        let mut world = Walls {
            sim: self,
            st,
            layout,
        };
        let _ = handler_0x59b1f::update(&mut actor, frame, mario_ref_x, &mut world);
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
