//! View-size arithmetic shared by the native and DX renderers.

use sml2_core::camera::Room;

pub const NATIVE: (usize, usize) = (160, 144);

/// How far an enlarged view's native rect sits from its top-left corner.
pub fn margin(view: (usize, usize)) -> (i32, i32) {
    ((view.0 as i32 - 160) / 2, (view.1 as i32 - 144) / 2)
}

/// Raise a zoomed-out level's bottom edge: three quarters of the extra
/// height reveal space above the native view, one quarter below it.
/// Fixed screens and overworld composition keep the centred `margin`.
pub fn level_bias(view: (usize, usize)) -> i32 {
    if enlarged(view) {
        (view.1 as i32 - NATIVE.1 as i32) / 4
    } else {
        0
    }
}

/// Native gameplay rect inside a level view (including demos).
pub fn level_margin(view: (usize, usize)) -> (i32, i32) {
    let (x, y) = margin(view);
    (x, y + level_bias(view))
}

pub(crate) fn raw_level_origin(view: (usize, usize), cam: (u16, u16), pan: i32) -> (i32, i32) {
    (
        cam.0 as i32 - view.0 as i32 / 2,
        cam.1 as i32 - view.1 as i32 / 2 - level_bias(view) - pan,
    )
}

/// The span each axis of an enlarged view's origin may take: inside both
/// `room` and the layout. A view longer than the room centres on it.
fn origin_range(view: (usize, usize), room: Room) -> ((i32, i32), (i32, i32)) {
    let axis = |lo: i32, hi: i32, world: i32, len: usize| {
        let len = len as i32;
        let (lo, hi) = (lo.max(0), hi.min(world));
        if hi - lo >= len {
            (lo, hi - len)
        } else {
            let at = ((lo + hi - len) / 2).clamp(0, (world - len).max(0));
            (at, at)
        }
    };
    (
        axis(room.left, room.right, room.world.0, view.0),
        axis(room.top, room.bottom, room.world.1, view.1),
    )
}

/// Enlarged gameplay views stop at the camera's room and the layout edges.
/// Native views retain the hardware window coordinates, including their
/// wrapping behavior.
pub fn level_origin(view: (usize, usize), cam: (u16, u16), pan: i32, room: Room) -> (i32, i32) {
    let origin = raw_level_origin(view, cam, pan);
    if !enlarged(view) {
        return origin;
    }
    let (x, y) = origin_range(view, room);
    (origin.0.clamp(x.0, x.1), origin.1.clamp(y.0, y.1))
}

/// One presentation camera shared by terrain, sprites and interpolation.
/// Shifts are output pixels; the status bar stays fixed in screen space.
pub(crate) struct LevelViewport {
    pub origin: (i32, i32),
    pub bg_shift: (i32, i32),
    pub sprite_shift: (i32, i32),
}

impl LevelViewport {
    pub fn new(
        view: (usize, usize),
        cam: (u16, u16),
        pan: i32,
        room: Room,
        scale: usize,
        bg_shift: (i32, i32),
    ) -> Self {
        let raw = raw_level_origin(view, cam, pan);
        let origin = level_origin(view, cam, pan, room);
        let range = origin_range(view, room);
        let s = scale as i32;
        let clamp_shift = |shift: i32, raw: i32, at: i32, (lo, hi): (i32, i32)| {
            if enlarged(view) {
                at * s - (raw * s - shift).clamp(lo * s, hi * s)
            } else {
                shift
            }
        };
        let clamped = (
            clamp_shift(bg_shift.0, raw.0, origin.0, range.0),
            clamp_shift(bg_shift.1, raw.1, origin.1, range.1),
        );
        Self {
            origin,
            bg_shift: clamped,
            sprite_shift: (
                (raw.0 - origin.0) * s + clamped.0 - bg_shift.0,
                (raw.1 - origin.1) * s + clamped.1 - bg_shift.1,
            ),
        }
    }
}

/// The camera to draw an 8-bit-OAM sprite with, and the offset to add to its
/// cells. A native view keeps the wrapped camera-relative path; a resized one
/// draws the sprite with the camera on its own anchor and adds the true offset.
pub fn sprite_anchor(
    view: (usize, usize),
    cam: (u16, u16),
    at: (u16, u16),
) -> ((u16, u16), (i32, i32)) {
    if view == NATIVE {
        return (cam, (0, 0));
    }
    let (mx, my) = level_margin(view);
    (
        at,
        (
            at.0 as i32 - cam.0 as i32 + mx,
            at.1 as i32 - cam.1 as i32 + my,
        ),
    )
}

/// A view at least as large as native in both axes, and larger in one.
pub fn enlarged(view: (usize, usize)) -> bool {
    view.0 >= NATIVE.0 && view.1 >= NATIVE.1 && view != NATIVE
}

/// The tilemap window a view samples through: the engine's 256 px, or the
/// view itself (rounded to whole metatiles) when it is wider.
pub fn window_span(len: usize) -> i32 {
    ((len as i32 + 15) & !15).max(256)
}

/// Copy `rect = (x, y, w, h)` of a `src_w`-wide image to `at` in `dst`,
/// clipped to `dst_size`.
pub fn copy_rect<P: Copy>(
    dst: &mut [P],
    dst_size: (usize, usize),
    src: &[P],
    src_w: usize,
    rect: (usize, usize, usize, usize),
    at: (i32, i32),
) {
    let (x0, y0, w, h) = rect;
    for dy in 0..h {
        let ty = at.1 + dy as i32;
        if ty < 0 || ty >= dst_size.1 as i32 {
            continue;
        }
        for dx in 0..w {
            let tx = at.0 + dx as i32;
            if tx < 0 || tx >= dst_size.0 as i32 {
                continue;
            }
            dst[ty as usize * dst_size.0 + tx as usize] = src[(y0 + dy) * src_w + x0 + dx];
        }
    }
}

/// `src` centred in a `view`-sized frame filled with `fill`.
pub fn letterbox<P: Copy>(
    src: &[P],
    src_size: (usize, usize),
    view: (usize, usize),
    fill: P,
) -> Vec<P> {
    let mut out = vec![fill; view.0 * view.1];
    let at = (
        (view.0 as i32 - src_size.0 as i32) / 2,
        (view.1 as i32 - src_size.1 as i32) / 2,
    );
    copy_rect(
        &mut out,
        view,
        src,
        src_size.0,
        (0, 0, src_size.0, src_size.1),
        at,
    );
    out
}

/// A screen with native-only art: rendered at 160×144 and letterboxed into
/// an enlarged view; any other view renders as before.
pub fn fixed_screen<P: Copy>(
    view: (usize, usize),
    fill: P,
    render: impl FnOnce((usize, usize)) -> Vec<P>,
) -> Vec<P> {
    if enlarged(view) {
        letterbox(&render(NATIVE), NATIVE, view, fill)
    } else {
        render(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_camera_parks_at_edges_and_follows_between_them() {
        let view = (384, 216);
        for (cam, expected) in [
            ((80, 72), (0, 0)),
            ((192, 448), (0, 322)),
            ((1024, 448), (832, 322)),
            ((4016, 704), (3712, 552)),
        ] {
            assert_eq!(level_origin(view, cam, 0, Room::WORLD), expected);
        }
        assert_eq!(level_origin(NATIVE, (80, 704), 0, Room::WORLD), (0, 632));
    }

    #[test]
    fn a_room_holds_the_view_inside_its_locks() {
        let view = (384, 216);
        let room = Room {
            left: 0,
            top: 0x100,
            right: 0x800,
            bottom: 0x208,
            ..Room::WORLD
        };
        // At the $B0 lock the native view ends on the room's edge; the wide
        // one stops there too instead of showing past it.
        assert_eq!(level_origin(view, (0x7B0, 0x1C0), 0, room), (0x800 - 384, 0x208 - 216));
        assert_eq!(level_origin(NATIVE, (0x7B0, 0x1C0), 0, room), (0x7B0 - 80, 0x1C0 - 72));
        // Mid-room the view follows the camera.
        let raw = raw_level_origin(view, (0x400, 0x180), 0);
        assert_eq!(level_origin(view, (0x400, 0x180), 0, room), raw);
        // A room narrower than the view centres it.
        let cell = Room {
            left: 0x800,
            right: 0x900,
            ..room
        };
        assert_eq!(level_origin(view, (0x850, 0x1C0), 0, cell).0, 0x880 - 192);
    }

    #[test]
    fn a_wider_world_lets_the_view_pass_the_rom_layout_edge() {
        let view = (384, 216);
        let wide = sml2_core::world::World {
            screens_wide: 40,
            screens_high: 3,
            position_mask: 0xFFFF,
            wram_echo: false,
        }
        .room();
        let cam = (0x1800, 0x1C0);
        let raw = raw_level_origin(view, cam, 0);
        assert_eq!(level_origin(view, cam, 0, wide), raw);
        assert_eq!(level_origin(view, cam, 0, Room::WORLD).0, 0x1000 - 384);
        assert_eq!(level_origin(view, (0x27F0, 0x1C0), 0, wide).0, 0x2800 - 384);
    }

    #[test]
    fn interpolation_stops_at_the_edges_and_translates_sprites_with_terrain() {
        let view = (384, 216);
        for (cam, motion) in [
            ((80, 72), (3, 2)),
            ((80, 72), (-3, -2)),
            ((4016, 704), (3, 2)),
            ((4016, 704), (-3, -2)),
            ((192, 126), (3, 2)),
            ((3904, 678), (-3, -2)),
        ] {
            let v = LevelViewport::new(view, cam, 0, Room::WORLD, 2, motion);
            assert_eq!(v.bg_shift, (0, 0));
            let raw = raw_level_origin(view, cam, 0);
            // A static world point has the same final position through both
            // terrain coordinates and the OAM camera plus its interpolation.
            assert_eq!(
                (
                    -raw.0 * 2 + motion.0 + v.sprite_shift.0,
                    -raw.1 * 2 + motion.1 + v.sprite_shift.1
                ),
                (-v.origin.0 * 2, -v.origin.1 * 2),
            );
        }
        let interior = LevelViewport::new(view, (1024, 448), 3, Room::WORLD, 2, (3, -2));
        assert_eq!(interior.bg_shift, (3, -2));
        assert_eq!(interior.sprite_shift, (0, 0));
    }

    #[test]
    fn spans_and_margins_match_the_view_table() {
        assert_eq!(margin((384, 216)), (112, 36));
        assert_eq!(margin(NATIVE), (0, 0));
        assert_eq!(window_span(160), 256);
        assert_eq!(window_span(256), 256);
        assert_eq!(window_span(320), 320);
        assert_eq!(window_span(384), 384);
        assert!(enlarged((256, 144)) && !enlarged(NATIVE) && !enlarged((16, 16)));
    }

    #[test]
    fn level_zoom_reveals_three_quarters_above_and_one_below() {
        for (view, bias, top, bottom) in [
            (NATIVE, 0, 0, 0),
            ((256, 144), 0, 0, 0),
            ((200, 180), 9, 27, 9),
            ((384, 216), 18, 54, 18),
            ((512, 288), 36, 108, 36),
        ] {
            assert_eq!(level_bias(view), bias);
            assert_eq!(level_margin(view).1, top);
            assert_eq!(view.1 as i32 - top - 144, bottom);
            assert_eq!(margin(view).1, (view.1 as i32 - 144) / 2);
        }
        assert_eq!(level_bias((80, 72)), 0);
    }

    #[test]
    fn sprite_anchor_keeps_native_and_offsets_resized_views() {
        assert_eq!(
            sprite_anchor(NATIVE, (100, 50), (130, 60)),
            ((100, 50), (0, 0))
        );
        assert_eq!(
            sprite_anchor((384, 216), (100, 50), (130, 60)),
            ((130, 60), (142, 64))
        );
    }

    #[test]
    fn letterbox_centres_with_bars() {
        let out = letterbox(&[1u8; 4], (2, 2), (4, 4), 0);
        assert_eq!(out, vec![0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0]);
    }
}
