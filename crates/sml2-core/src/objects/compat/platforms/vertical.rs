//! Slot conversion and rider displacement for ROM `$58BBA` (anim 41) and
//! `$58BCB` (anim 40).
use super::{
    handler_0x58bba::{self, Kind},
    *,
};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    /// Shared slot adapter. `entry` keeps an invalid-state report attributed
    /// to the dispatching entry; `kind` selects that entry's half-span.
    pub(super) fn handler_vertical_platform(&mut self, entry: u32, kind: Kind, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x58bba::update(&mut actor, kind, self.mario_ref_y);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x58bba::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            // $3D10/$3CDF: the rider moves with the platform's 16-bit Y.
            Ok(handler_0x58bba::Effect::Moved { carry_dy: Some(dy) }) => {
                let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            }
            Ok(_) => {}
            Err(_) => self.note_unported(entry),
        }
    }

    pub(super) fn handler_0x58bba(&mut self, st: &mut Staging) {
        self.handler_vertical_platform(0x58BBA, handler_0x58bba::ANIM_41, st);
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/vertical.rs"]
mod tests;
