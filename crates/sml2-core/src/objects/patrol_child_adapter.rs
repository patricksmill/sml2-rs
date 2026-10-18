//! Compatibility boundary for anim 15: damage, screen targets and facing output.
use super::{patrol_child, ObjectSim, Staging};
use crate::actor::{Actor, FrameTick, ScreenPosition};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_patrol_child(&mut self, staging: &mut Staging) {
        // Removal mode 2 bypasses `$9285` even when interaction flags are set.
        if self.remove_mode != 2 {
            self.apply_boss_damage(staging);
        }
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = patrol_child::update(
            &mut actor,
            patrol_child::Inputs {
                frame: FrameTick(u64::from(self.fff0)),
                player: ScreenPosition {
                    x: self.player_screen_x,
                    y: self.player_screen_y,
                },
                remove_mode: self.remove_mode,
                reward_slot: self.boss_slot_ptr_lo == 0x40,
                hit_processed: self.done != 0,
            },
        );
        staging.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if let Some(relation) = effects.target_relation {
                    self.facing = if relation == Ordering::Less { 255 } else { 1 };
                }
            }
            Err(_) => self.note_unported(0xA029),
        }
    }
}

#[cfg(test)]
#[path = "patrol_child_adapter_tests.rs"]
mod tests;
