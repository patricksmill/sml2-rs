//! World and slot conversion boundary for boss anim 19.
use super::{boss_stun::HitStunContext, circuit_boss, ObjectSim, Staging};
use crate::actor::{Actor, ContactKind, FrameTick};

impl ObjectSim {
    pub(super) fn handler_circuit_boss(&mut self, staging: &mut Staging) {
        self.apply_boss_damage(staging);
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = circuit_boss::update(
            &mut actor,
            circuit_boss::Inputs {
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
                if let Some(sound) = effects.sound2 {
                    self.sfx2 = sound;
                }
            }
            Err(()) => self.note_unported(0xA372),
        }
    }
}

#[cfg(test)]
#[path = "circuit_boss_adapter_tests.rs"]
mod tests;
