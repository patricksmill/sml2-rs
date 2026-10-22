//! Blocking timing of `$Bx` tile transfers (`$216C -> $2202 -> $22B9`).
use super::SceneVm;
use crate::vram_transfer::TRANSFER_BURST_MASK;

impl SceneVm {
    pub(super) fn wait_for_tile_transfer(&mut self, opcode: u8) {
        // `$2202/$222E/$224E`: B1 copies $600 bytes to $9200, B3 copies
        // $400 to $8B00, and every other low nibble copies $380 to $8B00.
        let length: u16 = match opcode & 0x0F {
            1 => 0x600,
            3 => 0x400,
            _ => 0x380,
        };
        // `$22D5` waits for every 64-byte burst serviced by `$26D7`.
        // `$21E8` then waits once more. The opcode-fetch step spends the
        // first transfer wait, leaving `bursts` waits before the next opcode.
        self.opcode_wait = (length / (TRANSFER_BURST_MASK + 1)) as u8;
        self.opcode_transition_tail = false;
    }
}

#[cfg(test)]
#[path = "scene_transfer_tests.rs"]
mod tests;
