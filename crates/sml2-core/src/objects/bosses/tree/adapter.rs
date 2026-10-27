//! World effects and slot conversion for boss anim 1 (`$93A7`).
use super::{tree_boss, ObjectSim, Staging};
use crate::actor::Actor;
use crate::object::ObjectTables;

impl ObjectSim {
    pub(super) fn handler_tree_boss(&mut self, staging: &mut Staging, tables: &ObjectTables) {
        self.apply_boss_damage(staging);
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = tree_boss::update(
            &mut actor,
            tree_boss::Inputs {
                frame: self.frame_tick(),
                fade_idle: self.fade_timer == 0,
                hit: if self.done != 0 {
                    Some(self.boss_hit_latch != 0)
                } else {
                    None
                },
                primary: self.boss_slot_ptr_lo == 0,
                timer_reload: self.boss_timer_reload,
                removal_pending: self.remove_mode != 0,
                definition_sprite: self.boss_sprite,
                saved_sprite: self.spawn_sprite,
                saved_state: self.saved_frame_state,
                remaining_hits: self.boss_hits_remaining,
                alternate_phase: self.boss_alt_phase,
                entrance_profile: &tables.boss_picker_profile,
                sweep_profile: &tables.boss_sweep_profile,
            },
        );
        staging.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if let Some((sprite, state, contact)) = effects.saved_hit {
                    self.spawn_sprite = sprite;
                    self.saved_frame_state = state;
                    self.saved_type = contact.raw();
                }
                if let Some(phase) = effects.alternate_phase {
                    self.boss_alt_phase = phase;
                }
                if let Some(sound) = effects.sound1 {
                    self.sfx1 = sound;
                }
                if let Some(sound) = effects.sound2 {
                    self.sfx2 = sound;
                }
                if effects.silence_music {
                    self.music_request = Some(255);
                }
            }
            Err(()) => self.note_unported(0x93A7),
        }
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
