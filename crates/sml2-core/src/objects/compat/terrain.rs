//! Shared legacy terrain boundary for native patrol and fallwalker behavior.
use super::{falling_dropper, fallwalker, *};
use crate::actor::Actor;

pub(super) struct TerrainWorld<'a> {
    pub(super) sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl<'a> TerrainWorld<'a> {
    pub(super) fn new(
        sim: &'a mut ObjectSim,
        st: &'a mut Staging,
        layout: &'a LevelLayout,
    ) -> Self {
        Self { sim, st, layout }
    }

    fn stage_position(&mut self, actor: &Actor) {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
    }
}

impl fallwalker::PatrolWorld for TerrainWorld<'_> {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool {
        self.stage_position(actor);
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }

    fn probe_right_wall(&mut self, actor: &Actor) -> bool {
        self.stage_position(actor);
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }

    fn probe_ground(&mut self, actor: &Actor) -> bool {
        self.stage_position(actor);
        // Keep the legacy helper's scratch writes, including alignment early returns.
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl fallwalker::FallWalkerWorld for TerrainWorld<'_> {
    fn step_fall(&mut self, actor: &mut Actor) -> u8 {
        falling_dropper::step_fall_speed(actor)
    }
}
