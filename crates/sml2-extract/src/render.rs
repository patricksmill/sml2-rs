//! PNG output for the extracted assets: raw tile sheets, per-level world
//! renders and metatile sheets.
//!
//! Shade convention: a Game Boy 2bpp color value 0..3 maps to grey
//! `[255, 170, 85, 0]`; level renders additionally pass through the level
//! header's BGP palette (`color c -> SHADES[(bgp >> 2c) & 3]`).

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use png::{BitDepth, ColorType, Encoder};

use sml2_core::object::MappingEntry;

pub use sml2_core::video::{bgp_shades, tile_colors, BG_VRAM_LEN, SHADES};

/// Write an 8-bit greyscale PNG.
pub fn write_gray_png(path: &Path, width: u32, height: u32, data: &[u8]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = Encoder::new(BufWriter::new(file), width, height);
    enc.set_color(ColorType::Grayscale);
    enc.set_depth(BitDepth::Eight);
    let mut writer = enc
        .write_header()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writer
        .write_image_data(data)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

/// Render a 2bpp blob as a sheet of 8x8 tiles, `tiles_per_row` wide.
pub fn write_tile_sheet(path: &Path, data: &[u8], tiles_per_row: usize) -> Result<(), String> {
    if data.is_empty() || !data.len().is_multiple_of(16) {
        return Err(format!(
            "{}: not a whole number of 16-byte tiles ({} bytes)",
            path.display(),
            data.len()
        ));
    }
    let n = data.len() / 16;
    let cols = tiles_per_row.min(n);
    let rows = n.div_ceil(cols);
    let (w, h) = (cols * 8, rows * 8);
    let mut img = vec![0u8; w * h];
    for t in 0..n {
        let mut b = [0u8; 16];
        b.copy_from_slice(&data[t * 16..t * 16 + 16]);
        let colors = tile_colors(&b);
        let (cx, cy) = ((t % cols) * 8, (t / cols) * 8);
        for y in 0..8 {
            for x in 0..8 {
                img[(cy + y) * w + cx + x] = SHADES[colors[y * 8 + x] as usize];
            }
        }
    }
    write_gray_png(path, w as u32, h as u32, &img)
}

/// Fetch one tile from the composed BG block (`$8800-$97FF`, LCDC bit 4 = 0:
/// ids `< 128` read `$9000 + t*16`, `>= 128` read `$8800 + (t-128)*16`).
fn decode_bg_tile(bg_vram: &[u8], tid: usize) -> [u8; 64] {
    let addr = if tid < 128 {
        0x9000 + tid * 16
    } else {
        0x8800 + (tid - 128) * 16
    };
    let off = addr - 0x8800;
    let mut b = [0u8; 16];
    b.copy_from_slice(&bg_vram[off..off + 16]);
    tile_colors(&b)
}

/// One metatile's 4 tile ids from a 512-byte metatile table.
fn metatile_quad(metatiles: &[u8], id: usize) -> [u8; 4] {
    [
        metatiles[id * 4],
        metatiles[id * 4 + 1],
        metatiles[id * 4 + 2],
        metatiles[id * 4 + 3],
    ]
}

/// Blits metatiles into a `w`-wide greyscale image, through `shades`.
struct Blit<'a> {
    img: &'a mut [u8],
    w: usize,
    shades: [u8; 4],
    grey_cache: Vec<Option<[u8; 64]>>,
    bg_vram: &'a [u8],
}

impl Blit<'_> {
    /// Draw one metatile (4 bytes: TL, TR, BL, BR tile ids) at (cx, cy).
    fn metatile(&mut self, cx: usize, cy: usize, quad: [u8; 4]) {
        for (k, tid) in quad.iter().enumerate() {
            let tid = *tid as usize;
            let (shades, bg_vram) = (self.shades, self.bg_vram);
            let gt = self.grey_cache[tid].get_or_insert_with(|| {
                let c = decode_bg_tile(bg_vram, tid);
                let mut g = [0u8; 64];
                for (i, o) in g.iter_mut().enumerate() {
                    *o = shades[c[i] as usize];
                }
                g
            });
            let (dx, dy) = (k % 2, k / 2);
            let px0 = cx * 16 + dx * 8;
            let py0 = cy * 16 + dy * 8;
            for y in 0..8 {
                let dst = (py0 + y) * self.w + px0;
                self.img[dst..dst + 8].copy_from_slice(&gt[y * 8..y * 8 + 8]);
            }
        }
    }
}

/// Render a level's sprite sheet: one 64x96 cell per sprite id (16 per row),
/// but only for the sprite ids this level actually spawns — drawn from the
/// level's composed tile block. Other ids' art belongs to other zones'
/// enemy-set files and is omitted.
///
/// OBJ addressing (LCDC bit 2 = 0 — verified in-game: rLCDC = $E3): tile `t`
/// lives at VRAM `$8000 + t*16`; ids `>= $80` therefore land in the composed
/// block at `(t-$80)*16` while `t < $80` refers to Mario's `$8000-$87FF`
/// graphics and is drawn as a placeholder. Mapping offsets are signed and are
/// added to the cell origin; the palette is OBP1 (see below), and each
/// mapping attr's own flips ($20 X / $40 Y) are applied.
pub fn write_sprite_sheet(
    path: &Path,
    mappings: &[Vec<MappingEntry>],
    used: &[bool],
    bg_vram: &[u8],
    obp1: u8,
) -> Result<(), String> {
    // Cell sized to the data's offset extents (x -25..18, y -64..4, plus one
    // 8px tile each way) so no sprite piece is clipped.
    let (cell_w, cell_h, cols) = (64usize, 96usize, 16usize);
    let (ox, oy) = (32i32, 72i32);
    let rows = mappings.len().div_ceil(cols);
    let (w, h) = (cols * cell_w, rows * cell_h);
    let mut img = vec![255u8; w * h];
    let pal1 = bgp_shades(obp1);
    for (id, entries) in mappings.iter().enumerate() {
        // Only sprite ids this level's object stream actually spawns: other
        // ids reference tiles whose art belongs to other zones' enemy-set
        // files, which are not loaded here.
        if !used.get(id).copied().unwrap_or(false) {
            continue;
        }
        let cx = ((id % cols) * cell_w) as i32;
        let cy = ((id / cols) * cell_h) as i32;
        for e in entries {
            let colors = if e.tile >= 0x80 {
                let off = (e.tile as usize - 0x80) * 16;
                let mut b = [0u8; 16];
                b.copy_from_slice(&bg_vram[off..off + 16]);
                tile_colors(&b)
            } else {
                // Tiles < $80 reference the $8000-$87FF region (Mario's
                // graphics in-level; map icons on the overworld screens are
                // context-dependent) — draw a checkerboard placeholder.
                let mut c = [1u8; 64];
                for (i, o) in c.iter_mut().enumerate() {
                    *o = if ((i / 8 / 2) + (i % 8 / 2)) % 2 == 0 {
                        1
                    } else {
                        3
                    };
                }
                c
            };
            // In-game the palette select is bit 4 of sSpriteFlipFlags (XOR of
            // slot+12/+13/+14, XORed into every OAM attr by $C000): the
            // live-verified spawn rendered via OBP1, so sheets use it.
            let shades = &pal1;
            let px0 = cx + ox + e.x as i32;
            let py0 = cy + oy + e.y as i32;
            for y in 0..8 {
                for x in 0..8 {
                    // flips mirror the SOURCE pixel; the output position stays
                    // at (x, y) within the tile.
                    let src_x = if e.attr & 0x20 != 0 { 7 - x } else { x };
                    let src_y = if e.attr & 0x40 != 0 { 7 - y } else { y };
                    let color = colors[src_y * 8 + src_x];
                    if color == 0 {
                        continue; // color 0 is transparent
                    }
                    let (px, py) = (px0 + x as i32, py0 + y as i32);
                    if px < cx || px >= cx + cell_w as i32 || py < cy || py >= cy + cell_h as i32 {
                        continue; // keep cells visually separated
                    }
                    img[(py as usize) * w + px as usize] = shades[color as usize];
                }
            }
        }
    }
    write_gray_png(path, w as u32, h as u32, &img)
}

/// Render a level's whole world: 256x48 metatiles at 16x16 px = 4096x768.
pub fn write_world_render(
    path: &Path,
    layout: &[u8],
    metatiles: &[u8],
    bg_vram: &[u8],
    bgp: u8,
) -> Result<(), String> {
    let (w, h) = (4096usize, 768usize);
    let shades = bgp_shades(bgp);
    let mut img = vec![0u8; w * h];
    let mut blit = Blit {
        img: &mut img,
        w,
        shades,
        grey_cache: vec![None; 256],
        bg_vram,
    };
    for cy in 0..48 {
        for cx in 0..256 {
            let mid = layout[cy * 256 + cx] as usize;
            blit.metatile(cx, cy, metatile_quad(metatiles, mid));
        }
    }
    write_gray_png(path, w as u32, h as u32, &img)
}

/// Render one level's 128 metatile definitions as a 16x8 grid of 16x16 px
/// blocks (256x128 px), in defined order 0..128.
pub fn write_metatile_sheet(
    path: &Path,
    metatiles: &[u8],
    bg_vram: &[u8],
    bgp: u8,
) -> Result<(), String> {
    let (w, h) = (256usize, 128usize);
    let shades = bgp_shades(bgp);
    let mut img = vec![0u8; w * h];
    let mut blit = Blit {
        img: &mut img,
        w,
        shades,
        grey_cache: vec![None; 256],
        bg_vram,
    };
    for id in 0..128 {
        let (cx, cy) = (id % 16, id / 16);
        blit.metatile(cx, cy, metatile_quad(metatiles, id));
    }
    write_gray_png(path, w as u32, h as u32, &img)
}
