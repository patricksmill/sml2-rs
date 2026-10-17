//! Slot conversion and rider carry for ROM `$5A355` (mode B anim 67).
use super::{handler_0x5a355, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a355(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5a355::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            handler_0x5a355::Effect::Initialized { facing } => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            handler_0x5a355::Effect::Moved { carry_dx, carry_dy } => {
                if let Some(dx) = carry_dx {
                    let x = self.mario_x_adjust.unwrap_or(self.mario.x);
                    self.mario_x_adjust = Some(x.wrapping_add_signed(dx));
                }
                if let Some(dy) = carry_dy {
                    let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                    self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
                }
            }
            handler_0x5a355::Effect::Idle => {}
        }
    }
}

#[cfg(test)]
#[path = "handler_0x5A355_tests.rs"]
mod tests;
