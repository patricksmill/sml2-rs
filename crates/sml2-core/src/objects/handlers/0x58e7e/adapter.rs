//! Slot conversion and rider displacement for ROM `$58E7E` (anim 48).
use super::{handler_0x58e7e, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x58e7e(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x58e7e::update(&mut actor, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x58e7e::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x58e7e::Effect::Moved { carry_dx, carry_dy }) => {
                if let Some(dx) = carry_dx {
                    let x = self.mario_x_adjust.unwrap_or(self.mario.x);
                    self.mario_x_adjust = Some(x.wrapping_add_signed(dx));
                }
                if let Some(dy) = carry_dy {
                    let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                    self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
                }
            }
            Err(_) => self.note_unported(0x58E7E),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
