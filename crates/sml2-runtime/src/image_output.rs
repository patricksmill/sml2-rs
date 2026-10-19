//! PNG export for CLI renders and replay filmstrips.

use std::fs;
use std::path::Path;

/// Write an 8-bit greyscale PNG, upscaled by an integer factor (nearest).
pub(crate) fn write_png_scaled(
    path: &Path,
    w: usize,
    h: usize,
    scale: usize,
    gray: &[u8],
) -> Result<(), String> {
    let (w2, h2) = (w * scale, h * scale);
    let mut data = vec![0u8; w2 * h2];
    for y in 0..h {
        for x in 0..w {
            let v = gray[y * w + x];
            for dy in 0..scale {
                let row = (y * scale + dy) * w2 + x * scale;
                data[row..row + scale].fill(v);
            }
        }
    }
    let file = fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w2 as u32, h2 as u32);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&data).map_err(|e| e.to_string())?;
    Ok(())
}
