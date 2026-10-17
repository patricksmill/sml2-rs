//! Visible sprite frame finalization shared by native scene consumers.

use crate::player::Cell;

pub const MAX_SPRITE_CELLS: usize = 40;

const EMPTY_CELL: Cell = Cell {
    y: 0,
    x: 0,
    tile: 0,
    attr: 0,
};

/// A frame exceeded the original visible sprite budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteBatchOverflow {
    pub attempted: usize,
}

/// Forty visible cells plus the length of the last submitted frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteBatch {
    cells: [Cell; MAX_SPRITE_CELLS],
    previous_used: usize,
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self {
            cells: [EMPTY_CELL; MAX_SPRITE_CELLS],
            previous_used: 0,
        }
    }
}

impl SpriteBatch {
    pub fn cells(&self) -> &[Cell; MAX_SPRITE_CELLS] {
        &self.cells
    }

    pub fn previous_used(&self) -> usize {
        self.previous_used
    }

    /// Copy a complete frame, then apply the source $52E7 cleanup boundaries.
    /// Oversized frames leave both cells and the previous count untouched.
    pub fn write_frame(&mut self, frame: &[Cell]) -> Result<(), SpriteBatchOverflow> {
        let used = frame.len();
        if used > MAX_SPRITE_CELLS {
            return Err(SpriteBatchOverflow { attempted: used });
        }
        self.cells[..used].copy_from_slice(frame);
        if used < self.previous_used {
            self.cells[used..self.previous_used].fill(EMPTY_CELL);
        } else if used == self.previous_used && used < MAX_SPRITE_CELLS {
            self.cells[used].y = 0;
        }
        self.previous_used = used;
        Ok(())
    }

    /// Scene loading clears visible records without changing the previous count.
    pub fn clear_cells(&mut self) {
        self.cells.fill(EMPTY_CELL);
    }
}

#[cfg(test)]
mod tests;
