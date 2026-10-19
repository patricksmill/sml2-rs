//! Slot conversion and legacy global-facing effects for ROM $58B43.
use super::{
    handler_0x58b43::{self, World},
    *,
};
use crate::actor::{Actor, FrameTick, Position};
use std::cmp::Ordering;

struct Inputs<'a>(&'a mut ObjectSim);
impl World for Inputs<'_> {
    fn player_position(&self) -> Position {
        Position {
            x: self.0.mario_ref_x,
            y: self.0.mario_ref_y,
        }
    }
    fn horizontal_profile(&self) -> &[u8] {
        &FALL_SPEED
    }
    fn set_facing(&mut self, relation: Ordering) {
        self.0.facing = match relation {
            Ordering::Less => 255,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
    }
}
impl ObjectSim {
    pub(super) fn handler_0x58b43(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x58b43::update(&mut actor, frame, &mut Inputs(self));
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x58B43);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/diagonal_tracker.rs"]
mod tests;
