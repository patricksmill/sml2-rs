# Ending Raster Projection & Scanline Addressing Split

- **ROM addresses:**
  - Raster wobble loop: ROM `0x68400` (`main.asm:39733..39780`), bank `$1A`, CPU `$4400`
  - Sine offset table: ROM `0x6844F` (`main.asm:39781`), bank `$1A`, CPU `$444F`
  - Substate 18 split & wobble: ROM `0x6809E` (`main.asm:39327..39374`), bank `$1A`, CPU `$409E`
  - Substate 19 split display: ROM `0x68267` (`main.asm:39516..39537`), bank `$1A`, CPU `$4267`
- **Subsystem:** Ending scene presentation / Scanline video projection
- **Module:** `crates/sml2-core/src/ending_raster.rs`

## 1. Domain Overview

During the ending sequence (state 19), two hardware display effects create the visual presentation of Mario walking in front of the castle with the credits rolling:

1. **Raster Sine Wobble (`$68400`, table `$6844F`)**:
   - In substates 7, 8, 9, 10, and 18, a horizontal wave effect is applied to the screen by updating `SCX` (`$FF43`) on each scanline during HBlank.
   - The effect reads a 16-entry sine displacement table at ROM `$6844F`.
   - The lookup index into the table is:
     `index = (((frame & 0x3C) >> 2) + scanline) & 0x0F`
   - Active scanline window: The SM83 assembly waits until `LY >= 16` before writing displacements, and terminates the wobble when `LY >= sEndingRasterY`, zeroing `SCX` for subsequent scanlines.
   - Thus, scanlines strictly greater than 16 up to `sEndingRasterY` (inclusive) receive sine displacement, while scanlines `<= 16` or `> sEndingRasterY` have horizontal scroll `0`.
   - When `sEndingRasterY` is retracted to 13 (substates 6 and 10), no scanlines fall in the active range `16 < scanline <= 13`, disabling the wave effect cleanly.

2. **Dual-Mode LCD Background Addressing Split (`$6809E`, `$68267`)**:
   - In substates 18 and 19, the screen is split horizontally at scanline 111 (`$6F`):
     - Scanlines `0..=110`: `LCDC = 195` (`$C3`, bit 4 = 0), selecting signed background tile addressing (`$8800..$97FF`). This region contains Mario, the walking ground, and the rolling credits text (`$9A20`).
     - Scanlines `111..=143`: `LCDC = 211` (`$D3`, bit 4 = 1), selecting unsigned background tile addressing (`$8000..$8FFF`). This region contains the castle graphic base.
   - On original hardware, this is accomplished by spinning in a CPU loop until `LY >= 111` and switching `LCDC`. In native Rust, this is cleanly represented as a per-scanline projection parameter (`unsigned_bg_tiles: bool`) for the renderer.

## 2. Exact ROM Table and Constants

### Sine Table at ROM `$6844F` (16 bytes)
```text
Offset:  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
Hex:     00 05 09 0C 0D 0C 09 05 00 FB F7 F4 F3 F4 F7 FB
Signed:   0  5  9 12 13 12  9  5  0 -5 -9 -12 -13 -12 -9 -5
```

### Scanline Thresholds
- `RASTER_START_SCANLINE = 16`: Scanlines `<= 16` have scroll offset 0.
- `BG_SPLIT_SCANLINE = 111`: Scanlines `>= 111` switch to unsigned background tile addressing.
- `FRAME_CADENCE_MASK = 0x3C`: Frame counter bits [5:2] determine wave phase (updates every 4 frames, wraps mod 16).
- `SCREEN_HEIGHT_SCANLINES = 144`: Game Boy LCD vertical resolution (scanlines 0..=143).

## 3. Native Rust Architecture

```rust
pub const RASTER_SINE_TABLE: [i8; 16] = [
    0, 5, 9, 12, 13, 12, 9, 5, 0, -5, -9, -12, -13, -12, -9, -5,
];

pub fn raster_sine_offset(frame: u8, scanline: u8) -> i8;
pub fn scanline_scroll_x(frame: u8, scanline: u8, raster_y: u8) -> i8;
pub fn scanline_unsigned_bg_tiles(scanline: u8) -> bool;
pub fn background_tile_offset(tile_id: u8, unsigned_bg_tiles: bool) -> usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanlineProjection {
    pub scanline: u8,
    pub scroll_x: i8,
    pub unsigned_bg_tiles: bool,
}

pub fn project_frame(
    frame: u8,
    raster_y: u8,
    split_enabled: bool,
) -> [ScanlineProjection; SCREEN_HEIGHT_SCANLINES as usize];
```

## 4. Verification Plan

1. **Table Fidelity**: Verify all 16 entries match ROM bytes and signed conversions.
2. **Phase Cadence**: Test 4-frame cadence (`frame & 0x3C >> 2`) and wrapping.
3. **Threshold Gates**:
   - `scanline <= 16`: Always 0 offset.
   - `scanline > raster_y`: Always 0 offset.
   - `16 < scanline <= raster_y`: Exact sine offset.
   - Retracted `raster_y = 13`: All 144 scanlines 0 offset.
4. **Addressing Split**:
   - `scanline < 111`: Signed addressing (`false`).
   - `scanline >= 111`: Unsigned addressing (`true`).
5. **Tile Offsets**: Compare signed and unsigned VRAM offset calculation across 0..=255.
