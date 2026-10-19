//! Slot conversion for ROM `$5945A` (anim 66).
//!
//! The frame body is shared with `$59431`; see `handler_landing_walker`.
use super::*;

impl ObjectSim {
    pub(super) fn handler_0x5945a(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.handler_landing_walker(0x5945A, handler_0x5945a::ANIM_66, st, layout);
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
