//! Slot compatibility and rider displacement for the native rising platform.
use super::{rising_platform, ObjectSim, Staging};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_rising_platform(&mut self, staging: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = rising_platform::update(&mut actor);
        staging.b = actor.to_raw_slot();
        match result {
            Ok(rising_platform::RisingPlatformEffect::Rising { carry_dy: Some(dy) }) => {
                let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            }
            Err(_) => self.note_unported(0x58E12),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "integration.rs"]
mod tests;
