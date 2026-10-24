//! Native ending raster wobble and scanline projection services.
//!
//! Replaces SM83 cycle-counting HBlank loops with typed, deterministic
//! per-scanline projection records (`ScanlineProjection`) for modern renderers.
//!
//! Ground truth references:
//! - ROM `$68400` (`main.asm:39733..39780`): Raster wobble loop
//! - ROM `$6844F` (`main.asm:39781`): 16-entry sine table
//! - ROM `$6809E` (`main.asm:39327..39374`): Substate 18 addressing split & wobble
//! - ROM `$68267` (`main.asm:39516..39537`): Substate 19 addressing split display

#[cfg(test)]
mod tests;

/// Exact 16-entry sine displacement table from ROM `$6844F`.
/// Values represent horizontal pixel offsets written to `SCX` ($FF43).
pub const RASTER_SINE_TABLE: [i8; 16] =
    [0, 5, 9, 12, 13, 12, 9, 5, 0, -5, -9, -12, -13, -12, -9, -5];

/// Frame counter cadence mask: bits [5:2] select wave phase (advances every 4 frames).
pub const FRAME_CADENCE_MASK: u8 = 0x3C;

/// First scanline index where raster wave displacement can begin (ROM `$6840B`: `cp $10`).
/// Scanlines `<= 16` have horizontal scroll 0.
pub const RASTER_START_SCANLINE: u8 = 16;

/// Scanline where background tile addressing switches from signed to unsigned
/// (ROM `$680B1`, `$6827A`: `cp $6F`, 111 decimal).
pub const BG_SPLIT_SCANLINE: u8 = 111;

/// Total vertical scanlines on the Game Boy LCD (0..=143).
pub const SCREEN_HEIGHT_SCANLINES: usize = 144;

/// Look up the raw sine displacement offset for a given frame counter and scanline.
///
/// In SM83 assembly:
/// ```text
/// ldh_a_n $97 ; frame counter
/// and $3C
/// srl a
/// srl a       ; phase = (frame & 0x3C) >> 2
/// add scanline
/// and $0F     ; index = (phase + scanline) & 15
/// ```
#[inline]
pub fn raster_sine_offset(frame: u8, scanline: u8) -> i8 {
    let phase = (frame & FRAME_CADENCE_MASK) >> 2;
    let index = (phase.wrapping_add(scanline) & 0x0F) as usize;
    RASTER_SINE_TABLE[index]
}

/// Compute the horizontal scroll displacement (`SCX`) for a specific scanline.
///
/// Scanlines strictly greater than `RASTER_START_SCANLINE` (16) up to `raster_y` (inclusive)
/// receive the sine wave displacement. Scanlines `<= 16` or `> raster_y` have scroll offset 0.
#[inline]
pub fn scanline_scroll_x(frame: u8, scanline: u8, raster_y: u8) -> i8 {
    if scanline > RASTER_START_SCANLINE && scanline <= raster_y {
        raster_sine_offset(frame, scanline)
    } else {
        0
    }
}

/// Determine whether background tile addressing is unsigned ($8000..$8FFF, LCDC bit 4 = 1)
/// or signed ($8800..$97FF, LCDC bit 4 = 0) for a given scanline.
///
/// Scanlines `0..=110`: signed addressing (top half: credits text, Mario, ground).
/// Scanlines `111..=143`: unsigned addressing (bottom half: castle graphics).
#[inline]
pub fn scanline_unsigned_bg_tiles(scanline: u8) -> bool {
    scanline >= BG_SPLIT_SCANLINE
}

/// Calculate the byte offset within the composed tile memory block ($8000..$97FF)
/// for a given tile ID under signed or unsigned addressing.
///
/// Composed tile data layout (6144 bytes, 384 tiles):
/// - `$8000..$87FF` (offset 0x0000..0x0800): tiles 0..127 in unsigned addressing
/// - `$8800..$8FFF` (offset 0x0800..0x1000): tiles 128..255 in unsigned addressing,
///   or tiles 128..255 in signed addressing
/// - `$9000..$97FF` (offset 0x1000..0x1800): tiles 0..127 in signed addressing
#[inline]
pub fn background_tile_offset(tile_id: u8, unsigned_bg_tiles: bool) -> usize {
    if unsigned_bg_tiles {
        tile_id as usize * 16
    } else {
        0x800 + crate::video::bg_tile_offset(tile_id)
    }
}

/// Per-scanline projection parameters for renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanlineProjection {
    pub scanline: u8,
    pub scroll_x: i8,
    pub unsigned_bg_tiles: bool,
}

impl ScanlineProjection {
    /// Compute the projection parameters for a single scanline.
    pub fn compute(frame: u8, scanline: u8, raster_y: u8, split_enabled: bool) -> Self {
        Self {
            scanline,
            scroll_x: scanline_scroll_x(frame, scanline, raster_y),
            unsigned_bg_tiles: split_enabled && scanline_unsigned_bg_tiles(scanline),
        }
    }
}

/// The terminal credits handler ($68267) performs the addressing split but
/// never enters the wobble loop, even if the last streaming frame left it active.
pub fn ending_projection(
    frame: u8,
    scanline: u8,
    raster_y: u8,
    substate: u8,
) -> ScanlineProjection {
    ScanlineProjection::compute(
        frame,
        scanline,
        if substate == 19 { 13 } else { raster_y },
        substate >= 18,
    )
}

/// Project all 144 Game Boy scanlines (0..=143) for a given frame.
pub fn project_frame(
    frame: u8,
    raster_y: u8,
    split_enabled: bool,
) -> [ScanlineProjection; SCREEN_HEIGHT_SCANLINES] {
    let mut projections = [ScanlineProjection {
        scanline: 0,
        scroll_x: 0,
        unsigned_bg_tiles: false,
    }; SCREEN_HEIGHT_SCANLINES];

    for (y, proj) in projections.iter_mut().enumerate() {
        *proj = ScanlineProjection::compute(frame, y as u8, raster_y, split_enabled);
    }

    projections
}
