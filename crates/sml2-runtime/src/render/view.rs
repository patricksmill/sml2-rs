//! View-size arithmetic shared by the native and DX renderers.

pub const NATIVE: (usize, usize) = (160, 144);

/// How far an enlarged view's native rect sits from its top-left corner.
pub fn margin(view: (usize, usize)) -> (i32, i32) {
    ((view.0 as i32 - 160) / 2, (view.1 as i32 - 144) / 2)
}

/// Raise a zoomed-out level's bottom edge: three quarters of the extra
/// height reveal space above the native view, one quarter below it.
/// Fixed screens and overworld composition keep the centred `margin`.
pub fn level_bias(view: (usize, usize)) -> i32 {
    if enlarged(view) { (view.1 as i32 - NATIVE.1 as i32) / 4 } else { 0 }
}

/// Native gameplay rect inside a level view (including demos).
pub fn level_margin(view: (usize, usize)) -> (i32, i32) {
    let (x, y) = margin(view);
    (x, y + level_bias(view))
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
