//! Zone loader decoration lists, bank $0F. `$3E2A2` copies x/y/id triples;
//! the four-byte hub slot names in the disassembly alias this same storage.
use super::MapSim;

type ZoneSprite = (u8, u8, u8);

// ROM $0F:$65DE.
const PUMPKIN: &[ZoneSprite] = &[
    (0x54, 0x35, 0x0F),
    (0x78, 0x34, 0x12),
    (0x18, 0x10, 0x16),
    (0x56, 0x65, 0xFF),
    (0x6A, 0x4F, 0xFF),
    (0x3A, 0x4F, 0xFF),
    (0x57, 0x43, 0xFF),
];

// ROM $0F:$65BA.
const TREE: &[ZoneSprite] = &[
    (0x44, 0x11, 0x00),
    (0x80, 0x58, 0x04),
    (0x1C, 0x32, 0x06),
    (0x57, 0x6D, 0xFF),
    (0x34, 0x7A, 0xFF),
    (0x5C, 0x7A, 0xFF),
    (0x73, 0x4E, 0xFF),
    (0x30, 0x4E, 0xFF),
    (0x4E, 0x2E, 0xFF),
    (0x80, 0x84, 0xFF),
];

// ROM $0F:$662C.
const MACRO: &[ZoneSprite] = &[
    (0x17, 0x49, 0xFF),
    (0x00, 0x00, 0xFF),
    (0x00, 0x00, 0xFF),
    (0x00, 0x00, 0xFF),
    (0x17, 0x81, 0xFF),
    (0x27, 0x71, 0xFF),
    (0x5B, 0x71, 0xFF),
    (0x63, 0x49, 0xFF),
];

// ROM $0F:$65FF.
const SPACE: &[ZoneSprite] = &[
    (0x24, 0x28, 0x49),
    (0x20, 0x47, 0x43),
    (0x20, 0x20, 0x43),
    (0x40, 0x81, 0x43),
    (0x4F, 0x5D, 0xFF),
    (0x3F, 0x2D, 0xFF),
    (0xE0, 0xE0, 0x46),
    (0xF8, 0x90, 0xFF),
    (0xF8, 0x90, 0xFF),
    (0x56, 0x56, 0xFF),
    (0x97, 0x2D, 0xFF),
];

// ROM $0F:$6680.
const TURTLE: &[ZoneSprite] = &[
    (0x58, 0x48, 0xFF),
    (0x18, 0x78, 0x2A),
    (0x70, 0x70, 0x2B),
    (0x90, 0x60, 0x2C),
    (0x4F, 0x2D, 0xFF),
    (0x77, 0x65, 0xFF),
    (0x5F, 0x7D, 0xFF),
    (0x80, 0x40, 0x2E),
    (0x30, 0x50, 0x2E),
    (0xA0, 0x40, 0x32),
    (0x94, 0x10, 0xFF),
];

// ROM $0F:$664D.
const MARIO: &[ZoneSprite] = &[
    (0x46, 0x24, 0x1C),
    (0x5A, 0x24, 0x1D),
    (0x4C, 0x10, 0x23),
    (0x4A, 0x2C, 0x24),
    (0x5E, 0x2C, 0x24),
    (0x38, 0x77, 0x25),
    (0x72, 0x2B, 0x1E),
    (0x42, 0x7B, 0xFF),
    (0x54, 0x65, 0xFF),
    (0x54, 0x52, 0xFF),
    (0x54, 0x16, 0xFF),
];

// ROM $0F:$66AA.
const WARIO_CASTLE: &[ZoneSprite] = &[
    (0x19, 0x20, 0x34),
    (0x50, 0x21, 0x35),
    (0x34, 0x1C, 0xFF),
    (0x70, 0x62, 0x39),
];

// ROM $0F:$66BC.
const CASTLE: &[ZoneSprite] = &[
    (0x08, 0x10, 0x42),
    (0x80, 0x14, 0x42),
    (0x80, 0x88, 0x5F),
    (0x8D, 0x90, 0x60),
    (0x95, 0x90, 0x60),
    (0x9D, 0x90, 0x60),
];

impl MapSim {
    pub(super) fn seed_zone_sprites(&mut self, screen: u8, exit_tile_flag: u8) {
        let list = match screen {
            1 => PUMPKIN,
            2 => TREE,
            3 => MACRO,
            4 => SPACE,
            5 => TURTLE,
            6 => MARIO,
            7 => WARIO_CASTLE,
            8 => CASTLE,
            _ => return,
        };
        self.zone_slots = [(0xFF, 0xFF, 0xFF); 16];
        self.zone_slots[..list.len()].copy_from_slice(list);
        match screen {
            1 if self.level_progress[9] & 0x80 != 0 => self.zone_slots[2].2 = 0xFF,
            2 if self.level_progress[5] & 0x80 != 0 => self.zone_slots[0].2 = 0xFF,
            5 => {
                // $3D6B7 precedes $3D69C: a fresh secret exit uses $7D,
                // while an already unlocked branch uses $81.
                if self.level_progress[22] & 1 != 0 {
                    self.zone_slots[10].2 = 129;
                } else if exit_tile_flag != 0 && self.mario_dead == 0 {
                    self.zone_slots[10].2 = 125;
                }
            }
            6 => {
                self.zone_slots[5].0 = 74; // sMapSprite3Id, $A6C4
                let collected = self.level_progress[29] & 0x80 != 0;
                self.zone_slots[2].0 = if collected { 92 } else { 76 };
                self.zone_slots[2].1 = if collected { 19 } else { 16 };
            }
            _ => {}
        }
    }

    pub(super) fn seed_zone_path_markers(&mut self, screen: u8) {
        // $3DB00 writes $0E for bit 7 set, $FF otherwise, every third byte.
        let (first_slot, first_progress, count) = match screen {
            1 => (3, 6, 4),
            2 => (4, 1, 5),
            3 => (4, 11, 4),
            4 => (4, 16, 2),
            5 => (4, 21, 3),
            6 => (7, 26, 4),
            _ => return,
        };
        for i in 0..count {
            self.zone_slots[first_slot + i].2 = marker(self.level_progress[first_progress + i]);
        }
        if screen == 3 {
            self.zone_slots[0].2 = marker(self.level_progress[35]);
        } else if screen == 4 {
            self.zone_slots[10].2 = marker(self.level_progress[41]);
        }
    }
}

fn marker(progress: u8) -> u8 {
    if progress & 0x80 != 0 {
        14
    } else {
        0xFF
    }
}

#[cfg(test)]
#[path = "zone_sprite_loading_tests.rs"]
mod tests;
