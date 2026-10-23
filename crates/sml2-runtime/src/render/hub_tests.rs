use super::*;
use crate::content::SessionAssets;
use crate::session::SessionState;
use sml2_core::map::PAN_ARROWS;
use sml2_core::map_sim::MapSim;

const ARROW: u32 = 0xff0000;
const CURSOR: u32 = 0xfe0000;

// Native panels encode world positions, with visible controls and a cursor.
// This makes padding, repeated panels, stray cursors and misplaced arrows distinct.
fn panel(map: &MapSim, size: (usize, usize), arrows_on: bool) -> Result<Vec<u32>, String> {
    let (w, h) = size;
    let mut out: Vec<_> = (0..h)
        .flat_map(|y| {
            (0..w).map(move |x| {
                (u32::from(map.cam_y) + y as u32) * 512 + u32::from(map.cam_x) + x as u32 + 1
            })
        })
        .collect();
    if map.scroll_only_latch != 0 && arrows_on {
        for (y, x, _, _) in PAN_ARROWS {
            for dy in 0..8 {
                for dx in 0..8 {
                    out[(usize::from(y) - 16 + dy) * w + usize::from(x) - 8 + dx] = ARROW;
                }
            }
        }
    } else if map.scroll_only_latch == 0 && usize::from(map.cursor_y) < h {
        out[usize::from(map.cursor_y) * w + usize::from(map.cursor_x)] = CURSOR;
    }
    Ok(out)
}

#[test]
fn panning_fills_the_available_hub_and_draws_each_blinking_arrow_once() {
    let session = Session::new(SessionAssets::load().unwrap());
    let SessionState::Overworld(map) = &session.state else {
        panic!("boots to hub")
    };
    let mut map = map.clone();
    map.screen_id = 0;
    map.scroll_only_latch = 0xff;
    map.cursor_x = 60;
    map.cursor_y = 60;
    for view in [(256, 144), (320, 180), (384, 216), (512, 288), (600, 144)] {
        let size = (view.0.min(512), view.1.min(512));
        let at = ((view.0 - size.0) / 2, (view.1 - size.1) / 2);
        for camera in [0, 192, 400] {
            map.cam_x = camera;
            map.cam_y = camera;
            let ox = (i32::from(camera) + 80 - size.0 as i32 / 2).clamp(0, 512 - size.0 as i32);
            let oy = (i32::from(camera) + 72 - size.1 as i32 / 2).clamp(0, 512 - size.1 as i32);
            for arrows_on in [true, false] {
                let frame = session
                    .overworld_view(&map, view, 0, |m, s| panel(m, s, arrows_on))
                    .unwrap();
                // Independent expected screen coordinates retain native edge insets.
                let arrows = [
                    (size.0 / 2 - 8, 8),
                    (size.0 / 2 - 8, size.1 - 20),
                    (8, size.1 / 2 - 4),
                    (size.0 - 16, size.1 / 2 - 4),
                ];
                let mut count = 0;
                for y in 0..view.1 {
                    for x in 0..view.0 {
                        let expected =
                            if x < at.0 || x >= at.0 + size.0 || y < at.1 || y >= at.1 + size.1 {
                                0
                            } else {
                                let (x, y) = (x - at.0, y - at.1);
                                if arrows_on
                                    && arrows.iter().any(|&(ax, ay)| {
                                        x >= ax && x < ax + 8 && y >= ay && y < ay + 8
                                    })
                                {
                                    ARROW
                                } else {
                                    (oy as u32 + y as u32) * 512 + ox as u32 + x as u32 + 1
                                }
                            };
                        let pixel = frame[y * view.0 + x];
                        count += usize::from(pixel == ARROW);
                        assert_eq!(
                            pixel, expected,
                            "view={view:?} cam={camera} at=({x},{y}) blink={arrows_on}"
                        );
                    }
                }
                assert_eq!(count, if arrows_on { 256 } else { 0 });
                assert_eq!(
                    (
                        map.cam_x,
                        map.cam_y,
                        map.cursor_x,
                        map.cursor_y,
                        map.scroll_only_latch
                    ),
                    (camera, camera, 60, 60, 0xff),
                    "rendering preserves live state"
                );
            }
        }
    }
}
