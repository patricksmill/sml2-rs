//! Slot conversion for ROM `$58BCB` (anim 40).
//!
//! The frame body is shared with `$58BBA`; see `handler_vertical_platform`.
use super::*;

impl ObjectSim {
    pub(super) fn handler_0x58bcb(&mut self, st: &mut Staging) {
        self.handler_vertical_platform(0x58BCB, handler_0x58bcb::ANIM_40, st);
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/short_vertical.rs"]
mod tests;
