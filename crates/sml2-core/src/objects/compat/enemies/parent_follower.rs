//! Slot conversion for ROM `$5A5C7` (mode B anim 69).
use super::{handler_0x5a5c7, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a5c7(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let source =
            self.project_slot_source(actor.spawn_record)
                .map(|s| handler_0x5a5c7::Source {
                    position: s.position,
                    marker: s.marker,
                });
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5a5c7::update(&mut actor, frame, source);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x5A5C7);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/parent_follower.rs"]
mod tests;
