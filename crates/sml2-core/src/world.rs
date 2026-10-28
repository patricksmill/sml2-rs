//! How big a level's world is and how positions wrap in it.
//!
//! The engine addresses the world in 256-px *screens*: a position's high byte
//! is its screen number. The ROM keeps 12 bits of every camera coordinate
//! (`and $0F` on the high byte), so its world is 16 screens on each axis and
//! screens 16+ alias onto 0-15. Its layout fills 16x3 of them (256x48 cells);
//! a screen row past the third reads past the 48-byte tables, i.e. 0.
//!
//! [`World::ROM`] reproduces that exactly and is what every original level
//! uses. Expanded levels ([`World::expanded`]) keep the full high byte instead.

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
    /// Rows past the layout read WRAM's echo: `$B000 + row*$100` for rows
    /// `$30-$4D` lands in `$E000-$FDFF`, the mirror of `$C000-$DDFF`, i.e.
    /// rows `$10-$2D`. The ROM's player probes see it; expanded worlds have
    /// no echo.
    pub wram_echo: bool,
}

impl World {
    /// The original game's world: a 16x3-screen layout in a 12-bit space.
    pub const ROM: World = World {
        screens_wide: 16,
        screens_high: 3,
        position_mask: 0x0FFF,
        wram_echo: true,
    };

    /// Widest expanded world: every X high byte is a screen.
    pub const MAX_SCREENS_WIDE: u16 = 256;
    /// Tallest expanded world: Y high byte `$FF` means "above the level"
    /// (`PlayerSim`'s `y >> 8 == $FF` checks), so it is never a screen.
    pub const MAX_SCREENS_HIGH: u16 = 255;

    /// A world with the full 16-bit position space and no WRAM echo, for
    /// expanded levels.
    pub fn expanded(screens_wide: u16, screens_high: u16) -> Result<World, String> {
        if !(1..=Self::MAX_SCREENS_WIDE).contains(&screens_wide)
            || !(1..=Self::MAX_SCREENS_HIGH).contains(&screens_high)
        {
            return Err(format!(
                "world {screens_wide}x{screens_high} screens: limits are 1-{}x1-{}",
                Self::MAX_SCREENS_WIDE,
                Self::MAX_SCREENS_HIGH
            ));
        }
        Ok(World {
            screens_wide,
            screens_high,
            position_mask: 0xFFFF,
            wram_echo: false,
        })
    }

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

    /// The `(column, row)` cell holding world pixel `(x, y)` after the wrap
    /// (the ROM: `(x >> 4) & $FF`, `(y >> 4) & $FF`). It can lie past the
    /// layout.
    pub const fn cell(self, x: u16, y: u16) -> (usize, usize) {
        ((self.wrap(x) >> 4) as usize, (self.wrap(y) >> 4) as usize)
    }

    /// The layout row a player probe of `row` reads (see [`Self::wram_echo`]).
    pub const fn echo_row(self, row: usize) -> usize {
        if self.wram_echo && row >= 0x30 && row < 0x4E {
            row - 0x20
        } else {
            row
        }
    }

    /// Row-major layout index of a cell, the key of `PlayerSim::layout_writes`
    /// (the ROM: `row * $100 + col`, the offset from `$B000`). Rows past the
    /// layout still have a key: the ROM's write lands in WRAM past the map.
    /// A column past the layout's width has none, since it would alias the
    /// next row (the ROM's columns never exceed its width).
    pub const fn cell_key(self, col: usize, row: usize) -> Option<u32> {
        if col < self.cells_wide() {
            Some((row * self.cells_wide() + col) as u32)
        } else {
            None
        }
    }

    /// Layout width in cells (the row stride).
    pub const fn cells_wide(self) -> usize {
        self.screens_wide as usize * SCREEN_CELLS
    }

    /// Layout height in cells.
    pub const fn cells_high(self) -> usize {
        self.screens_high as usize * SCREEN_CELLS
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
            world: (
                self.screens_wide as i32 * SCREEN_PX as i32,
                self.screens_high as i32 * SCREEN_PX as i32,
            ),
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
        assert_eq!(r.world, (4096, 768));
    }

    #[test]
    fn expanded_worlds_reserve_the_above_the_level_screen_row() {
        assert!(World::expanded(256, 255).is_ok());
        assert!(World::expanded(256, 256).is_err(), "Y high byte $FF");
        assert!(World::expanded(257, 3).is_err());
        assert!(World::expanded(0, 3).is_err());
        let w = World::expanded(16, 4).unwrap();
        assert_eq!(
            (w.position_mask, w.wram_echo, w.screens()),
            (0xFFFF, false, 64)
        );
    }

    #[test]
    fn a_full_byte_world_keeps_screens_past_fifteen() {
        let w = World::expanded(40, 5).unwrap();
        assert_eq!(w.screen(0x2710), 0x27);
        assert_eq!(w.screen_index(0x2710, 0x0480), 4 * 40 + 0x27);
        assert_eq!(w.cell(0x2710, 0x0480), (0x271, 0x48));
        assert_eq!(w.cell_key(0x271, 0x48), Some(0x48 * 640 + 0x271));
        assert_eq!(w.cell_key(640, 0), None, "past the right edge");
        assert_eq!(w.echo_row(0x30), 0x30, "no WRAM echo");
    }

    #[test]
    fn rom_cells_match_the_b000_sampler_and_its_wram_echo() {
        let w = World::ROM;
        // `$1EFA`: `$B000 + ((y-16)>>4)*$100 + ((x-8)>>4)`, bytes masked.
        assert_eq!(w.cell(0x1F08, 0x0110), (0xF0, 0x11));
        assert_eq!(w.cell_key(0xF0, 0x11), Some(0x11F0));
        assert_eq!(w.cell_key(0, 48), Some(0x3000), "past the map, still keyed");
        assert_eq!(w.cell_key(0xFF, 0xFF), Some(0xFFFF));
        let rows = [0x2F, 0x30, 0x4D, 0x4E].map(|r| w.echo_row(r));
        assert_eq!(rows, [0x2F, 0x10, 0x2D, 0x4E]);
    }
}
