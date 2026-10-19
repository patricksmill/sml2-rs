//! Slot conversion for ROM `$5934A` (anim 62).
//!
//! The frame body is shared with `$5902D`; see `handler_emergence_tracker`.
use super::*;

impl ObjectSim {
    pub(super) fn handler_0x5934a(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.handler_emergence_tracker(0x5934A, handler_0x5934a::STATE_THREE, st, tables);
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
