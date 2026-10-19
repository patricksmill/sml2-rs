//! Slot conversion for ROM `$59431` (anim 65) and `$5945A` (anim 66).
use super::{
    handler_0x59431::{self, Kind},
    *,
};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl fallwalker::FallWalkerWorld for Inputs<'_> {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn probe_right_wall(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
    fn step_fall(&mut self, actor: &mut Actor) -> u8 {
        let dy = self.sim.fall_step(self.st);
        actor.motion_phase = self.st.b[B_PHASE];
        dy
    }
}

impl handler_0x59431::World for Inputs<'_> {
    fn mario_ref_x(&self) -> u16 {
        self.sim.mario_ref_x
    }
    fn set_facing(&mut self, relation: Ordering) {
        self.sim.facing = match relation {
            Ordering::Less => 255,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
    }
}

impl ObjectSim {
    /// Shared slot adapter for ROM `$59431` (anim 65) and `$5945A` (anim 66).
    /// `entry` keeps an out-of-profile report attributed to the dispatching
    /// entry; `kind` selects that entry's landing contact and fall flash.
    pub(super) fn handler_landing_walker(
        &mut self,
        entry: u32,
        kind: Kind,
        st: &mut Staging,
        layout: &LevelLayout,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x59431::update(&mut actor, frame, &mut world, kind)
        };
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(entry);
        }
    }

    pub(super) fn handler_0x59431(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.handler_landing_walker(0x59431, handler_0x59431::ANIM_65, st, layout);
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/landing_walker.rs"]
mod tests;
