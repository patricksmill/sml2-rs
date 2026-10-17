use super::{handler_0x59774, pipe_bobber::dist_mario_x, *};
use crate::actor::{Actor, FrameTick};

struct Probes<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x59774::World for Probes<'_> {
    fn probe_ceiling(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ceil_own(self.st, self.layout, 18);
        self.sim.move_flags & 0x04 != 0
    }
    fn probe_floor(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x59774(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let mario_ref_x = self.mario_ref_x;
        let dist = dist_mario_x(mario_ref_x, actor.position.x);
        let mut world = Probes {
            sim: self,
            st,
            layout,
        };
        let effect = handler_0x59774::update(&mut actor, frame, mario_ref_x, dist, &mut world);
        st.b = actor.to_raw_slot();
        let _ = effect;
    }
}

#[cfg(test)]
#[path = "handler_0x59774_tests.rs"]
mod tests;
