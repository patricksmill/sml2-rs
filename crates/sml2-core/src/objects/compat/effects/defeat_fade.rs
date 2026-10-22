//! Slot conversion for ROM `$5AC01` (mode B anim 101).
use super::{handler_0x5ac01, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5ac01(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x5ac01::update(&mut actor, self.easy_mode);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ac01::Effect::Continue) => {}
            Ok(handler_0x5ac01::Effect::Rebirth) => {
                // `$3AB4` -> `$85DC` -> `$863C`: re-derive in place; the object
                // keeps its position, marker and activity.
                self.finish_sprite_swap(st, tables);
                st.b[B_TYPE] = handler_0x5ac01::CONTACT;
                st.b[B_FSTATE] = st.b[B_MASK];
            }
            Ok(handler_0x5ac01::Effect::RemoveBoss) => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
                if self.boss_defeated {
                    self.boss_defeated = false;
                }
            }
            Err(_) => self.note_unported(0x5AC01),
        }
    }
}

#[cfg(test)]
#[path = "../../effects/tests/defeat_fade.rs"]
mod tests;
