//! Native ending sprite composition from ROM $6845F/$68492/$684C5/$684E0.
use crate::actor::{FrameTick, ScreenPosition};
use crate::object::ObjectTables;
use crate::player::{append_cells, Cell, CellStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingEndingFrame(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndingPose {
    pub frame: u8,
    pub position: ScreenPosition,
    pub style: CellStyle,
}

impl EndingPose {
    pub fn append(
        self,
        tables: &ObjectTables,
        cells: &mut Vec<Cell>,
    ) -> Result<(), MissingEndingFrame> {
        append_cells(tables, self.frame, self.position, self.style, cells)
            .ok_or(MissingEndingFrame(self.frame))
    }
}

fn walking_pose(timer: &mut u8, credits_y: u8, paired: bool) -> EndingPose {
    *timer = timer.wrapping_add(1);
    if *timer >= 48 {
        *timer = 0;
    }
    EndingPose {
        frame: (if paired { 0xE8 } else { 0xE5 }) + (*timer >> 4),
        position: ScreenPosition {
            x: 84,
            y: credits_y,
        },
        style: CellStyle {
            force_palette_one: !paired,
            behind_background: !paired,
        },
    }
}

/// Advance the persistent walk timer before looking up the sprite asset.
pub fn append_walk(
    tables: &ObjectTables,
    timer: &mut u8,
    credits_y: u8,
    cells: &mut Vec<Cell>,
) -> Result<(), MissingEndingFrame> {
    walking_pose(timer, credits_y, false).append(tables, cells)
}

/// Append both source-order poses ($68492 falls through to $684C5).
/// The timer advances even on failure; a missing secondary retains the first pose.
pub fn append_pair(
    tables: &ObjectTables,
    timer: &mut u8,
    credits_y: u8,
    cells: &mut Vec<Cell>,
) -> Result<(), MissingEndingFrame> {
    walking_pose(timer, credits_y, true).append(tables, cells)?;
    append_secondary(tables, credits_y, false, cells)
}

/// The secondary pose inherits priority from its caller, with no palette override.
pub fn secondary_pose(credits_y: u8, behind_background: bool) -> EndingPose {
    EndingPose {
        frame: 0xEB,
        position: ScreenPosition {
            x: 79,
            y: credits_y.wrapping_add(52),
        },
        style: CellStyle {
            force_palette_one: false,
            behind_background,
        },
    }
}

pub fn append_secondary(
    tables: &ObjectTables,
    credits_y: u8,
    behind_background: bool,
    cells: &mut Vec<Cell>,
) -> Result<(), MissingEndingFrame> {
    secondary_pose(credits_y, behind_background).append(tables, cells)
}

/// Request-channel effect only; the future sound consumer owns playback/ticking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndingSoundEffect {
    RequestSound1(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurstComposition {
    pub sound_effect: Option<EndingSoundEffect>,
    pub sprites: Result<(), MissingEndingFrame>,
}

pub fn burst_pose(frame: FrameTick) -> EndingPose {
    const FRAMES: [u8; 8] = [0xEB, 0xEC, 0xED, 0xEE, 0xEF, 0xEC, 0xED, 0xEE];
    EndingPose {
        frame: FRAMES[((frame.0 & 0x1C) >> 2) as usize],
        position: ScreenPosition { x: 79, y: 96 },
        style: CellStyle::default(),
    }
}

/// Sound is requested before sprite lookup in the source. Preserve that effect
/// in the outcome even if the mapping is missing; callers must inspect `sprites`.
#[must_use]
pub fn append_burst(
    tables: &ObjectTables,
    frame: FrameTick,
    cells: &mut Vec<Cell>,
) -> BurstComposition {
    let sound_effect = (frame.0 & 1 != 0).then_some(EndingSoundEffect::RequestSound1(24));
    let sprites = burst_pose(frame).append(tables, cells);
    BurstComposition {
        sound_effect,
        sprites,
    }
}

#[cfg(test)]
mod tests;
