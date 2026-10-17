//! Native credits text service for ROM $68572.
//!
//! Renders 20-character rows of credits text into a 32x32 background tilemap,
//! mapping ASCII characters to font tile IDs and handling blank rows, phase
//! countdowns, page-completion fade timers, and the 0xFE sequence terminator.

pub const CREDITS_ROW_LEN: usize = 20;
pub const TILEMAP_WIDTH: usize = 32;
pub const CREDITS_TILEMAP_BASE_OFFSET: usize = 0x0220; // $9A20 - $9800 = row 17 * 32 = 544
pub const BLANK_TILE: u8 = 0xFF;
pub const BLANK_ROW_MARKER: u8 = 0xFF;
pub const SEQUENCE_TERMINATOR: u8 = 0xFE;
pub const PAGE_FADE_TIMER: u16 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditsTextError {
    TextTruncated { required: usize, available: usize },
    TilemapOutOfBounds { required: usize, available: usize },
}

impl std::fmt::Display for CreditsTextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextTruncated { required, available } => {
                write!(
                    f,
                    "credits text truncated: needed {required} bytes, had {available}"
                )
            }
            Self::TilemapOutOfBounds { required, available } => {
                write!(
                    f,
                    "tilemap out of bounds: needed {required} bytes, had {available}"
                )
            }
        }
    }
}

impl std::error::Error for CreditsTextError {}

/// Maps a character byte to its font tile ID per ROM $68596.
///
/// Each character is shifted by subtracting 48 (ASCII '0').
/// ASCII space (0x20 - 48 = 0xF0) maps to the blank tile (0xFF).
#[inline]
pub const fn char_to_tile(c: u8) -> u8 {
    let tile = c.wrapping_sub(48);
    if tile == 0xF0 {
        BLANK_TILE
    } else {
        tile
    }
}

/// Converts a phase value (typically 4 down to 1) to a row index (0..3)
/// using the ROM's arithmetic: `(phase ^ 0xFF).wrapping_add(5)`.
#[inline]
pub const fn phase_to_row(phase: u8) -> u8 {
    (phase ^ 0xFF).wrapping_add(5)
}

/// Computes the tilemap offset for a given phase.
#[inline]
pub const fn phase_to_tilemap_offset(phase: u8) -> usize {
    let raw_row = phase_to_row(phase);
    CREDITS_TILEMAP_BASE_OFFSET + (raw_row as usize) * TILEMAP_WIDTH
}

/// Persistent state for the credits text stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditsTextState {
    pub ptr: usize,
    pub phase: u8,
    pub done: bool,
}

impl Default for CreditsTextState {
    fn default() -> Self {
        Self {
            ptr: 0,
            phase: 4,
            done: false,
        }
    }
}

/// Result of rendering one row of credits text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditsStepResult {
    pub row_index: u8,
    pub dest_offset: usize,
    pub is_blank_row: bool,
    pub phase_remaining: u8,
    pub page_completed: bool,
    pub sequence_done: bool,
    pub fade_timer: Option<u16>,
}

impl CreditsTextState {
    pub fn new(initial_ptr: usize) -> Self {
        Self {
            ptr: initial_ptr,
            phase: 4,
            done: false,
        }
    }

    /// Resets the phase to 4 for a new page of credits text.
    pub fn start_page(&mut self) {
        self.phase = 4;
    }

    /// Renders one row from `text` into `tilemap`.
    ///
    /// Corresponds directly to ROM $68572..$685D4.
    pub fn step(
        &mut self,
        text: &[u8],
        tilemap: &mut [u8],
    ) -> Result<CreditsStepResult, CreditsTextError> {
        let row_index = phase_to_row(self.phase);
        let dest_offset = phase_to_tilemap_offset(self.phase);
        if dest_offset + CREDITS_ROW_LEN > tilemap.len() {
            return Err(CreditsTextError::TilemapOutOfBounds {
                required: dest_offset + CREDITS_ROW_LEN,
                available: tilemap.len(),
            });
        }

        if self.ptr >= text.len() {
            return Err(CreditsTextError::TextTruncated {
                required: self.ptr + 1,
                available: text.len(),
            });
        }

        let first_byte = text[self.ptr];
        let is_blank_row = first_byte == BLANK_ROW_MARKER;

        if is_blank_row {
            self.ptr += 1;
            tilemap[dest_offset..dest_offset + CREDITS_ROW_LEN].fill(BLANK_TILE);
        } else {
            if self.ptr + CREDITS_ROW_LEN > text.len() {
                return Err(CreditsTextError::TextTruncated {
                    required: self.ptr + CREDITS_ROW_LEN,
                    available: text.len(),
                });
            }
            let chunk = &text[self.ptr..self.ptr + CREDITS_ROW_LEN];
            self.ptr += CREDITS_ROW_LEN;
            for (i, &c) in chunk.iter().enumerate() {
                tilemap[dest_offset + i] = char_to_tile(c);
            }
        }

        if self.ptr < text.len() && text[self.ptr] == SEQUENCE_TERMINATOR {
            self.done = true;
        }

        self.phase = self.phase.wrapping_sub(1);
        let page_completed = self.phase == 0;
        let fade_timer = if page_completed {
            Some(PAGE_FADE_TIMER)
        } else {
            None
        };

        Ok(CreditsStepResult {
            row_index,
            dest_offset,
            is_blank_row,
            phase_remaining: self.phase,
            page_completed,
            sequence_done: self.done,
            fade_timer,
        })
    }
}

#[cfg(test)]
mod tests;
