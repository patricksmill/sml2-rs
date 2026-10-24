use eframe::egui::ColorImage;

pub(super) fn save(path: &std::path::Path, image: &ColorImage) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(file, image.size[0] as u32, image.size[1] as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
    writer.write_image_data(&pixels).map_err(|e| e.to_string())
}

/// Compare the actual GPU screenshot with the CPU frame submitted on that render.
/// GPU presentation may round a grey level by one (sRGB conversion on Windows),
/// so each channel is allowed a single level of error.
pub(super) fn verify(image: &ColorImage, frame: &ColorImage, rect: [u32; 4]) -> Result<(), String> {
    let [x, y, w, h] = rect.map(|n| n as usize);
    let [fw, fh] = frame.size;
    if frame.pixels.len() != fw * fh
        || fw == 0
        || fh == 0
        || w == 0
        || h == 0
        || ((w as f64 * fh as f64 / fw.max(1) as f64).round() as usize).abs_diff(h) > 1
        || x + w > image.size[0]
        || y + h > image.size[1]
    {
        return Err("Game rectangle does not match the submitted game view".into());
    }
    for py in 0..h {
        for px in 0..w {
            // Nearest sampling chooses the texel under each destination pixel's center.
            let sx = ((px * 2 + 1) * fw / (w * 2)).min(fw - 1);
            let sy = ((py * 2 + 1) * fh / (h * 2)).min(fh - 1);
            let expected = frame.pixels[sy * fw + sx].to_array();
            let [er, eg, eb, _] = expected;
            let actual = image.pixels[(y + py) * image.size[0] + x + px].to_array();
            let close = actual[3] == 255
                && (i32::from(actual[0]) - i32::from(er)).abs() <= 1
                && (i32::from(actual[1]) - i32::from(eg)).abs() <= 1
                && (i32::from(actual[2]) - i32::from(eb)).abs() <= 1;
            if !close {
                return Err(format!(
                    "GPU pixel mismatch at {},{}: {:?}, expected {:?}",
                    x + px,
                    y + py,
                    actual,
                    expected
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Color32;

    fn image(level: u8) -> ColorImage {
        ColorImage::new([160, 144], vec![Color32::from_gray(level); 160 * 144])
    }

    #[test]
    fn verify_accepts_one_level_of_gpu_rounding() {
        assert!(verify(&image(84), &image(85), [0, 0, 160, 144]).is_ok());
        assert!(verify(&image(85), &image(85), [0, 0, 160, 144]).is_ok());
    }

    #[test]
    fn verify_rejects_a_real_mismatch() {
        assert!(verify(&image(82), &image(85), [0, 0, 160, 144]).is_err());
    }

    #[test]
    fn verify_rejects_incomplete_native_frames() {
        assert!(verify(
            &image(85),
            &ColorImage::new([1, 1], vec![Color32::BLACK]),
            [0, 0, 160, 144]
        )
        .is_err());
    }

    #[test]
    fn verify_accepts_an_integer_scaled_enlarged_view() {
        let frame = ColorImage::new([256, 144], vec![Color32::from_gray(85); 256 * 144]);
        let shown = ColorImage::new([512, 288], vec![Color32::from_gray(85); 512 * 288]);
        assert!(verify(&shown, &frame, [0, 0, 512, 288]).is_ok());
        assert!(verify(&shown, &frame, [0, 0, 512, 280]).is_err());
    }

    #[test]
    fn verify_accepts_fractional_nearest_scaling_and_still_detects_wrong_pixels() {
        let frame = ColorImage::new(
            [2, 2],
            vec![Color32::RED, Color32::GREEN, Color32::BLUE, Color32::WHITE],
        );
        let mut shown = ColorImage::new([5, 5], vec![Color32::BLACK; 25]);
        let source = [0, 0, 1, 1, 1];
        for y in 0..5 {
            for x in 0..5 {
                shown.pixels[y * 5 + x] = frame.pixels[source[y] * 2 + source[x]];
            }
        }
        assert!(verify(&shown, &frame, [0, 0, 5, 5]).is_ok());
        shown.pixels[12] = Color32::BLACK;
        assert!(verify(&shown, &frame, [0, 0, 5, 5]).is_err());
    }
}
