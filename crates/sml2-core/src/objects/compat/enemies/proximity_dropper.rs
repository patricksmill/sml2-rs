//! Slot conversion and terrain probes for ROM `$5AC3C` (mode B anim 102).
use super::{handler_0x5ac3c, *};
use crate::actor::Actor;

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl Inputs<'_> {
    fn sync(&mut self, actor: &Actor) {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
    }
}

impl handler_0x5ac3c::World for Inputs<'_> {
    fn grounded(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
    fn ceiling(&mut self, actor: &Actor) -> bool {
        if self
            .layout
            .get(
                usize::from((actor.position.x >> 4) & 255),
                usize::from((actor.position.y.wrapping_sub(32) >> 4) & 255),
            )
            .unwrap_or(0)
            < 0x38
        {
            self.sim.move_flags = 0x44;
            true
        } else {
            self.sim.move_flags = 0x40;
            false
        }
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5ac3c(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let player_x = self.mario_ref_x;
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x5ac3c::update(&mut actor, frame, player_x, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ac3c::Effect::Continue) => {}
            Ok(handler_0x5ac3c::Effect::Landed) => self.sfx1 = 10,
            Err(_) => self.note_unported(0x5AC3C),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/proximity_dropper.rs"]
mod tests;
