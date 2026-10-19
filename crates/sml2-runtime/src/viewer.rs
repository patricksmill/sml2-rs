//! Window host for inspecting a level without simulation.

use crate::present::letterbox_gray;
use crate::render::{apply_tile_anim, render_view, SpriteCtx};
use sml2_core::level::{LevelLayout, LevelManifest, LAYOUT_HEIGHT, LAYOUT_WIDTH};

/// Everything the window viewer needs per frame.
pub(crate) struct Viewer<'a> {
    pub(crate) manifest: &'a LevelManifest,
    pub(crate) layout: &'a LevelLayout,
    pub(crate) bg_base: &'a [u8],
    pub(crate) anim_rows: &'a [u8],
    pub(crate) hud_row: &'a [u8],
    pub(crate) hud_y: u8,
    pub(crate) scale: usize,
    pub(crate) spr: Option<&'a SpriteCtx>,
    /// Viewport in logical pixels (`--size`).
    pub(crate) view: (usize, usize),
}

/// Interactive viewer: arrow keys scroll the camera, Q/ESC quits; the window
/// is resizable (integer scale + letterbox). Animated tiles advance on the
/// engine's own cadence (`speed * 2` frames per phase).
pub(crate) fn run_window(
    v: &Viewer,
    mut phase: u8,
    start: (u16, u16),
) -> Result<(), Box<dyn std::error::Error>> {
    let Viewer {
        manifest,
        layout,
        bg_base,
        anim_rows,
        hud_row,
        hud_y,
        scale,
        spr,
        view,
    } = *v;
    let (vw, vh) = view;
    use minifb::{Key, Window, WindowOptions};
    let mut window = Window::new(
        &format!(
            "sml2 level {} {}x{} (arrows scroll, Q quits)",
            manifest.level_id, vw, vh
        ),
        vw * scale,
        vh * scale,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| format!("cannot open a window ({e}) — no display found; use --out instead"))?;
    window.set_target_fps(60);
    let (mut cx, mut cy) = (start.0 as i32, start.1 as i32);
    let max_x = (LAYOUT_WIDTH * 16) as i32;
    let max_y = (LAYOUT_HEIGHT * 16) as i32;
    let mut buf = vec![0u32; vw * scale * vh * scale];
    let mut anim_timer: u32 = 0;
    while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
        let step = if window.is_key_down(Key::LeftShift) {
            16
        } else {
            2
        };
        if window.is_key_down(Key::Right) {
            cx = (cx + step).min(max_x);
        }
        if window.is_key_down(Key::Left) {
            cx = (cx - step).max(0);
        }
        if window.is_key_down(Key::Down) {
            cy = (cy + step).min(max_y);
        }
        if window.is_key_down(Key::Up) {
            cy = (cy - step).max(0);
        }
        anim_timer += 1;
        if manifest.tile_anim.enabled && anim_timer >= manifest.tile_anim.speed as u32 * 2 {
            anim_timer = 0;
            phase = (phase + 1) & 3;
        }
        let mut bg = bg_base.to_vec();
        apply_tile_anim(&mut bg, &manifest.tile_anim, anim_rows, phase);
        let frame = render_view(
            manifest,
            layout,
            &[],
            &bg,
            cx as u16,
            cy as u16,
            vw,
            vh,
            Some((hud_row, hud_y)),
            spr,
            None,
        )?;
        // Integer-scale + letterbox into the current window size.
        let (win_w, win_h) = window.get_size();
        if win_w == 0 || win_h == 0 {
            continue;
        }
        letterbox_gray(&frame, (vw, vh), (win_w, win_h), &mut buf);
        window
            .update_with_buffer(&buf, win_w, win_h)
            .map_err(|e| e.to_string())?;
    }
    println!("camera ended at {}x{}", cx, cy);
    Ok(())
}
