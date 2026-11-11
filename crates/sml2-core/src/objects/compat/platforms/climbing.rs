//! Slot conversion and rider carry for ROM `$5A85C` (mode B anim 79).
use super::{handler_0x5a85c, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5a85c(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5a85c::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a85c::Effect::Continue) => {}
            Ok(handler_0x5a85c::Effect::Ride { dy }) => {
                if actor.interactions.stood_on() {
                    // $3D22/$3D2D/$3D3D carry Mario's $FFC2/$FFC0 bytes; $3CF1
                    // also publishes the left-scroll delta the camera consumes.
                    let x = self.mario_x_adjust.unwrap_or(self.mario.x);
                    self.mario_x_adjust = Some(x.wrapping_sub(1));
                    let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                    self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
                    self.scroll_delta_left = self.scroll_delta_left.wrapping_add(1);
                }
            }
            Ok(handler_0x5a85c::Effect::Reset) => {
                if self.reset_record_origin(st, tables).is_err() {
                    self.note_unported(0x5A85C);
                }
            }
            Err(_) => self.note_unported(0x5A85C),
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/climbing.rs"]
mod tests;
