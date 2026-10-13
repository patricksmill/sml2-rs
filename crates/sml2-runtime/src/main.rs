//! Super Mario Land 2 runtime — frame renderer (BG + tile anim + HUD + OBJ).
//!
//! Consumes only `extracted/` (no ROM parsing). Reconstructs the game's view
//! for a camera position. The native 160x144 GB screen is the default
//! viewport, but any `--size WxH` viewport works: the view is centred on the
//! camera (`screen_top_left = camera - (W/2, H/2)`, clamped to the world) and
//! the native rect stays pixel-exact inside it — the HUD strip remains at its
//! native position, so the centre 160x144 slice of a larger frame equals the
//! native frame. That property is the display regression test.
//!
//! Usage (run from the repo root or from `sml2-rs/`):
//!   sml2-runtime --level N [--camera X Y] [--size WxH] [--out frame.png]
//!       [--scale 4] [--anim-phase 0..3] [--hud-digits DDDDDDDDDD]
//!       [--sprites sprites.json]
//!   sml2-runtime --level N [--camera X Y] [--size WxH] --window
//!     (resizable window: integer scale + letterbox; arrows scroll,
//!      shift = fast, Q/ESC quits)
//!
//! Camera defaults to the level header's `camera_start_x/y`. Tiles resolve
//! through `sml2_core::video`, which keeps palette indices as data — the
//! seam a texture pack plugs into later (see sml2-rs/DESIGN.md).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest, LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::object::ObjectTables;
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_core::video;

/// Native GB screen; the HUD strip and the parity checks are anchored to it.
const SCREEN_W: usize = 160;
const SCREEN_H: usize = 144;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut level, mut camera) = (None, None);
    let mut out = PathBuf::from("frame.png");
    let mut scale: usize = 4;
    let mut window_mode = false;
    let mut anim_phase: u8 = 0;
    let mut hud_digits: Option<String> = None;
    let mut sprites_file: Option<PathBuf> = None;
    let mut mario: Option<(u16, u16, u8)> = None;
    let mut replay_file: Option<PathBuf> = None;
    let mut filmstrip: Option<PathBuf> = None;
    let mut play_mode = false;
    let mut view = (SCREEN_W, SCREEN_H);
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--level" => {
                level = Some(args.next().ok_or("--level needs a value")?.parse::<u8>()?);
            }
            "--camera" => {
                let x = args.next().ok_or("--camera needs X Y")?.parse::<u16>()?;
                let y = args.next().ok_or("--camera needs X Y")?.parse::<u16>()?;
                camera = Some((x, y));
            }
            "--out" => out = PathBuf::from(args.next().ok_or("--out needs a path")?),
            "--scale" => scale = args.next().ok_or("--scale needs a value")?.parse()?,
            "--anim-phase" => {
                anim_phase = args
                    .next()
                    .ok_or("--anim-phase needs 0..3")?
                    .parse::<u8>()?;
            }
            "--hud-digits" => {
                hud_digits = Some(args.next().ok_or("--hud-digits needs 10 digits")?);
            }
            "--sprites" => {
                sprites_file = Some(PathBuf::from(args.next().ok_or("--sprites needs a path")?));
            }
            "--mario" => {
                let x = args
                    .next()
                    .ok_or("--mario needs X Y FRAME")?
                    .parse::<u16>()?;
                let y = args
                    .next()
                    .ok_or("--mario needs X Y FRAME")?
                    .parse::<u16>()?;
                let f = args
                    .next()
                    .ok_or("--mario needs X Y FRAME")?
                    .parse::<u8>()?;
                mario = Some((x, y, f));
            }
            "--size" => {
                let s = args.next().ok_or("--size needs WxH")?;
                let (w, h) = s
                    .split_once('x')
                    .or_else(|| s.split_once('X'))
                    .ok_or("--size needs WxH (e.g. 320x144)")?;
                view = (w.parse::<usize>()?, h.parse::<usize>()?);
            }
            "--replay" => {
                replay_file = Some(PathBuf::from(args.next().ok_or("--replay needs a path")?));
            }
            "--filmstrip" => {
                filmstrip = Some(PathBuf::from(args.next().ok_or("--filmstrip needs a dir")?));
            }
            "--play" => play_mode = true,
            "--window" => window_mode = true,
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let level = level.ok_or("--level is required")?;
    if scale == 0 || scale > 16 {
        return Err("--scale must be in 1..=16".into());
    }
    let (world_px_w, world_px_h) = (LAYOUT_WIDTH * 16, LAYOUT_HEIGHT * 16);
    if view.0 < 16 || view.1 < 16 || view.0 > world_px_w || view.1 > world_px_h {
        return Err(format!("--size must be 16..={world_px_w} x 16..={world_px_h}").into());
    }
    if view.0 * scale > 8192 || view.1 * scale > 8192 {
        return Err("--size * --scale must stay within 8192 px per side".into());
    }
    let nn = level + 1;
    let levels_dir = extracted_dir().join("levels");
    let manifest: LevelManifest = serde_json::from_str(&fs::read_to_string(
        levels_dir.join(format!("level{nn:02}.manifest.json")),
    )?)?;
    let layout = LevelLayout::from_bytes(&fs::read(levels_dir.join(&manifest.layout.file))?)?;
    let bg = fs::read(levels_dir.join(format!("level{nn:02}.bgblock.bin")))?;
    if bg.len() != video::BG_VRAM_LEN {
        return Err(format!("bgblock.bin: expected {} bytes", video::BG_VRAM_LEN).into());
    }
    if anim_phase > 3 {
        return Err("--anim-phase must be 0..=3".into());
    }
    let anim_rows = fs::read(extracted_dir().join("tileanim.bin"))
        .map_err(|e| format!("tileanim.bin: {e} (regenerate with sml2-extract)"))?;
    if anim_rows.len() != 16 * 0x100 {
        return Err("tileanim.bin: expected 4096 bytes".into());
    }
    let hud: HudStrip =
        serde_json::from_str(&fs::read_to_string(extracted_dir().join("hud.json"))?)?;
    let hud_row = hud_row(&hud, hud_digits.as_deref());
    let spr: Option<SpriteCtx> =
        if sprites_file.is_some() || mario.is_some() || replay_file.is_some() || play_mode {
            let insts: Vec<SpriteInst> = match &sprites_file {
                Some(f) => serde_json::from_str(&fs::read_to_string(f)?)?,
                None => Vec::new(),
            };
            let objblock = fs::read(levels_dir.join(format!("level{nn:02}.objblock.bin")))?;
            if objblock.len() != 0x1000 {
                return Err("objblock.bin: expected 4096 bytes".into());
            }
            let tables: ObjectTables =
                serde_json::from_str(&fs::read_to_string(extracted_dir().join("objects.json"))?)?;
            if let Some(f) = &sprites_file {
                println!("{} object instance(s) from {}", insts.len(), f.display());
            }
            Some(SpriteCtx {
                insts,
                objblock,
                tables,
                level_id: level,
                obp0: manifest.header.palettes[1],
                obp1: manifest.header.palettes[2],
                player: mario,
                bumpers: Vec::new(),
            })
        } else {
            None
        };
    let start = (
        camera
            .map(|c| c.0)
            .unwrap_or(manifest.header.camera_start_x),
        camera
            .map(|c| c.1)
            .unwrap_or(manifest.header.camera_start_y),
    );
    if let Some(rf) = replay_file {
        return run_replay(
            &rf,
            &manifest,
            &layout,
            &bg,
            &anim_rows,
            &hud,
            hud.window_y,
            spr,
            filmstrip.as_deref(),
        );
    }
    if play_mode {
        let mut sim = PlayerSim::new();
        sim.powerup = 0;
        sim.x = manifest.header.mario_spawn_x;
        sim.y = manifest.header.mario_spawn_y;
        let mut cam = sml2_core::camera::CameraSim::new(
            manifest.header.camera_start_x,
            manifest.header.camera_start_y,
        );
        cam.prev_mario_y_lo = sim.y as u8;
        let edge = fs::read(levels_dir.join(format!("level{nn:02}.edge.bin"))).ok();
        let obj = sml2_core::objects::ObjectSim::new(manifest.spawns.records.clone(), cam.x);
        return run_play(
            &manifest,
            &layout,
            &bg,
            &anim_rows,
            &hud,
            hud.window_y,
            scale,
            view,
            spr,
            sim,
            cam,
            obj,
            edge,
        );
    }
    if window_mode {
        run_window(
            &Viewer {
                manifest: &manifest,
                layout: &layout,
                bg_base: &bg,
                anim_rows: &anim_rows,
                hud_row: &hud_row,
                hud_y: hud.window_y,
                scale,
                spr: spr.as_ref(),
                view,
            },
            anim_phase,
            start,
        )?;
    } else {
        let mut bg = bg.clone();
        apply_tile_anim(&mut bg, &manifest.tile_anim, &anim_rows, anim_phase);
        let frame = render_view(
            &manifest,
            &layout,
            &[],
            &bg,
            start.0,
            start.1,
            view.0,
            view.1,
            Some((&hud_row, hud.window_y)),
            spr.as_ref(),
        )?;
        write_png_scaled(&out, view.0, view.1, scale, &frame)?;
        println!(
            "wrote {} (level {}, view {}x{}, camera {}x{}, anim phase {}, scale x{})",
            out.display(),
            level,
            view.0,
            view.1,
            start.0,
            start.1,
            anim_phase,
            scale
        );
    }
    Ok(())
}

/// Build the 20-tile HUD row: ROM template + optional digit substitution
/// (ten digits in column order: lives, coins, money, time; tile = $80 + digit).
fn hud_row(hud: &HudStrip, digits: Option<&str>) -> Vec<u8> {
    let mut row = hud.template.clone();
    if let Some(d) = digits {
        for (i, col) in hud.digit_columns.iter().enumerate() {
            if let Some(v) = d.chars().nth(i).and_then(|c| c.to_digit(10)) {
                if let Some(slot) = row.get_mut(*col as usize) {
                    *slot = 0x80 + v as u8;
                }
            }
        }
    }
    row
}

/// The HUD row with the coin digits taken from the live sim. The engine's
/// `$076B` patches the status bar every in-level frame: `$9C07 = sCoinHigh &
/// $0F`, `$9C08 = sCoinLow >> 4`, `$9C09 = sCoinLow & $0F` (tile = `$80 +
/// digit`). `digit_columns` is in display order (lives, coins, money, time),
/// so the coin group is entries 2..5; everything else keeps the template.
fn hud_row_coins(hud: &HudStrip, coins_high: u8, coins_low: u8) -> Vec<u8> {
    let mut row = hud.template.clone();
    for (k, v) in [coins_high & 0x0F, coins_low >> 4, coins_low & 0x0F]
        .iter()
        .enumerate()
    {
        if let Some(col) = hud.digit_columns.get(2 + k) {
            if let Some(slot) = row.get_mut(*col as usize) {
                *slot = 0x80 + v;
            }
        }
    }
    row
}

/// Overlay the level's animated tiles (VRAM `$9200`..`$923F`) at the given
/// phase onto the composed BG block (offset `$A00` = tile 32). No-op when the
/// level's animation is disabled.
fn apply_tile_anim(bg: &mut [u8], anim: &sml2_core::level::TileAnim, rows: &[u8], phase: u8) {
    if !anim.enabled {
        return;
    }
    let off = anim.row as usize * 0x100 + (phase & 3) as usize * 0x40;
    bg[0xA00..0xA40].copy_from_slice(&rows[off..off + 0x40]);
}

/// Locate `extracted/` whether run from the repo root or a crate dir.
fn extracted_dir() -> PathBuf {
    for cand in ["extracted", "../extracted", "sml2-rs/extracted"] {
        let p = PathBuf::from(cand);
        if p.join("levels").is_dir() {
            return p;
        }
    }
    PathBuf::from("extracted")
}

/// Everything the window viewer needs per frame.
struct Viewer<'a> {
    manifest: &'a LevelManifest,
    layout: &'a LevelLayout,
    bg_base: &'a [u8],
    anim_rows: &'a [u8],
    hud_row: &'a [u8],
    hud_y: u8,
    scale: usize,
    spr: Option<&'a SpriteCtx>,
    /// Viewport in logical pixels (`--size`).
    view: (usize, usize),
}

/// One live object instance fed to the renderer: world position, sprite id and
/// the object's `sSpriteFlipFlags` byte (bit4 = OBP select, bit5 = X flip,
/// bit6 = Y flip, bit7 = BG-over-OBJ priority).
#[derive(Debug, Clone, Deserialize)]
struct SpriteInst {
    sprite: u8,
    x: u16,
    y: u16,
    #[serde(default)]
    flip: u8,
}

/// Everything needed for the OBJ pass.
struct SpriteCtx {
    insts: Vec<SpriteInst>,
    objblock: Vec<u8>,
    tables: ObjectTables,
    level_id: u8,
    obp0: u8,
    obp1: u8,
    /// Optional player (Mario): world position + `$FFC6` frame.
    player: Option<(u16, u16, u8)>,
    /// Brick-bump sprites for this frame (`PlayerSim::block_draw`): the engine
    /// stages `$FFC4`/`$FFC5`/`$FFC6` and enqueues them (`$5297`), so they are
    /// extra OAM sprites drawn at the block — through the same player-sprite
    /// path, after Mario (his entries come first, so he wins overlaps).
    bumpers: Vec<(u16, u16, u8)>,
}

/// Interactive viewer: arrow keys scroll the camera, Q/ESC quits; the window
/// is resizable (integer scale + letterbox). Animated tiles advance on the
/// engine's own cadence (`speed * 2` frames per phase).
fn run_window(
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
        )?;
        // Integer-scale + letterbox into the current window size.
        let (win_w, win_h) = window.get_size();
        if win_w == 0 || win_h == 0 {
            continue;
        }
        if buf.len() != win_w * win_h {
            buf = vec![0; win_w * win_h];
        } else {
            buf.fill(0);
        }
        let s = (win_w / vw).min(win_h / vh).max(1);
        let ox = win_w.saturating_sub(vw * s) / 2;
        let oy = win_h.saturating_sub(vh * s) / 2;
        for y in 0..vh {
            let py0 = oy + y * s;
            if py0 + s > win_h {
                break;
            }
            for x in 0..vw {
                let px0 = ox + x * s;
                if px0 + s > win_w {
                    break;
                }
                let g = frame[y * vw + x] as u32;
                let px = g | (g << 8) | (g << 16);
                for dy in 0..s {
                    let row = (py0 + dy) * win_w + px0;
                    buf[row..row + s].fill(px);
                }
            }
        }
        window
            .update_with_buffer(&buf, win_w, win_h)
            .map_err(|e| e.to_string())?;
    }
    println!("camera ended at {}x{}", cx, cy);
    Ok(())
}

/// Keep a viewport span inside `world`: clamp when it fits, centre when the
/// viewport is larger than the world.
fn clamp_axis(lo0: i32, span: i32, world: i32) -> i32 {
    if span >= world {
        (world - span) / 2
    } else {
        lo0.clamp(0, world - span)
    }
}

/// Render a `vw`x`vh` view centred on the camera as an 8-bit grey frame. The
/// native 160x144 rect stays pixel-exact inside it; `hud` draws the
/// status-bar row at its native position (window Y within the rect).
///
/// `overlay` is the sim's sparse level-map rewrite list (`layout_writes`: keys
/// `row * 256 + col`, the engine's `$B000` addressing). The engine edits the
/// same WRAM map its probes read, so a cell the sim rewrote — a collected
/// coin's `$60`, a bump's `$24`/`$07`/… — must draw as the new metatile.
#[allow(clippy::too_many_arguments)]
fn render_view(
    manifest: &LevelManifest,
    layout: &LevelLayout,
    overlay: &[(u32, u8)],
    bg: &[u8],
    cam_x: u16,
    cam_y: u16,
    vw: usize,
    vh: usize,
    hud: Option<(&[u8], u8)>,
    spr: Option<&SpriteCtx>,
) -> Result<Vec<u8>, String> {
    // Decode every tile in the composed block once.
    let tiles: Vec<[u8; 64]> = (0..256u16)
        .map(|t| {
            let off = video::bg_tile_offset(t as u8);
            let bytes: [u8; 16] = bg[off..off + 16].try_into().expect("block is $1000 bytes");
            video::tile_colors(&bytes)
        })
        .collect();
    let shades = video::bgp_shades(manifest.header.palettes[0]);
    let world_w = (LAYOUT_WIDTH * 16) as i32;
    let world_h = (LAYOUT_HEIGHT * 16) as i32;
    let sx = clamp_axis(cam_x as i32 - vw as i32 / 2, vw as i32, world_w);
    let sy = clamp_axis(cam_y as i32 - vh as i32 / 2, vh as i32, world_h);
    let mut img = vec![shades[0]; vw * vh];
    let mut bg_idx = vec![0u8; vw * vh];
    for py in 0..vh {
        let wy = sy + py as i32;
        if !(0..world_h).contains(&wy) {
            continue;
        }
        let (cy, ty) = ((wy as usize) >> 4, (wy as usize) & 15);
        for px in 0..vw {
            let wx = sx + px as i32;
            if !(0..world_w).contains(&wx) {
                continue;
            }
            let (cx, tx) = ((wx as usize) >> 4, (wx as usize) & 15);
            let key = (cy * 256 + cx) as u32;
            let Some(id) = overlay
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| *v)
                .or_else(|| layout.get(cx, cy))
            else {
                continue;
            };
            let mt = manifest
                .metatiles
                .get(id)
                .ok_or_else(|| format!("metatile id {id} out of table"))?;
            let q = (ty >> 3) * 2 + (tx >> 3);
            let tile = mt.tiles[q] as usize;
            let col = tiles[tile][(ty & 7) * 8 + (tx & 7)];
            bg_idx[py * vw + px] = col;
            img[py * vw + px] = shades[col as usize];
        }
    }
    // Window layer: the status-bar strip (no scroll; window at WX=7 -> px 0).
    // It stays in the native rect, centred in a larger viewport.
    if let Some((row, wy)) = hud {
        let hx0 = (vw as i32 - SCREEN_W as i32) / 2;
        let hy0 = (vh as i32 - SCREEN_H as i32) / 2 + wy as i32;
        for dy in 0..8usize {
            let py = hy0 + dy as i32;
            if py < 0 || py >= vh as i32 {
                continue;
            }
            for col in 0..SCREEN_W {
                let Some(&tile) = row.get(col) else { break };
                for dx in 0..8usize {
                    let px = hx0 + (col * 8 + dx) as i32;
                    if px < 0 || px >= vw as i32 {
                        continue;
                    }
                    let c = tiles[tile as usize][dy * 8 + dx];
                    let idx = py as usize * vw + px as usize;
                    bg_idx[idx] = c;
                    img[idx] = shades[c as usize];
                }
            }
        }
    }
    // OBJ sprites: the player first (its cells are OAM entries 0.., highest
    // priority), then objects — mapping entries expanded like $C000/$5297
    // (attr ^= the object's flip flags). GB priority: smaller X first, then
    // earlier instance; BG wins where attr bit 7 is set and the BG pixel is
    // non-transparent.
    if let Some(s) = spr {
        let pal0 = video::bgp_shades(s.obp0);
        let pal1 = video::bgp_shades(s.obp1);
        let mut taken = vec![false; vw * vh];
        if let Some((mx, my, frame)) = s.player {
            let cells =
                sml2_core::player::draw_cells(&s.tables, frame, mx, my, cam_x, cam_y, false)
                    .ok_or_else(|| format!("player frame {frame} has no mapping"))?;
            // The player base already contains the camera (+96/+98 is world -
            // cam), so these cells are in final screen space: no `sx/sy`.
            for c in cells {
                blit_obj_cell(
                    &mut img,
                    &bg_idx,
                    &mut taken,
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8,
                    c.y as i32 - 16,
                    c.tile,
                    c.attr,
                    vw,
                    vh,
                );
            }
        }
        // The brick-bump sprites ($2C82's $5297 enqueues), drawn after Mario's
        // cells and before the objects — the engine's OAM order.
        let bumpers = s.bumpers.clone();
        for (bx, by, bf) in bumpers {
            let cells = sml2_core::player::draw_cells(&s.tables, bf, bx, by, cam_x, cam_y, false)
                .ok_or_else(|| format!("bump frame {bf} has no mapping"))?;
            for c in cells {
                blit_obj_cell(
                    &mut img,
                    &bg_idx,
                    &mut taken,
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8,
                    c.y as i32 - 16,
                    c.tile,
                    c.attr,
                    vw,
                    vh,
                );
            }
        }
        let mut order: Vec<usize> = (0..s.insts.len()).collect();
        order.sort_by_key(|&i| (s.insts[i].x, i));
        for &i in &order {
            let inst = &s.insts[i];
            let Some(mapping) = s.tables.mapping_for(s.level_id, inst.sprite) else {
                continue;
            };
            let flip_x = inst.flip & 0x20 != 0;
            let flip_y = inst.flip & 0x40 != 0;
            for e in mapping {
                let attr = e.attr ^ inst.flip;
                let ex = if flip_x {
                    (!e.x).wrapping_sub(7) as i8 as i32
                } else {
                    e.x as i32
                };
                let ey = if flip_y {
                    (!e.y).wrapping_sub(7) as i8 as i32
                } else {
                    e.y as i32
                };
                blit_obj_cell(
                    &mut img,
                    &bg_idx,
                    &mut taken,
                    &s.objblock,
                    &pal0,
                    &pal1,
                    inst.x as i32 - sx + ex,
                    inst.y as i32 - sy + ey,
                    e.tile,
                    attr,
                    vw,
                    vh,
                );
            }
        }
    }
    Ok(img)
}

/// Blit one 8x8 OAM cell at screen `(px0, py0)` with the engine's rules:
/// colour 0 is transparent, attr bit5 = X flip / bit6 = Y flip / bit4 = OBP1 /
/// bit7 = BG priority, earlier cells win overlaps, and OBJ tiles use the
/// `$8000` mode (`$8000 + t*16`).
#[allow(clippy::too_many_arguments)]
fn blit_obj_cell(
    img: &mut [u8],
    bg_idx: &[u8],
    taken: &mut [bool],
    objblock: &[u8],
    pal0: &[u8],
    pal1: &[u8],
    px0: i32,
    py0: i32,
    tile: u8,
    attr: u8,
    vw: usize,
    vh: usize,
) {
    if px0 <= -8 || px0 >= vw as i32 || py0 <= -8 || py0 >= vh as i32 {
        return;
    }
    let (xf, yf) = (attr & 0x20 != 0, attr & 0x40 != 0);
    let pal = if attr & 0x10 != 0 { pal1 } else { pal0 };
    let t = tile as usize;
    let off = if t < 128 {
        t * 16
    } else {
        0x800 + (t - 128) * 16
    };
    let bytes: [u8; 16] = objblock[off..off + 16]
        .try_into()
        .expect("objblock is $1000 bytes");
    let colors = video::tile_colors(&bytes);
    for dy in 0..8 {
        for dx in 0..8 {
            let src = (if yf { 7 - dy } else { dy }) * 8 + (if xf { 7 - dx } else { dx });
            let c = colors[src];
            if c == 0 {
                continue;
            }
            let (px, py) = (px0 + dx as i32, py0 + dy as i32);
            if px < 0 || px >= vw as i32 || py < 0 || py >= vh as i32 {
                continue;
            }
            let idx = py as usize * vw + px as usize;
            if taken[idx] || (attr & 0x80 != 0 && bg_idx[idx] != 0) {
                continue;
            }
            taken[idx] = true;
            img[idx] = pal[c as usize];
        }
    }
}

/// Write an 8-bit greyscale PNG, upscaled by an integer factor (nearest).
fn write_png_scaled(
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

/// One captured live frame (`trajectory_capture*.py`) for sim replays.
#[derive(Deserialize)]
struct Frame {
    x: u16,
    y: u16,
    hvel: u8,
    vvel: u8,
    ground: bool,
    dir_right: bool,
    camx: u8,
    #[serde(default)]
    camxh: u8,
    #[serde(default)]
    camy: u8,
    #[serde(default)]
    camyh: u8,
    #[serde(default)]
    frame: u8,
    #[serde(default)]
    edge: u8,
    #[serde(default)]
    limit: bool,
    #[serde(default)]
    autoscroll: u8,
    #[serde(default)]
    en_r: bool,
    #[serde(default)]
    en_l: bool,
    #[serde(default)]
    slots: Vec<SlotJ>,
    #[serde(default)]
    sptr: u16,
    #[serde(default)]
    spx: u16,
    #[serde(default)]
    sdir: u8,
    #[serde(default)]
    standby: u8,
    #[serde(default)]
    active: u8,
    #[serde(default)]
    easy: u8,
    #[serde(default)]
    odd: u8,
    phase: u8,
    /// `$FFF0` (the full byte; `& 3` gates sprite toggles / state machines).
    #[serde(default)]
    ff0: Option<u8>,
    /// `sObjectUpdatePhase`.
    #[serde(default)]
    uphase: Option<u8>,
    /// `sPipeTravelDirection` ($A221) — arming + travel state.
    #[serde(default)]
    pipe: Option<u8>,
    /// `sScreenTransition` ($A20E) — the `$0DED` scroll state.
    #[serde(default)]
    tr: Option<u8>,
    /// The engine's previous-camera latch ($AF24/$AF25) — the scan-direction
    /// source. Captures sampled at the level's very first frame need it
    /// (the load leaves it 0, so frame 1 scans forward).
    #[serde(default)]
    pcx: Option<u16>,
    /// `sMoveFlags`.
    #[serde(default)]
    mf: Option<u8>,
    /// `sObjectFacing`.
    #[serde(default)]
    fac: Option<u8>,
    /// `sMarioDead`.
    #[serde(default)]
    dead: Option<u8>,
    /// `sStompBounceTimer` / `sStompChainCount` / `sMoneyBagCount`.
    #[serde(default)]
    sbt: Option<u8>,
    #[serde(default)]
    scc: Option<u8>,
    #[serde(default)]
    mb: Option<u8>,
    /// `sCameraPanTimer`.
    #[serde(default)]
    pan: Option<u8>,
    /// `sLives` ($A22C, BCD) — present in full-death-cycle captures.
    #[serde(default)]
    lives: Option<u8>,
    /// `sCurPowerup` / `sInvincibilityTimer` / `sGoalBell` / `sSecretRouteFlag`
    /// — the `$3451` item-consumer observables.
    #[serde(default)]
    pow: Option<u8>,
    #[serde(default)]
    inv: Option<u8>,
    #[serde(default)]
    goal: Option<u8>,
    #[serde(default)]
    secret: Option<u8>,
    /// `sSpinJumpBounce` ($A220) — the ceiling-bonk flag (`$FF` after a bump).
    #[serde(default)]
    spin: Option<u8>,
    /// `sCoinLow`/`sCoinHigh` ($A262/$A263, BCD).
    #[serde(default)]
    coins: Option<u8>,
    #[serde(default)]
    coinshi: Option<u8>,
    /// The 8 `sBlockAnimSlots` ($AA80) as 16-byte hex dumps — the brick-bump
    /// system's whole state (type, timer, Y, X per slot).
    #[serde(default)]
    bslots: Vec<String>,
    /// The fade/map/reload gap carried by the first post-respawn record of a
    /// full-death-cycle capture.
    #[serde(default)]
    gap: Option<GapJ>,
    /// 16 raw 32-byte slot dumps (64 hex chars each) — the full-byte check.
    #[serde(default)]
    slot_bytes: Vec<String>,
    #[allow(dead_code)]
    timer: u8,
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
}

#[derive(Deserialize, Default)]
struct SlotJ {
    #[serde(default)]
    state: u8,
    #[serde(default)]
    x: u16,
    #[serde(default)]
    y: u16,
    #[serde(rename = "def_", default)]
    def_: u8,
    #[serde(default)]
    sprite: u8,
    #[serde(default)]
    rec: u16,
}

/// The unmodelled stretch of a full-death-cycle capture: the state machine
/// between the fade trigger and the level re-entry (the `$14` lives/save/map
/// setup → the input-dependent overworld map → the reload), as
/// `[[state, ticks], ...]` polled from `$FF9B`, plus the total tick gap.
#[derive(Deserialize)]
struct GapJ {
    total: u64,
    seq: Vec<[u64; 2]>,
}

/// Replay a captured trajectory through `sml2_core::player_sim` and report
/// frame-by-frame divergences from the live console state. With `filmstrip`,
/// also renders every 30th frame (sim + camera + player draw) as a 2x PNG.
#[allow(clippy::too_many_arguments)]
fn run_replay(
    path: &Path,
    manifest: &LevelManifest,
    layout: &LevelLayout,
    bg_base: &[u8],
    anim_rows: &[u8],
    hud: &HudStrip,
    hud_y: u8,
    mut spr: Option<SpriteCtx>,
    filmstrip: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let frames: Vec<Frame> = serde_json::from_str(&fs::read_to_string(path)?)?;
    if frames.len() < 2 {
        return Err("replay: need at least 2 frames".into());
    }
    let mut sim = PlayerSim::new();
    sim.powerup = 0; // level-0 spawn starts small
    let f0 = &frames[0];
    sim.x = f0.x;
    sim.y = f0.y;
    sim.hvel = f0.hvel;
    sim.vvel = f0.vvel;
    sim.on_ground = f0.ground;
    sim.in_air = !f0.ground;
    sim.dir_right = f0.dir_right;
    sim.walk_timer = f0.timer;
    sim.lives = f0.lives.unwrap_or(5);
    let mut cam = sml2_core::camera::CameraSim::new(
        f0.camx as u16 | ((f0.camxh as u16 & 0x0F) << 8),
        f0.camy as u16 | ((f0.camyh as u16 & 0x0F) << 8),
    );
    cam.prev_mario_y_lo = f0.y as u8;
    cam.map_scroll_limit = f0.limit;
    cam.auto_scroll = f0.autoscroll;
    let mut obj = sml2_core::objects::ObjectSim::new(manifest.spawns.records.clone(), cam.x);
    obj.easy_mode = f0.easy != 0;
    obj.configure(manifest.level_id);
    obj.fff0 = f0.ff0.unwrap_or(u8::from(f0.odd != 0));
    obj.update_phase = f0.uphase.unwrap_or(0);
    // The toggle flips `phase`, so the recorded phase seeds the bool directly
    // (a capture sampled before the level's first frame has uphase 0).
    if let Some(u) = f0.uphase {
        obj.phase = u == 1;
    }
    if let Some(p) = f0.pcx {
        obj.prev_cam_x = p;
    }
    obj.odd = obj.fff0 & 1 == 1;
    // The object handlers need the anim→sprite/hitbox tables; clone them out
    // of the sprite context so the loop can still borrow it mutably.
    let tables = spr
        .as_ref()
        .map(|s| s.tables.clone())
        .ok_or("replay requires objects.json (no sprite context)")?;
    if cam.auto_scroll != 0 {
        println!("note: level is autoscroll; that path is not modelled");
    }
    if let Some(dir) = filmstrip {
        fs::create_dir_all(dir)?;
    }
    let mut counts: Vec<usize> = Vec::new();
    let mut names: Vec<&'static str> = Vec::new();
    let mut shown = 0usize;
    let mut gap_ok = true;
    let mut prev_a = f0.a;
    for (i, f) in frames.iter().enumerate().skip(1) {
        let keys = PlayerKeys {
            right: f.right,
            left: f.left,
            up: f.up,
            down: f.down,
            a: f.a,
            b: f.b,
        };
        let a_pressed = f.a && !prev_a;
        prev_a = f.a;
        // $FF97 & 7 — the speed-dither phase. The engine increments it once
        // per frame ($0300/$2729 wait for the VBlank flag, then `inc`), so
        // the $181C integration always reads "previous residue + 1". The
        // capture samples $FF97 at the PyBoy frame boundary, which slips an
        // extra increment on traj10 f2986/2988/2989 — the recorded x-deltas
        // there only match the cadence value (f2988's console step is +1,
        // where $FF97 = 5 would force +2). Keep the record when it agrees
        // with the cadence; the death captures' per-logic-frame sampling
        // breaks the cadence elsewhere, so the record still wins there.
        let expect = (sim.phase + 1) & 7;
        sim.phase = if f.phase == (expect + 1) & 7 {
            expect
        } else {
            f.phase
        };
        sim.camx = f.camx as u16;
        sim.camy = f.camy as u16;
        // $05E4/$0640: the frame loop's three branches. Dead (`$FF`) runs only
        // the arc + the object engine (the camera freezes); the death fade
        // (`$FF9B = 9`) leaves the level entirely, freezing everything.
        let was_dead = sim.dead == 0xFF;
        // The fade-trigger frame is the last one to run the object engine
        // (the dead branch continues into `$0652`); the true fade frames do
        // not. Capture that before the step flips it.
        //
        // `$FF9B` is what the frame loop dispatches on: 9 = the death fade
        // (state `$09`), 24 = the exit fade the `$1CBA` bell arm starts
        // (state `$18`). Both leave the in-level loop, so the player, camera
        // and objects all freeze while either is set. VBlank's `sFadeTimer`
        // decrement still runs, though: stepping the exit fade is `$3547`'s
        // poll, and its last call credits the BCD tally and marks the `$14`
        // hand-off. (The death fade's decrement stays with the gap handler
        // below, whose duration check owns that cycle.)
        let fade_at_entry = sim.dead_fade || sim.exit_phase;
        if sim.dead_fade || sim.exit_phase {
            if sim.exit_phase {
                sim.exit_fade_step();
            }
        } else if was_dead {
            sim.death_arc(cam.y);
        } else {
            // a screen transition skips the player update — the camera's
            // `$0DED` handler moves Mario instead.
            if cam.transition == 0 {
                sim.step(&keys, a_pressed, layout);
            }
            // `$3451` id 4's conditional `sStompChainCount` reset (star
            // pickup on an expired timer). The counter lives in the object
            // engine, so the consumer's one-shot flag is applied here —
            // before this frame's object pass can bump it again.
            if sim.take_stomp_chain_reset() {
                obj.stomp_chain = 0;
            }
            // $2F72 — the block-bump alloc's tail. It runs inside the player
            // update ($0F71), before $0B2A/$2062 recompute the scroll, so its
            // screen math reads the *previous* frame's sScroll — the camera
            // this loop still holds before `cam.step` below. The object engine
            // ($8000) reads the below-hit flag later this frame, so the pass
            // has to land before `obj.step`; `resolve_bump_coin` is the
            // `$19D2` coin half of the split.
            if let Some((by, bx)) = sim.below_bonk.take() {
                let scroll_y =
                    cam.y
                        .wrapping_sub(72)
                        .wrapping_sub((f.pan.unwrap_or(0) & 3) as u16) as u8;
                let scroll_x = cam.x.wrapping_sub(80) as u8;
                let special = obj.below_bonk(by, bx, scroll_y, scroll_x);
                sim.resolve_bump_coin(special);
            }
            cam.edge_now = Some(f.edge);
            cam.step(&mut sim);
            cam.update_enables(sim.x);
        }
        let mario = sml2_core::objects::MarioContact {
            x: sim.x,
            y: sim.y,
            dead: sim.dead,
            powerup: sim.powerup,
            ducking: keys.down && sim.on_ground,
            spin_jump: sim.spin_jump != 0,
            star_high: sim.star_high,
            invincibility: sim.invincibility,
            flutter: 0,
            vvel: sim.vvel,
            on_ground: sim.on_ground,
            on_object: false,
            b_held: keys.b,
            a_held: keys.a,
            scene_state: 0,
            moon_physics: sim.moon,
            pan: f.pan.unwrap_or(0),
            travelling: sim.pipe_dir >= 0x10,
        };
        // $2D0A: the frame selection runs *before* the interaction pass, so
        // on the contact frame the sprite still shows the grounded pose.
        let frame_idx = if was_dead {
            107
        } else {
            sim.frame_index(sim.walk_timer)
        };
        // $2C82 (`$5997`): the brick-bump update sits between the frame
        // selection (`$2D0A`) and the object engine, so when a bump draws, its
        // frame is what `$FFC6` — and the capture's `frame` check — ends the
        // frame with. The dead branch skips it (no `$2C82` there), and so does
        // the level-leaving freeze.
        let frame_check = if was_dead || fade_at_entry {
            frame_idx
        } else {
            sim.block_update();
            sim.block_frame.unwrap_or(frame_idx)
        };
        if !fade_at_entry {
            // $8000's dispatcher gate reads sMarioDead; the interaction pass
            // then writes it back (15 on a harmful hit).
            obj.mario_dead = sim.dead;
            obj.step(cam.x, cam.y, layout, &tables, &mario);
            if sim.dead == 0 && obj.mario_dead != 0 {
                sim.dead = obj.mario_dead;
            }
            // `$2DC6`'s item branch stages `sPendingItemId`; the *next* frame's
            // `$3451` (inside `$0F71`) consumes it.
            if obj.pending_item != 0 {
                sim.pending_item = obj.pending_item;
                obj.pending_item = 0;
            }
            // The interaction pass writes back into Mario's state.
            if let Some(y) = obj.mario_y_adjust {
                sim.y = y;
            }
            if obj.on_ground_cleared {
                sim.on_ground = false;
            }
            if obj.stomp_jump {
                sim.in_air = true;
                sim.vvel = if keys.a { 75 } else { 48 };
            }
            if obj.stomp_bounce {
                sim.in_air = true;
                sim.vvel = 49;
            }
        }
        // The full-death-cycle gap: after the fade-trigger frame the game
        // leaves the level (fade 192 frames → `$14` lives/save/map setup →
        // the overworld map, which waits for input → re-entry reloads the
        // level). Only the fade and the reload's final state are modelled;
        // the input-dependent map is skipped over the captured tick gap.
        if let Some(g) = &f.gap {
            let fade_capture = g.seq.first().copied();
            let modeled = 192u64;
            let acct: u64 = 1 + g.seq.iter().map(|s| s[1]).sum::<u64>();
            let fade_ok = matches!(fade_capture, Some([9, n]) if n == modeled)
                && sim.dead_fade
                && sim.fade_timer == modeled as u16;
            // Run the modelled fade (its last step decrements the lives).
            while sim.fade_timer > 0 {
                sim.fade_step();
            }
            // $0F2A + $04F1 + $EBBB: the reload puts Mario back at the
            // header spawn and clears the object scratch (fff0 persists).
            sim.respawn(manifest.header.mario_spawn_x, manifest.header.mario_spawn_y);
            cam.reset_level_start(
                manifest.header.camera_start_x,
                manifest.header.camera_start_y,
                sim.y as u8,
            );
            obj.reset_level_start();
            let skipped = g.total.saturating_sub(1 + modeled);
            let ok = fade_ok && acct == g.total;
            println!(
                "  frame {i}: death cycle: fade {modeled} modeled vs {fade_capture:?} captured; +{skipped} unmodeled ticks skipped (seq {:?}, total {} vs {} accounted){}",
                g.seq,
                g.total,
                acct,
                if ok { "" } else { "  <- MISMATCH" }
            );
            gap_ok = gap_ok && ok;
        }
        if let Some(dir) = filmstrip {
            if i % 30 == 0 {
                if let Some(s) = spr.as_mut() {
                    s.player = Some((sim.x, sim.y, frame_idx));
                    // The dead path's own draw overwrites the staging pair.
                    s.bumpers = if was_dead {
                        Vec::new()
                    } else {
                        sim.block_draw.clone()
                    };
                }
                let mut rbg = bg_base.to_vec();
                apply_tile_anim(&mut rbg, &manifest.tile_anim, anim_rows, 0);
                let hud_live = hud_row_coins(hud, sim.coins_high, sim.coins_low);
                let frame = render_view(
                    manifest,
                    layout,
                    &sim.layout_writes,
                    &rbg,
                    cam.x,
                    cam.y,
                    SCREEN_W,
                    SCREEN_H,
                    Some((&hud_live, hud_y)),
                    spr.as_ref(),
                )?;
                let out = dir.join(format!("frame_{i:04}.png"));
                write_png_scaled(&out, SCREEN_W, SCREEN_H, 2, &frame)?;
            }
        }
        let mut checks: Vec<(&'static str, bool, String)> = Vec::new();
        {
            let (slot_ok, slot_msg) = {
                let mut bad = false;
                let mut msg = String::new();
                for (k, sl) in f.slots.iter().enumerate() {
                    let s = &obj.slots[k];
                    let st = match s.state() {
                        sml2_core::objects::SlotState::Inactive => 0u8,
                        sml2_core::objects::SlotState::Standby => 1,
                        sml2_core::objects::SlotState::Active => 2,
                    };
                    let ok = if st != sl.state {
                        false
                    } else if st == 0 {
                        true
                    } else {
                        s.def() == sl.def_
                            && s.sprite() == sl.sprite
                            && s.rec_index() == sl.rec
                            && s.x() == sl.x
                            && s.y() == sl.y
                    };
                    if !ok {
                        bad = true;
                        if msg.is_empty() {
                            msg = format!(
                                "slot {k}: state sim={st} exp={} def sim={} exp={} spr sim={} exp={} rec sim=${:04X} exp=${:04X} pos sim=({},{})",
                                sl.state, s.def(), sl.def_, s.sprite(), sl.sprite, s.rec_index(), sl.rec,
                                s.x(), s.y()
                            );
                        }
                    }
                }
                (!bad, msg)
            };
            let mut push = |name: &'static str, ok: bool, msg: String| {
                checks.push((name, ok, msg));
            };
            push("x", sim.x == f.x, format!("x sim={} exp={}", sim.x, f.x));
            push("y", sim.y == f.y, format!("y sim={} exp={}", sim.y, f.y));
            push(
                "hvel",
                sim.hvel == f.hvel,
                format!("hvel sim=${:02X} exp=${:02X}", sim.hvel, f.hvel),
            );
            push(
                "vvel",
                sim.vvel == f.vvel,
                format!("vvel sim=${:02X} exp=${:02X}", sim.vvel, f.vvel),
            );
            push(
                "ground",
                sim.on_ground == f.ground,
                format!("ground sim={} exp={}", sim.on_ground, f.ground),
            );
            push(
                "timer",
                sim.walk_timer == f.timer,
                format!("timer sim={} exp={}", sim.walk_timer, f.timer),
            );
            push(
                "dir",
                sim.dir_right == f.dir_right,
                format!("dir sim={} exp={}", sim.dir_right, f.dir_right),
            );
            push(
                "frame",
                frame_check == f.frame,
                format!("frame sim={} exp={}", frame_check, f.frame),
            );
            push(
                "camx",
                (cam.x as u8) == f.camx && ((cam.x >> 8) as u8 & 0x0F) == (f.camxh & 0x0F),
                format!(
                    "camx sim=${:03X} exp=${:02X}{:X}",
                    cam.x,
                    f.camxh & 0x0F,
                    f.camx
                ),
            );
            push(
                "camy",
                (cam.y as u8) == f.camy && ((cam.y >> 8) as u8 & 0x0F) == (f.camyh & 0x0F),
                format!(
                    "camy sim=${:03X} exp=${:02X}{:X}",
                    cam.y,
                    f.camyh & 0x0F,
                    f.camy
                ),
            );
            push(
                "en_r",
                cam.right_enabled == f.en_r,
                format!("en_r sim={} exp={}", cam.right_enabled, f.en_r),
            );
            push(
                "en_l",
                cam.left_enabled == f.en_l,
                format!("en_l sim={} exp={}", cam.left_enabled, f.en_l),
            );
            push(
                "sptr",
                obj.stream_index as u16 + 0xAB06 == f.sptr,
                format!(
                    "sptr sim=${:04X} exp=${:04X}",
                    obj.stream_index as u16 + 0xAB06,
                    f.sptr
                ),
            );
            push(
                "spx",
                obj.spawn_x == f.spx,
                format!("spx sim={} exp={}", obj.spawn_x, f.spx),
            );
            push(
                "sdir",
                obj.scan_dir == f.sdir,
                format!("sdir sim={} exp={}", obj.scan_dir, f.sdir),
            );
            push(
                "standby",
                obj.standby == f.standby,
                format!("standby sim={} exp={}", obj.standby, f.standby),
            );
            push(
                "active",
                obj.active == f.active,
                format!("active sim={} exp={}", obj.active, f.active),
            );
            push("objslots", slot_ok, slot_msg);
            push(
                "odd",
                obj.odd == (f.odd != 0),
                format!("odd sim={} exp={}", obj.odd, f.odd),
            );
            // Raw slot bytes: the strongest check (every byte the engine
            // writes, including positions, sprites, frame state, flags).
            if !f.slot_bytes.is_empty() {
                let mut bad: Vec<usize> = Vec::new();
                let mut detail = String::new();
                for (k, hex) in f.slot_bytes.iter().enumerate() {
                    if k >= 16 {
                        break;
                    }
                    let Some(exp) = parse_slot_hex(hex) else {
                        continue;
                    };
                    if obj.slots[k].b != exp {
                        bad.push(k);
                        if detail.is_empty() {
                            let mut diffs = String::new();
                            for (j, &ev) in exp.iter().enumerate() {
                                if obj.slots[k].b[j] != ev {
                                    diffs += &format!(
                                        " +{j:02X} sim=${:02X} exp=${:02X};",
                                        obj.slots[k].b[j], ev
                                    );
                                }
                            }
                            detail = format!("slot {k}:{diffs}");
                        }
                    }
                }
                push("slots_raw", bad.is_empty(), detail);
            }
            if let Some(v) = f.mf {
                push(
                    "mf",
                    obj.move_flags == v,
                    format!("mf sim=${:02X} exp=${:02X}", obj.move_flags, v),
                );
            }
            if let Some(v) = f.fac {
                push(
                    "fac",
                    obj.facing == v,
                    format!("fac sim=${:02X} exp=${:02X}", obj.facing, v),
                );
            }
            if let Some(v) = f.dead {
                push(
                    "dead",
                    obj.mario_dead == v,
                    format!("dead sim={} exp={}", obj.mario_dead, v),
                );
            }
            if let Some(v) = f.lives {
                push(
                    "lives",
                    sim.lives == v,
                    format!("lives sim={:02X} exp={:02X}", sim.lives, v),
                );
            }
            if let Some(v) = f.pow {
                push(
                    "pow",
                    sim.powerup == v,
                    format!("pow sim={} exp={}", sim.powerup, v),
                );
            }
            if let Some(v) = f.inv {
                push(
                    "inv",
                    sim.invincibility == v,
                    format!("inv sim={} exp={}", sim.invincibility, v),
                );
            }
            if let Some(v) = f.goal {
                // The ROM stores 12 (`$3451` id 6); the port only ever tests it
                // for nonzero, so the sim keeps a bool.
                push(
                    "goal",
                    (if sim.goal_bell { 12 } else { 0 }) == v,
                    format!("goal sim={} exp={}", if sim.goal_bell { 12 } else { 0 }, v),
                );
            }
            if let Some(v) = f.secret {
                push(
                    "secret",
                    sim.secret_route == v,
                    format!("secret sim=${:02X} exp=${:02X}", sim.secret_route, v),
                );
            }
            if let Some(v) = f.sbt {
                push(
                    "sbt",
                    obj.stomp_bounce_timer == v,
                    format!("sbt sim={} exp={}", obj.stomp_bounce_timer, v),
                );
            }
            if let Some(v) = f.scc {
                push(
                    "scc",
                    obj.stomp_chain == v,
                    format!("scc sim={} exp={}", obj.stomp_chain, v),
                );
            }
            if let Some(v) = f.mb {
                push(
                    "mb",
                    obj.money_bag == v,
                    format!("mb sim=${:02X} exp=${:02X}", obj.money_bag, v),
                );
            }
            if let Some(v) = f.uphase {
                push(
                    "uphase",
                    obj.update_phase == v,
                    format!("uphase sim={} exp={}", obj.update_phase, v),
                );
            }
            if let Some(v) = f.ff0 {
                push(
                    "ff0",
                    obj.fff0 == v,
                    format!("ff0 sim=${:02X} exp=${:02X}", obj.fff0, v),
                );
            }
            if let Some(v) = f.pipe {
                push(
                    "pipe",
                    sim.pipe_dir == v,
                    format!("pipe sim=${:02X} exp=${:02X}", sim.pipe_dir, v),
                );
            }
            if let Some(v) = f.tr {
                push(
                    "tr",
                    cam.transition == v,
                    format!("tr sim=${:02X} exp=${:02X}", cam.transition, v),
                );
            }
            if let Some(v) = f.pcx {
                push(
                    "pcx",
                    obj.prev_cam_x == v,
                    format!("pcx sim=${:04X} exp=${:04X}", obj.prev_cam_x, v),
                );
            }
            if let Some(v) = f.spin {
                push(
                    "spin",
                    sim.spin_jump_bounce == v,
                    format!("spin sim=${:02X} exp=${:02X}", sim.spin_jump_bounce, v),
                );
            }
            if let Some(v) = f.coins {
                push(
                    "coins",
                    sim.coins_low == v,
                    format!("coins sim=${:02X} exp=${:02X}", sim.coins_low, v),
                );
            }
            if let Some(v) = f.coinshi {
                push(
                    "coinshi",
                    sim.coins_high == v,
                    format!("coinshi sim=${:02X} exp=${:02X}", sim.coins_high, v),
                );
            }
            // The bump slots, byte for byte — the timer sequence, the freed
            // slot's stale Y/X and the two-slot `$01` bump all live here.
            if !f.bslots.is_empty() {
                let mut bad: Vec<usize> = Vec::new();
                let mut detail = String::new();
                for (k, hex) in f.bslots.iter().enumerate() {
                    if k >= 8 {
                        break;
                    }
                    let Some(exp) = parse_slot16_hex(hex) else {
                        continue;
                    };
                    if sim.block_slots[k] != exp {
                        bad.push(k);
                        if detail.is_empty() {
                            let mut diffs = String::new();
                            for (j, &ev) in exp.iter().enumerate() {
                                if sim.block_slots[k][j] != ev {
                                    diffs += &format!(
                                        "+{j:02X} sim=${:02X} exp=${ev:02X}; ",
                                        sim.block_slots[k][j]
                                    );
                                }
                            }
                            detail = format!("bslot {k}: {diffs}");
                        }
                    }
                }
                push(
                    "bslots",
                    bad.is_empty(),
                    if bad.is_empty() {
                        String::new()
                    } else {
                        format!("{detail} ({} bad slots)", bad.len())
                    },
                );
            }
        }
        if counts.is_empty() {
            counts = vec![0; checks.len()];
            names = checks.iter().map(|(n, _, _)| *n).collect();
        }
        for (k, (_name, ok, msg)) in checks.iter().enumerate() {
            if !ok {
                counts[k] += 1;
                if shown < 20 {
                    println!("  frame {i}: {msg}");
                    shown += 1;
                }
            }
        }
    }
    let n = frames.len() - 1;
    println!("replay: {n} frames compared");
    print!("mismatches:");
    for (name, count) in names.iter().zip(counts.iter()) {
        print!(" {name}={count}");
    }
    println!();
    if !gap_ok {
        println!("note: the death-cycle gap check FAILED (fade length / tick accounting)");
    }
    if !obj.unported.is_empty() {
        println!(
            "note: unported bank-16 handler path(s) hit: {}",
            obj.unported
                .iter()
                .map(|a| format!("${a:05X}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

/// Parse a 32-hex-char dump into 16 bytes (a `sBlockAnimSlots` entry).
fn parse_slot16_hex(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Parse a 64-hex-char slot dump into 32 bytes.
fn parse_slot_hex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Interactive play mode: the ported player physics + camera drive the
/// viewport (arrows move, Z/Space = A, X/Shift = B, Q/ESC quits).
#[allow(clippy::too_many_arguments)]
fn run_play(
    manifest: &LevelManifest,
    layout: &LevelLayout,
    bg_base: &[u8],
    anim_rows: &[u8],
    hud: &HudStrip,
    hud_y: u8,
    scale: usize,
    view: (usize, usize),
    mut spr: Option<SpriteCtx>,
    mut sim: PlayerSim,
    mut cam: sml2_core::camera::CameraSim,
    mut obj: sml2_core::objects::ObjectSim,
    edge_map: Option<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (vw, vh) = view;
    if let Some(e) = edge_map.as_ref() {
        if e.len() == 48 {
            cam.edge_map.copy_from_slice(e);
        }
    }
    use minifb::{Key, Window, WindowOptions};
    let mut window = Window::new(
        &format!(
            "sml2 level {} — play (arrows move, Z jump, X run, Q quits)",
            manifest.level_id
        ),
        vw * scale,
        vh * scale,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| format!("cannot open a window ({e}) — no display found"))?;
    window.set_target_fps(60);
    let mut buf = vec![0u32; vw * scale * vh * scale];
    let mut anim_timer: u32 = 0;
    let mut anim_phase: u8 = 0;
    let mut prev_a = false;
    obj.configure(manifest.level_id);
    let obj_tables = spr.as_ref().map(|s| s.tables.clone());
    while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
        let a_down = window.is_key_down(Key::Z) || window.is_key_down(Key::Space);
        let keys = PlayerKeys {
            right: window.is_key_down(Key::Right),
            left: window.is_key_down(Key::Left),
            up: window.is_key_down(Key::Up),
            down: window.is_key_down(Key::Down),
            a: a_down,
            b: window.is_key_down(Key::X) || window.is_key_down(Key::LeftShift),
        };
        let a_pressed = a_down && !prev_a;
        prev_a = a_down;
        sim.camx = cam.x;
        sim.camy = cam.y;
        // $FF97: the engine's frame counter ticks once per game frame.
        sim.phase = (sim.phase + 1) & 7;
        // $05E4/$0640: dead → the arc only; the death fade freezes everything.
        let was_dead = sim.dead == 0xFF;
        // The fade-trigger frame still runs the object engine ($0640
        // continues into $0652); the true fade frames do not. `$FF9B`: 9 =
        // the death fade, 24 = the exit fade ($1CBA's bell arm) — both leave
        // the in-level loop, so everything freezes while either is set.
        // VBlank still ticks `sFadeTimer`; the exit fade's last step is
        // `$3547`'s BCD tally + the `$14` hand-off.
        let fade_at_entry = sim.dead_fade || sim.exit_phase;
        if sim.dead_fade || sim.exit_phase {
            if sim.exit_phase {
                sim.exit_fade_step();
            }
        } else if was_dead {
            sim.death_arc(cam.y);
        } else {
            if cam.transition == 0 {
                sim.step(&keys, a_pressed, layout);
            }
            // `$3451` id 4's `sStompChainCount` reset — see the replay loop.
            if sim.take_stomp_chain_reset() {
                obj.stomp_chain = 0;
            }
            // $2F72 — the bump alloc's tail; see the replay loop. Play mode
            // has no `sCameraPanTimer`, so the pan term is 0.
            if let Some((by, bx)) = sim.below_bonk.take() {
                let scroll_y = cam.y.wrapping_sub(72) as u8;
                let scroll_x = cam.x.wrapping_sub(80) as u8;
                let special = obj.below_bonk(by, bx, scroll_y, scroll_x);
                sim.resolve_bump_coin(special);
            }
            cam.step(&mut sim);
            cam.update_enables(sim.x);
        }
        if let Some(tables) = &obj_tables {
            if !fade_at_entry {
                let mario = sml2_core::objects::MarioContact {
                    x: sim.x,
                    y: sim.y,
                    dead: sim.dead,
                    powerup: sim.powerup,
                    ducking: keys.down && sim.on_ground,
                    spin_jump: sim.spin_jump != 0,
                    vvel: sim.vvel,
                    on_ground: sim.on_ground,
                    b_held: keys.b,
                    a_held: keys.a,
                    travelling: sim.pipe_dir >= 0x10,
                    ..Default::default()
                };
                obj.mario_dead = sim.dead;
                obj.step(cam.x, cam.y, layout, tables, &mario);
                if sim.dead == 0 && obj.mario_dead != 0 {
                    sim.dead = obj.mario_dead;
                }
                if obj.pending_item != 0 {
                    sim.pending_item = obj.pending_item;
                    obj.pending_item = 0;
                }
                if let Some(y) = obj.mario_y_adjust {
                    sim.y = y;
                }
                if obj.stomp_jump {
                    sim.in_air = true;
                    sim.vvel = if keys.a { 75 } else { 48 };
                }
                if obj.stomp_bounce {
                    sim.in_air = true;
                    sim.vvel = 49;
                }
            }
        }
        let f = if was_dead {
            107
        } else {
            sim.frame_index(sim.walk_timer)
        };
        // $2C82: tick the bump slots. The bump stages its own sprite at the
        // block's screen coords (`block_draw`), which the renderer draws as an
        // extra OAM sprite while the player sprite keeps Mario's frame.
        if !was_dead {
            sim.block_update();
        }
        if let Some(s) = spr.as_mut() {
            s.player = Some((sim.x, sim.y, f));
            // $2C82 is skipped while dead, and the dead path's own draw ($5550,
            // frame 107) overwrites the staging pair — so a bump sprite never
            // survives into the arc.
            if was_dead {
                s.bumpers.clear();
            } else {
                s.bumpers = sim.block_draw.clone();
            }
            // Draw ACTIVE objects: positions/flip from the engine's staging.
            s.insts = obj
                .slots
                .iter()
                .filter(|o| o.state() == sml2_core::objects::SlotState::Active)
                .map(|o| SpriteInst {
                    sprite: o.sprite(),
                    x: o.x(),
                    y: o.y(),
                    flip: o.b[sml2_core::objects::B_DEFEAT]
                        ^ o.b[sml2_core::objects::B_FLIP]
                        ^ o.b[sml2_core::objects::B_FLASH],
                })
                .collect();
        }
        anim_timer += 1;
        if manifest.tile_anim.enabled && anim_timer >= manifest.tile_anim.speed as u32 * 2 {
            anim_timer = 0;
            anim_phase = (anim_phase + 1) & 3;
        }
        let mut bg = bg_base.to_vec();
        apply_tile_anim(&mut bg, &manifest.tile_anim, anim_rows, anim_phase);
        // $076B: the status bar's coin digits follow the sim (the picked-up
        // coin shows up immediately, like the console's per-frame patch).
        let hud_live = hud_row_coins(hud, sim.coins_high, sim.coins_low);
        let frame = render_view(
            manifest,
            layout,
            &sim.layout_writes,
            &bg,
            cam.x,
            cam.y,
            vw,
            vh,
            Some((&hud_live, hud_y)),
            spr.as_ref(),
        )?;
        // Integer-scale + letterbox into the current window size.
        let (win_w, win_h) = window.get_size();
        if win_w == 0 || win_h == 0 {
            continue;
        }
        if buf.len() != win_w * win_h {
            buf = vec![0; win_w * win_h];
        } else {
            buf.fill(0);
        }
        let s = (win_w / vw).min(win_h / vh).max(1);
        let ox = win_w.saturating_sub(vw * s) / 2;
        let oy = win_h.saturating_sub(vh * s) / 2;
        for y in 0..vh {
            let py0 = oy + y * s;
            if py0 + s > win_h {
                break;
            }
            for x in 0..vw {
                let px0 = ox + x * s;
                if px0 + s > win_w {
                    break;
                }
                let g = frame[y * vw + x] as u32;
                let px = g | (g << 8) | (g << 16);
                for dy in 0..s {
                    let row = (py0 + dy) * win_w + px0;
                    buf[row..row + s].fill(px);
                }
            }
        }
        window
            .update_with_buffer(&buf, win_w, win_h)
            .map_err(|e| e.to_string())?;
    }
    println!(
        "play: ended at mario ({}, {}) cam ({}, {})",
        sim.x, sim.y, cam.x, cam.y
    );
    Ok(())
}
