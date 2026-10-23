//! Layered frames: a background with a border, an unshifted window layer and
//! pre-sampled OBJ cells, composed at an integer scale with per-layer shifts
//! in output pixels. At scale 1 with no shifts this is the plain GB picture.

/// An 8×8 OBJ cell at game position `(x, y)`, shifted by `shift` output
/// pixels; `None` pixels are transparent. Pushed in OAM priority order.
#[derive(Clone)]
pub(crate) struct Cell<O> {
    pub x: i32,
    pub y: i32,
    pub shift: (i32, i32),
    pub pixels: [Option<O>; 64],
}

pub(crate) struct Layers<B, O> {
    pub view: (usize, usize),
    pub border: usize,
    /// `(vw + 2·border) × (vh + 2·border)`, origin at `(-border, -border)`.
    pub bg: Vec<B>,
    /// The window (status bar), never shifted; `None` shows the background.
    pub window: Vec<Option<B>>,
    pub cells: Vec<Cell<O>>,
}

impl<B: Copy, O: Copy> Layers<B, O> {
    pub fn new(view: (usize, usize), border: usize, fill: B) -> Self {
        Self {
            view,
            border,
            bg: vec![fill; (view.0 + 2 * border) * (view.1 + 2 * border)],
            window: vec![None; view.0 * view.1],
            cells: Vec::new(),
        }
    }

    /// The background pixel at view coordinates `-border..view + border`.
    pub fn bg_mut(&mut self, x: i32, y: i32) -> &mut B {
        let b = self.border as i32;
        let w = self.view.0 + 2 * self.border;
        &mut self.bg[(y + b) as usize * w + (x + b) as usize]
    }

    /// Upsample by `scale`: the background shifted by `bg_shift`, the window
    /// over it, and the first cell pixel per output pixel that `hidden` does
    /// not reject (a rejected pixel leaves the slot to later cells).
    pub fn compose(
        &self,
        scale: usize,
        bg_shift: (i32, i32),
        hidden: impl Fn(&O, &B) -> bool,
    ) -> (Vec<B>, Vec<Option<O>>) {
        let (vw, vh) = self.view;
        let (w, h) = (vw * scale, vh * scale);
        let (s, b) = (scale as i32, self.border as i32);
        let (bw, bh) = ((vw + 2 * self.border) as i32, (vh + 2 * self.border) as i32);
        let mut bg = Vec::with_capacity(w * h);
        for y in 0..h as i32 {
            let gy = ((y - bg_shift.1).div_euclid(s) + b).clamp(0, bh - 1);
            for x in 0..w as i32 {
                let window = self.window[(y / s) as usize * vw + (x / s) as usize];
                bg.push(window.unwrap_or_else(|| {
                    let gx = ((x - bg_shift.0).div_euclid(s) + b).clamp(0, bw - 1);
                    self.bg[gy as usize * bw as usize + gx as usize]
                }));
            }
        }
        let mut obj = vec![None; w * h];
        for cell in &self.cells {
            for (i, p) in cell.pixels.iter().enumerate() {
                let Some(p) = p else { continue };
                let x0 = (cell.x + (i % 8) as i32) * s + cell.shift.0;
                let y0 = (cell.y + (i / 8) as i32) * s + cell.shift.1;
                for y in y0.max(0)..(y0 + s).min(h as i32) {
                    for x in x0.max(0)..(x0 + s).min(w as i32) {
                        let idx = y as usize * w + x as usize;
                        if obj[idx].is_none() && !hidden(p, &bg[idx]) {
                            obj[idx] = Some(*p);
                        }
                    }
                }
            }
        }
        (bg, obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(x: i32, y: i32, shift: (i32, i32), v: u8) -> Cell<u8> {
        Cell {
            x,
            y,
            shift,
            pixels: [Some(v); 64],
        }
    }

    #[test]
    fn a_half_pixel_background_shift_at_scale_two_moves_one_output_pixel() {
        let mut layers: Layers<u8, u8> = Layers::new((2, 1), 1, 0);
        layers.bg = vec![9, 9, 9, 9, 9, 1, 2, 9, 9, 9, 9, 9];
        let (bg, _) = layers.compose(2, (1, 0), |_, _| false);
        assert_eq!(&bg[..4], &[9, 1, 1, 2]);
        let (bg, _) = layers.compose(2, (0, 0), |_, _| false);
        assert_eq!(&bg[..4], &[1, 1, 2, 2]);
    }

    #[test]
    fn the_window_ignores_the_background_shift() {
        let mut layers: Layers<u8, u8> = Layers::new((2, 1), 1, 0);
        layers.window[1] = Some(7);
        let (bg, _) = layers.compose(1, (1, 0), |_, _| false);
        assert_eq!(bg[1], 7);
    }

    #[test]
    fn earlier_cells_win_and_hidden_pixels_leave_the_slot_free() {
        let mut layers: Layers<u8, u8> = Layers::new((8, 8), 0, 1);
        layers.cells.push(solid(0, 0, (0, 0), 5));
        layers.cells.push(solid(0, 0, (0, 0), 6));
        let (_, obj) = layers.compose(1, (0, 0), |_, _| false);
        assert_eq!(obj[0], Some(5));
        let (_, obj) = layers.compose(1, (0, 0), |&o, _| o == 5);
        assert_eq!(obj[0], Some(6));
    }

    #[test]
    fn cells_upsample_and_shift_in_output_pixels() {
        let mut layers: Layers<u8, u8> = Layers::new((8, 8), 0, 0);
        layers.cells.push(solid(-8, 0, (1, 0), 3));
        let (_, obj) = layers.compose(2, (0, 0), |_, _| false);
        assert_eq!(obj[0], Some(3), "last column, shifted in by one output px");
        assert_eq!(obj[1], None);
    }
}
