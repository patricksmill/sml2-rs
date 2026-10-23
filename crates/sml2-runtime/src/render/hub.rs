//! Expanded hub composition, shared by grayscale, hardware palettes and DX.
use crate::session::Session;

/// Source native arrow positions and their destinations on the enlarged view.
fn pan_arrow_positions((w, h): (usize, usize)) -> [((i32, i32), (i32, i32)); 4] {
    let (dx, dy) = ((w as i32 - 160) / 2, (h as i32 - 144) / 2);
    [
        ((72, 8), (72 + dx, 8)),
        ((72, 124), (72 + dx, h as i32 - 20)),
        ((8, 68), (8, 68 + dy)),
        ((144, 68), (w as i32 - 16, 68 + dy)),
    ]
}

impl Session {
    pub(super) fn stitched_hub<P: Copy>(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        view: (usize, usize),
        fill: P,
        mut render: impl FnMut(&sml2_core::map_sim::MapSim, (usize, usize)) -> Result<Vec<P>, String>,
    ) -> Result<Vec<P>, String> {
        use super::view::{copy_rect, NATIVE};
        const HUB: i32 = 512;
        // Rows above `$3F203`'s status-bar strip; each native frame's own strip is dropped.
        const ROWS: usize = 136;
        // OAM Y $F0 (and the +8 rows and walk shift, which wrap to 0) all land
        // above or below the frame, so the cursor draws nothing.
        const HIDDEN_CURSOR_Y: u8 = 0xF0;
        let (vw, vh) = view;
        let ox = (i32::from(map_sim.cam_x) + 80 - vw as i32 / 2).clamp(0, HUB - vw as i32);
        let oy = (i32::from(map_sim.cam_y) + 72 - vh as i32 / 2).clamp(0, HUB - vh as i32);
        let mut out = vec![fill; vw * vh];
        for ty in (0..vh).step_by(ROWS) {
            for tx in (0..vw).step_by(NATIVE.0) {
                let mut tile = map_sim.clone();
                tile.cam_x = (ox + tx as i32) as u16;
                tile.cam_y = (oy + ty as i32) as u16;
                let cx =
                    i32::from(map_sim.cursor_x) + i32::from(map_sim.cam_x) - i32::from(tile.cam_x);
                let cy =
                    i32::from(map_sim.cursor_y) + i32::from(map_sim.cam_y) - i32::from(tile.cam_y);
                // From -16 the 8-bit wrap still lands each cell where it truly is.
                if (-16..=176).contains(&cx) && (-16..=168).contains(&cy) {
                    tile.cursor_x = cx as u8;
                    tile.cursor_y = cy as u8;
                } else {
                    tile.cursor_y = HIDDEN_CURSOR_Y;
                }
                // Pan arrows belong to the view edges, not every stitched panel.
                // Render the world without a cursor, then add the controls once.
                if map_sim.scroll_only_latch != 0 {
                    tile.scroll_only_latch = 0;
                    tile.cursor_y = HIDDEN_CURSOR_Y;
                }
                let pixels = render(&tile, NATIVE)?;
                copy_rect(
                    &mut out,
                    view,
                    &pixels,
                    NATIVE.0,
                    (0, 0, NATIVE.0, ROWS),
                    (tx as i32, ty as i32),
                );
            }
        }
        if map_sim.scroll_only_latch != 0 {
            // Re-render each arrow's 8x8 patch against the background it covers.
            // Keeping native panels preserves OAM priority, palettes and blink
            // timing without wrapping screen coordinates beyond 255 pixels.
            for (source, target) in pan_arrow_positions(view) {
                let mut panel = map_sim.clone();
                panel.cam_x = (ox + target.0 - source.0) as u16;
                panel.cam_y = (oy + target.1 - source.1) as u16;
                let pixels = render(&panel, NATIVE)?;
                copy_rect(
                    &mut out,
                    view,
                    &pixels,
                    NATIVE.0,
                    (source.0 as usize, source.1 as usize, 8, 8),
                    target,
                );
            }
            return Ok(out);
        }
        let mut bar = map_sim.clone();
        bar.cursor_y = HIDDEN_CURSOR_Y;
        let pixels = render(&bar, NATIVE)?;
        let (x0, y0, x1, y1) = sml2_core::map::status_bar_strip_rect(NATIVE.0, NATIVE.1);
        copy_rect(
            &mut out,
            view,
            &pixels,
            NATIVE.0,
            (x0, y0, x1 - x0, y1 - y0),
            (
                (vw as i32 - 160) / 2 + x0 as i32,
                vh as i32 - (y1 - y0) as i32,
            ),
        );
        Ok(out)
    }
}

#[cfg(test)]
#[path = "hub_tests.rs"]
mod tests;
