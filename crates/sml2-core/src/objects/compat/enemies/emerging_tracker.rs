//! Slot conversion and engine effects for ROM $5902D.
use super::{
    handler_0x5902d::{self, Effect, World},
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
    fn divider(&self) -> u8 {
        self.0.div
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
    /// Shared slot adapter for ROM `$5902D` (anim 53) and `$5934A` (anim 62).
    /// `entry` keeps an unported-state report attributed to the dispatching
    /// entry; `state_three` selects that entry's wait-body immediates.
    pub(super) fn handler_emergence_tracker(
        &mut self,
        entry: u32,
        state_three: handler_0x5902d::StateThree,
        st: &mut Staging,
        tables: &ObjectTables,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5902d::update(&mut actor, frame, &mut Inputs(self), state_three);
        st.b = actor.to_raw_slot();
        match result {
            Ok(Effect::Despawn) => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
            }
            Ok(Effect::Continue) => {}
            Err(_) => self.note_unported(entry),
        }
    }

    pub(super) fn handler_0x5902d(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.handler_emergence_tracker(0x5902D, handler_0x5902d::ANIM_53_STATE_THREE, st, tables);
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/emerging_tracker.rs"]
mod tests;
