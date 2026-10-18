//! Damage, slot conversion and child commands for boss anim 14.
use super::{boss_stun::HitStunContext, patrol_boss, ObjectSim, Staging};
use crate::actor::{Actor, ContactKind, FrameTick};

impl ObjectSim {
    pub(super) fn handler_patrol_boss(&mut self, staging: &mut Staging) {
        self.apply_boss_damage(staging);
        let mut actor = Actor::from_raw_slot(&staging.b);
        let inputs = patrol_boss::Inputs {
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
            children_busy: [
                Actor::from_raw_slot(&self.slots[1].b).state != 0,
                Actor::from_raw_slot(&self.slots[2].b).state != 0,
            ],
            scene_slot_parity: self.scene_slot_parity,
        };
        let result = patrol_boss::update(&mut actor, inputs);
        staging.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if let Some(saved) = effects.saved_hit {
                    self.spawn_sprite = saved.sprite;
                    self.saved_frame_state = saved.state;
                    self.saved_type = saved.contact.raw();
                }
                if let Some(sound) = effects.sound1 {
                    self.sfx1 = sound;
                }
                if effects.silence_music {
                    self.music_request = Some(0xFF);
                }
                if let Some(parity) = effects.scene_slot_parity {
                    self.scene_slot_parity = parity;
                }
                if let Some(command) = effects.child {
                    let target = &mut self.slots[1 + command.slot.index()];
                    let mut child = Actor::from_raw_slot(&target.b);
                    child.screen = command.screen;
                    child.sprite = command.sprite;
                    target.b = child.to_raw_slot();
                }
            }
            Err(_) => self.note_unported(0x9EF4),
        }
    }
}

#[cfg(test)]
#[path = "patrol_boss_adapter_tests.rs"]
mod tests;
