//! How big a level's world is and how positions wrap in it.
//!
//! The engine addresses the world in 256-px *screens*: a position's high byte
//! is its screen number. The ROM keeps 12 bits of every camera coordinate
//! (`and $0F` on the high byte), so its world is 16 screens on each axis and
//! screens 16+ alias onto 0-15. Its layout fills 16x3 of them (256x48 cells);
//! a screen row past the third reads past the 48-byte tables, i.e. 0.
//!
//! [`World::ROM`] reproduces that exactly and is what every original level
//! uses. Expanded levels keep the full high byte instead.

use crate::camera::Room;
use crate::level::{LAYOUT_HEIGHT, LAYOUT_WIDTH};

/// Pixels per screen on each axis (one high-byte step).
pub const SCREEN_PX: u16 = 256;

/// Cells per screen on each axis.
pub const SCREEN_CELLS: usize = 16;

/// A level's extent in screens and the wrap its positions follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct World {
    /// Layout width in screens (the ROM: 16).
    pub screens_wide: u16,
    /// Layout height in screens (the ROM: 3).
    pub screens_high: u16,
    /// Mask applied to wrapped positions (the ROM: `$0FFF`, 12 bits).
    pub position_mask: u16,
}

impl World {
    /// The original game's world: a 16x3-screen layout in a 12-bit space.
    pub const ROM: World = World {
        screens_wide: 16,
        screens_high: 3,
        position_mask: 0x0FFF,
    };

    /// `v` after the world's wrap (`and $0F` on the high byte for the ROM).
    pub const fn wrap(self, v: u16) -> u16 {
        v & self.position_mask
    }

    /// The screen number of a position: its high byte after the wrap.
    pub const fn screen(self, v: u16) -> usize {
        (self.wrap(v) >> 8) as usize
    }

    /// Index of the screen holding `(x, y)` in a per-screen table such as
    /// `sScreenEdgeMap` or `sSceneMap` (row-major, [`Self::screens_wide`]
    /// per row). Rows past the layout index past the table.
    pub const fn screen_index(self, x: u16, y: u16) -> usize {
        self.screen(y) * self.screens_wide as usize + self.screen(x)
    }

    /// Entries in a per-screen table.
    pub const fn screens(self) -> usize {
        self.screens_wide as usize * self.screens_high as usize
    }

    /// The whole layout in world pixels.
    pub const fn room(self) -> Room {
        Room {
            left: 0,
            top: 0,
            right: self.screens_wide as i32 * SCREEN_PX as i32,
            bottom: self.screens_high as i32 * SCREEN_PX as i32,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::ROM
    }
}

const _: () = assert!(World::ROM.screens_wide as usize * SCREEN_CELLS == LAYOUT_WIDTH);
const _: () = assert!(World::ROM.screens_high as usize * SCREEN_CELLS == LAYOUT_HEIGHT);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rom_world_keeps_twelve_bits_and_aliases_screen_sixteen_onto_zero() {
        let w = World::ROM;
        assert_eq!(w.wrap(0x1234), 0x0234);
        assert_eq!(w.wrap(0xFFFF), 0x0FFF);
        assert_eq!(w.screen(0x10F0), 0);
        assert_eq!(w.screen(0x0F00), 15);
    }

    #[test]
    fn rom_screen_indexes_match_the_engines_sixteen_wide_table() {
        let w = World::ROM;
        // `(Yhi & $0F) * 16 + (Xhi & $0F)`
        assert_eq!(w.screen_index(0x0540, 0x01C0), 16 + 5);
        assert_eq!(w.screen_index(0x1F00, 0x1200), 2 * 16 + 15);
        // Row 3 (and the aliases above it) index past the 48-byte table.
        assert!(w.screen_index(0, 0x0300) >= w.screens());
        assert_eq!(w.screens(), 48);
    }

    #[test]
    fn the_rom_room_is_the_256_by_48_cell_layout() {
        let r = World::ROM.room();
        assert_eq!((r.left, r.top, r.right, r.bottom), (0, 0, 4096, 768));
    }

    #[test]
    fn a_full_byte_world_keeps_screens_past_fifteen() {
        let w = World {
            screens_wide: 40,
            screens_high: 5,
            position_mask: 0xFFFF,
        };
        assert_eq!(w.screen(0x2710), 0x27);
        assert_eq!(w.screen_index(0x2710, 0x0480), 4 * 40 + 0x27);
    }
}
