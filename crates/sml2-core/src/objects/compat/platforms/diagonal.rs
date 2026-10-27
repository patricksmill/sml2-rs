//! Slot conversion and rider displacement for ROM `$58D34` (anim 45).
use super::{handler_0x58d34, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x58d34(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x58d34::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x58d34::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x58d34::Effect::Moved { carry_dx, carry_dy }) => {
                if let Some(dx) = carry_dx {
                    let x = self.mario_x_adjust.unwrap_or(self.mario.x);
                    self.mario_x_adjust = Some(x.wrapping_add_signed(dx));
                }
                if let Some(dy) = carry_dy {
                    let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                    self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
                }
            }
            Ok(handler_0x58d34::Effect::Idle) => {}
            Err(_) => self.note_unported(0x58D34),
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/diagonal.rs"]
mod tests;
