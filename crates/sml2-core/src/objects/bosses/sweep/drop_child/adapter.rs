//! Compatibility boundary for boss anim 18.
use super::{sweep_drop_child, ObjectSim, Staging};
use crate::{
    actor::Actor,
    object::ObjectTables,
};

impl ObjectSim {
    pub(super) fn handler_sweep_drop_child(
        &mut self,
        staging: &mut Staging,
        tables: &ObjectTables,
    ) {
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = sweep_drop_child::update(
            &mut actor,
            sweep_drop_child::Inputs {
                frame: self.frame_tick(),
                player_x: self.player_screen_x,
                remove_mode: self.remove_mode,
                drop_profile: &tables.boss_picker_profile,
            },
        );
        staging.b = actor.to_raw_slot();
        match result {
            Ok(Some(sound)) => self.sfx1 = sound,
            Ok(None) => {}
            Err(_) => self.note_unported(0xA2BE),
        }
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
