//! Slot conversion and rider carry for ROM `$595FE` (anim 79).
use super::{handler_0x595fe, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x595fe(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x595fe::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x595fe::Effect::Moved { dy }) if actor.interactions.stood_on() => {
                let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            }
            Ok(_) => {}
            Err(_) => self.note_unported(0x595FE),
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/vertical_oscillator.rs"]
mod tests;
