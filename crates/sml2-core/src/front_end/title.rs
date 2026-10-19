//! `$00`/`$01`: the title screen loader and idle.
//!
//! The loader (`$14043`, bank 5) copies `GFX_TitleScreen` to `$8000` and the
//! tilemap to `$9800`, sets `BGP=$E4`, `OBP0=$D0`, `SCY=0`, stops music
//! (`$FF`), arms the fade timer at `5/$40`, and advances to `$01`.
//!
//! The idle handler (`$1407C`) only reads input: while the fade timer's high
//! byte is still `5` it does nothing; once the high byte reaches zero it
//! starts the current attract demo (`sDemoIndex & 3`) — that is timer value
//! `$00FF`, 1089 frames in; SELECT + UP starts a chosen demo early; START opens
//! the file select (`$19`). There is no per-frame palette animation: the
//! "fade timer" is the attract-mode countdown.

use super::keys::{SELECT, START, UP};

/// `$14043`: fade low/high, i.e. the attract countdown (`5*256+64` frames).
pub const TIMER_START_LOW: u8 = 64;
pub const TIMER_START_HIGH: u8 = 5;

/// `$1407C`: BGP and OBP0 written on entry.
pub const BGP: u8 = 0xE4;
pub const OBP0: u8 = 0xD0;

/// What the title asks the host to do this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAction {
    None,
    /// START: enter the file select (`$19`).
    OpenFileSelect,
    /// The countdown expired or SELECT+UP chose a demo: start demo `index & 3`.
    StartDemo(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleScreen {
    low: u8,
    high: u8,
    frame: u32,
    demo_index: u8,
}

impl Default for TitleScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl TitleScreen {
    pub fn new() -> Self {
        Self {
            low: TIMER_START_LOW,
            high: TIMER_START_HIGH,
            frame: 0,
            demo_index: 0,
        }
    }

    /// `sFadeTimerLow/High` as one 16-bit value.
    pub fn timer(&self) -> u16 {
        ((self.high as u16) << 8) | self.low as u16
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// `sDemoIndex` (`$A0F0`): page 0 starts at `$7F00`, page 3 at `$7C00`.
    pub fn demo_index(&self) -> u8 {
        self.demo_index
    }

    pub fn set_demo_index(&mut self, index: u8) {
        self.demo_index = index & 0x03;
    }

    /// `$38CE`: a demo ended; the next one is index + 1, wrapped to 2 bits.
    pub fn next_demo(&mut self) {
        self.demo_index = (self.demo_index + 1) & 0x03;
    }

    /// One frame. The VBlank 16-bit decrement runs before the state handler.
    pub fn step(&mut self, held: u8, pressed: u8) -> TitleAction {
        self.frame = self.frame.wrapping_add(1);
        self.tick_timer();

        // `$14085`: `cp $05 / ret nc` — the first 64 frames ignore input.
        if self.high >= TIMER_START_HIGH {
            return TitleAction::None;
        }
        // `$1408B`: the countdown reached zero.
        if self.high == 0 {
            return TitleAction::StartDemo(self.demo_index & 0x03);
        }
        // `$1409F`: SELECT + UP starts the selected attract demo.
        if pressed & SELECT != 0 && held & UP != 0 {
            self.demo_index = held & 0x03;
            return TitleAction::StartDemo(self.demo_index);
        }
        // `$140B1`: START opens the file select.
        if pressed & START != 0 {
            return TitleAction::OpenFileSelect;
        }
        TitleAction::None
    }

    /// The VBlank 16-bit `sFadeTimerLow/High` decrement, clamped at zero.
    fn tick_timer(&mut self) {
        if self.low == 0 {
            if self.high == 0 {
                return; // clamped (the ROM's borrow path writes 0/0)
            }
            self.high -= 1;
            self.low = 255;
        } else {
            self.low -= 1;
        }
    }
}

#[cfg(test)]
mod tests;
