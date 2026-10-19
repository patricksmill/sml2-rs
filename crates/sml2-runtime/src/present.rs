//! Conversion of logical game pixels into a window buffer.
//!
//! This module does not advance simulation or own a window. The existing
//! integer fit, centering, and clipping behavior is shared by every window path.

pub fn letterbox_gray(
    frame: &[u8],
    view: (usize, usize),
    window: (usize, usize),
    buffer: &mut Vec<u32>,
) {
    let (vw, vh) = view;
    let (win_w, win_h) = window;
    buffer.resize(win_w * win_h, 0);
    buffer.fill(0);
    if win_w == 0 || win_h == 0 {
        return;
    }
    let scale = (win_w / vw).min(win_h / vh).max(1);
    let ox = win_w.saturating_sub(vw * scale) / 2;
    let oy = win_h.saturating_sub(vh * scale) / 2;
    for y in 0..vh {
        let py = oy + y * scale;
        if py + scale > win_h {
            break;
        }
        for x in 0..vw {
            let px = ox + x * scale;
            if px + scale > win_w {
                break;
            }
            let gray = frame[y * vw + x] as u32;
            let rgb = gray | (gray << 8) | (gray << 16);
            for dy in 0..scale {
                let row = (py + dy) * win_w + px;
                buffer[row..row + scale].fill(rgb);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_integer_pixels_and_clears_reused_letterbox() {
        let mut pixels = vec![0xFFFFFF; 35];
        letterbox_gray(&[0x12, 0xAB], (2, 1), (7, 5), &mut pixels);
        assert_eq!(pixels.len(), 35);
        assert_eq!(&pixels[..7], &[0; 7]);
        for row in pixels[7..28].chunks_exact(7) {
            assert_eq!(
                row,
                &[0x121212, 0x121212, 0x121212, 0xABABAB, 0xABABAB, 0xABABAB, 0]
            );
        }
        assert_eq!(&pixels[28..], &[0; 7]);
    }

    #[test]
    fn preserves_clipping_below_native_size_and_handles_minimize() {
        let mut pixels = Vec::new();
        letterbox_gray(&[1, 2, 3, 4, 5, 6], (3, 2), (2, 1), &mut pixels);
        assert_eq!(pixels, [0x010101, 0x020202]);
        letterbox_gray(&[], (3, 2), (0, 0), &mut pixels);
        assert!(pixels.is_empty());
    }
}
