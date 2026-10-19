//! World and slot conversion boundary for boss anim 16.
use super::{boss_stun::HitStunContext, sweep_boss, ObjectSim, Staging};
use crate::{
    actor::{Actor, ContactKind, FrameTick},
    object::ObjectTables,
};

impl ObjectSim {
    pub(super) fn handler_sweep_boss(&mut self, staging: &mut Staging, tables: &ObjectTables) {
        self.apply_boss_damage(staging);
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = sweep_boss::update(
            &mut actor,
            sweep_boss::Inputs {
                frame: FrameTick(u64::from(self.fff0)),
                div: self.div,
                fade_idle: self.fade_timer == 0,
                hit: if self.done != 0 {
                    Some(self.boss_hit_latch != 0)
                } else {
                    None
                },
                primary: self.boss_slot_ptr_lo == 0,
                stun: HitStunContext {
                    removal_pending: self.remove_mode != 0,
                    definition_sprite: self.boss_sprite,
                    saved_sprite: self.spawn_sprite,
                    saved_state: self.saved_frame_state,
                    saved_contact_kind: ContactKind::from_raw(self.saved_type),
                },
                timer_reload: self.boss_timer_reload,
                volleys: self.door_phase,
                children_busy: [
                    Actor::from_raw_slot(&self.slots[1].b).state != 0,
                    Actor::from_raw_slot(&self.slots[2].b).state != 0,
                ],
                hover_profile: &tables.boss_picker_profile,
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
                if let Some(sound) = effects.sound1 {
                    self.sfx1 = sound;
                }
                if effects.silence_music {
                    self.music_request = Some(255);
                }
                if let Some(volleys) = effects.volleys {
                    self.door_phase = volleys;
                }
                for (index, command) in effects.children.into_iter().enumerate() {
                    if let Some(command) = command {
                        let target = &mut self.slots[index + 1];
                        let mut child = Actor::from_raw_slot(&target.b);
                        child.screen = command.screen;
                        child.sprite = command.sprite;
                        child.sprite_flip = command.flip;
                        target.b = child.to_raw_slot();
                    }
                }
            }
            Err(()) => self.note_unported(0xA0B9),
        }
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
