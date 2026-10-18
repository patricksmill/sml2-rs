//! Slot conversion for ROM `$5A969` (mode B anim 55).
use super::{handler_0x5a969, *};
use crate::actor::Actor;

impl ObjectSim {
    /// `$5B4F5` reads five bytes at the spawn pointer and the byte at
    /// pointer+8. Anim-55 children are only produced by `$5B36D`, so the
    /// pointer is a committed slot base ($AD00..$AEFF); anything else is an
    /// unresolved input rather than an invented projection.
    fn child_source(&self, pointer: u16) -> Option<handler_0x5a969::Source> {
        if !(0xAD00..=0xAEFF).contains(&pointer) {
            return None;
        }
        let slot = &self.slots[usize::from(pointer - 0xAD00) / 32];
        let parent = Actor::from_raw_slot(&slot.b);
        Some(handler_0x5a969::Source {
            position: parent.position,
            marker: parent.marker,
            gone: slot.b[B_STATE] != 2,
        })
    }

    pub(super) fn handler_0x5a969(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let source = self.child_source(actor.spawn_record);
        let result = handler_0x5a969::update(&mut actor, self.fff0 & 1 == 0, source);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a969::Effect::Continue) => {}
            Ok(handler_0x5a969::Effect::Remove) => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
            }
            Err(_) => self.note_unported(0x5A969),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x5A969_tests.rs"]
mod tests;
