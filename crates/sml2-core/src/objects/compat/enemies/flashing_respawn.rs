//! Slot conversion for ROM `$59376` (mode A anim 63).
use super::{handler_0x59376, *};
use crate::actor::{Actor, FrameTick};

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x59376::World for Inputs<'_> {
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x59376(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x59376::update(&mut actor, frame, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x59376::Effect::Continue) => {}
            Ok(handler_0x59376::Effect::Sfx2) => self.sfx2 = 1,
            Ok(handler_0x59376::Effect::Rebirth) => {
                // `$3AA0` -> `$863C`: re-derive in place; the fuse keeps its
                // landed position, marker and activity.
                self.finish_sprite_swap(st, tables);
                let mut tail = Actor::from_raw_slot(&st.b);
                self.facing = match tail.face_toward(self.mario_ref_x) {
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Less => 0xFF,
                    std::cmp::Ordering::Equal => 0,
                };
                handler_0x59376::arm_tail(&mut tail);
                st.b = tail.to_raw_slot();
            }
            Err(_) => self.note_unported(0x59376),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/flashing_respawn.rs"]
mod tests;
