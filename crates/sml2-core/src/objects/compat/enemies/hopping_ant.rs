//! Slot conversion for ROM `$5AFD7` (mode B anim 88).
use super::{handler_0x5afd7, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5afd7(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5afd7::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5afd7::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5afd7::Effect::Spawn) => {
                self.spawn_anim = 89;
                self.spawn_sprite = 112;
                self.spawn_dynamic_5b36d(st);
            }
            Ok(handler_0x5afd7::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5AFD7),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/hopping_ant.rs"]
mod tests;
