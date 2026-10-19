//! Slot conversion for ROM `$5A816` (mode B anim 78).
use super::{handler_0x5a816, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5a816(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        if handler_0x5a816::update(&mut actor).is_err() {
            self.note_unported(0x5A816);
        }
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../projectiles/tests/cannon_ant.rs"]
mod tests;
