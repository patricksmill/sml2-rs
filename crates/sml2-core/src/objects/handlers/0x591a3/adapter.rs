//! Slot conversion and rider displacement for ROM `$591A3` (anim 55).
use super::{handler_0x591a3, *};
use crate::actor::{Actor, FrameTick};

struct Inputs<'a>(&'a mut ObjectSim);
impl handler_0x591a3::World for Inputs<'_> {
    fn request_sfx1(&mut self, id: u8) {
        self.0.sfx1 = id;
    }
}

impl ObjectSim {
    pub(super) fn handler_0x591a3(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x591a3::update(&mut actor, frame, &mut Inputs(self));
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x591a3::Effect::Rising { carry_dy: Some(dy) })
            | Ok(handler_0x591a3::Effect::Descending { carry_dy: Some(dy) }) => {
                let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            }
            Ok(_) => {}
            Err(_) => self.note_unported(0x591A3),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
