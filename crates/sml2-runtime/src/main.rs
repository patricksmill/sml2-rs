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
//! `--level-progress BYTE` supplies the map entry progress byte in decimal;
//! bit 3 selects the ROM's alternate Mario/camera start (e.g. `8`).
//! Camera defaults to the resolved entry's camera start. Tiles resolve
//! through `sml2_core::video`, which keeps palette indices as data — the
//! seam a texture pack plugs into later (see sml2-rs/DESIGN.md).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod divider_clock;
#[cfg(test)]
mod door_boss_entry_tests;
#[cfg(test)]
mod patrol_boss_entry_tests;
#[cfg(test)]
mod tree_boss_entry_tests;
#[cfg(test)]
mod sweep_boss_entry_tests;
#[cfg(test)]
mod circuit_boss_entry_tests;
#[cfg(test)]
mod scene_pipe_tests;
use divider_clock::{NativeDividerClock, VirtualDividerClock};

use serde::Deserialize;
use sml2_core::ending_entry::{EndingEntry, EndingEntryState, EndingFrame, EndingInput};
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest, LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::map::MapNodes;
use sml2_core::object::ObjectTables;
use sml2_core::objects::FireHit;
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_core::scene::{boss_def_index, boss_def_portable, SceneScripts, SceneStep, SceneVm};
use sml2_core::tileset::Tilesets;
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
    let mut level_progress = 0u8;
    let mut headless_frames: Option<u64> = None;
    let mut keys_tape: Option<String> = None;
    let mut view = (SCREEN_W, SCREEN_H);
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            // A deterministic one-char-per-frame tape: `.` idle, R/L/U/D hold a
            // direction, A/B the buttons, S start, E select. The run stops when
            // the tape runs out and prints where it got to, so a play-test
            // report becomes reproducible instead of a guess.
            "--keys" => {
                keys_tape = Some(args.next().ok_or("--keys needs a tape")?);
            }
            "--level" => {
                level = Some(args.next().ok_or("--level needs a value")?.parse::<u8>()?);
            }
            // The map's `$A27D` entry byte (decimal), consumed by `$5579`.
            "--level-progress" => {
                level_progress = args.next().ok_or("--level-progress needs a byte")?.parse()?;
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
            // Deterministic offscreen play: run N frames of the real `run_play`
            // loop with no window and no rendering. Idle input unless a `--keys`
            // tape is given. Used by tools/smoke_levels.py to boot every level.
            "--headless" => {
                let n = args
                    .next()
                    .ok_or("--headless needs a frame count")?
                    .parse::<u64>()?;
                headless_frames = Some(n);
                play_mode = true;
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
    if level_progress & 8 != 0 && manifest.secret_entry.is_none() {
        return Err("secret entry missing: regenerate assets with sml2-extract".into());
    }
    let level_entry = manifest.header.entry(level_progress, manifest.secret_entry);
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
                player_shield: None,
                player_pan: 0,
                scroll_pan: 0,
                bumpers: Vec::new(),
                fireballs: Vec::new(),
                boss_oam: Vec::new(),
            })
        } else {
            None
        };
    let start = (
        camera
            .map(|c| c.0)
            .unwrap_or(level_entry.start.camera_x),
        camera
            .map(|c| c.1)
            .unwrap_or(level_entry.start.camera_y),
    );
    // The level's 48-byte `sScreenEdgeMap` (`$03F1`). Both modes need it: the
    // replay harness derives `$FF98` from it and checks the captured byte, and
    // play mode has no capture to fall back on — without the table its camera
    // runs with every edge bit clear, so the vertical follow takes the
    // no-hard-edge branches and drifts downward over a jump.
    let edge_map = fs::read(levels_dir.join(format!("level{nn:02}.edge.bin"))).ok();
    // The parallel 48-byte `sSceneMap` (`$03F1`, from the level header's
    // `[$4002]`), which `$0F0B` reads to latch `sSceneId` from the cell under
    // Mario. Unlike the edge map there is no capture field to fall back on —
    // no capture records scene data — so this is the only source; a missing
    // file leaves `scene_id` 0, which is what level 1's all-zero map gives
    // anyway.
    let scene_map = fs::read(levels_dir.join(format!("level{nn:02}.scene.bin"))).ok();
    if let Some(rf) = replay_file {
        return run_replay(
            &rf,
            level_entry,
            &manifest,
            &layout,
            &bg,
            &anim_rows,
            &hud,
            hud.window_y,
            spr,
            filmstrip.as_deref(),
            edge_map,
            scene_map,
        );
    }
    if play_mode {
        let mut sim = PlayerSim::new();
        seed_level_timer(&mut sim, &manifest);
        sim.powerup = 0;
        let mut cam = sml2_core::camera::CameraSim::new(
            level_entry.start.camera_x,
            level_entry.start.camera_y,
        );
        apply_level_entry(&mut sim, &mut cam, level_entry);
        cam.prev_mario_y_lo = sim.y as u8;
        // The two per-level property bytes the ROM loads at $04F1: sMapScrollLimit
        // from `LevelPropertiesUNK` and sAutoScroll from `LevelPropertiesScroll`
        // (home.asm:741-747 / 633-639). Both are already in the manifest, and the
        // replay path takes them from its capture's frame 0 — play mode had no
        // source at all and ran on the defaults, the same shape of gap the edge
        // map had. Level 1 is $00 in both, so its play and replay agree.
        cam.map_scroll_limit = manifest.properties.unknown != 0;
        cam.auto_scroll = manifest.properties.auto_scroll;
        // `$04DD`/`home.asm:661`: the THIRD property byte — `LevelPropertiesSpace`
        // (`$1F91`) into `sMoonPhysics` ($A287) — the same class of gap as the
        // two above, and level 1 is `$00` in all three so the gate is
        // structurally blind to it. Seeded through the sim's own method so play
        // and replay share one construction.
        sim.seed_moon_physics(manifest.properties.space_physics);
        // `$0609`: sSceneState is initialized from the level header's +18 high
        // nibble (ram.asm:665). All 32 levels carry 0 there, so nothing starts
        // mid-scene — but the byte is a real input: a nonzero high nibble makes
        // `$0B2A` park the camera and the frame loop run the alternate driver.
        cam.scene_state = manifest.header.scene_state;
        // `--mario`/`--camera` place Mario and the viewport directly. Without
        // them a spot deep in a long level is only reachable by playing there,
        // and a scripted "hold right" tape just walks into the level's first
        // step — so a spot-specific report could not be reproduced at all.
        if let Some((mx, my, _)) = mario {
            sim.x = mx;
            sim.y = my;
            cam.prev_mario_y_lo = my as u8;
        }
        if let Some((cx, cy)) = camera {
            cam.x = cx;
            cam.y = cy;
        }
        if let Some(m) = scene_map.as_ref() {
            if m.len() == 48 {
                cam.scene_map.clone_from(m);
            }
        }
        // The autoscroll path is no longer capture-less: traj19 (level 20) is in
        // the gate now — see the retired-flag note in `run_replay`.
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
            edge_map,
            scene_map,
            keys_tape.clone(),
            headless_frames,
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
            None,
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

/// The overworld's node tables (`extracted/map/nodes.bin`). One ROM-wide blob,
/// loaded like the scene scripts, and flagged when absent for the same reason:
/// with no table the hand-off cannot index `sLevelProgress`, so it would silently
/// stop marking progress.
fn load_map_nodes() -> MapNodes {
    match fs::read(extracted_dir().join("map/nodes.bin"))
        .ok()
        .as_deref()
        .and_then(MapNodes::from_bytes)
    {
        Some(n) => n,
        None => {
            println!("note: map node tables missing — the hand-off cannot mark progress");
            MapNodes::default()
        }
    }
}

/// The bank-5 scene script table (`extracted/scenes/script_table.bin`). It is
/// one ROM-wide blob rather than per level, and the missing-file flag matches
/// the two level maps: with no table the scene VM is inert, and the run says so
/// instead of quietly never running a script.
fn load_scene_scripts() -> SceneScripts {
    match fs::read(extracted_dir().join("scenes/script_table.bin"))
        .ok()
        .as_deref()
        .and_then(SceneScripts::from_bytes)
    {
        Some(s) => s,
        None => {
            println!("note: scene script table missing — no scene script can run");
            SceneScripts::default()
        }
    }
}

/// The `$6x` opcode's tileset swaps (`extracted/scenes/tilesets.json`). Also one
/// ROM-wide table — `$25AF`'s three sources are indexed only by
/// `sLevelGfxIndex`. Without it an arena keeps the level's own tiles and
/// metatile definitions, which draws the right *layout* with the wrong art.
fn load_tilesets() -> Tilesets {
    match fs::read_to_string(extracted_dir().join("scenes/tilesets.json"))
        .ok()
        .and_then(|t| Tilesets::from_json(&t).ok())
    {
        Some(t) => t,
        None => {
            println!("note: tileset table missing — a scene's $6x swap cannot run");
            Tilesets::default()
        }
    }
}

/// The `$6x` reload's level data (`extracted/scenes/arena_layout.bin`).
///
/// `$25AF` hardcodes `sLevelLayoutIndex = 3` and `sLevelBank = 19`, so the same
/// blob loads for **every** arena: the boss rooms are authored as screens of
/// that one layout and the per-arena look comes from the tileset swap. A level's
/// own layout at those cells holds that level's tiles, which is why an arena
/// rendered from it comes out as ordinary level walls.
fn load_arena_layout() -> Option<LevelLayout> {
    match fs::read(extracted_dir().join("scenes/arena_layout.bin")) {
        Ok(b) => match LevelLayout::from_bytes(&b) {
            Ok(l) => Some(l),
            Err(e) => {
                println!("note: scenes/arena_layout.bin unusable — {e}");
                None
            }
        },
        Err(_) => {
            println!(
                "note: scenes/arena_layout.bin missing — a $6x swap cannot reload the arena room"
            );
            None
        }
    }
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
    /// Optional 3-up moon shield ($54FB): world position + frame 220/221.
    player_shield: Option<(u16, u16, u8)>,
    /// Brick-bump sprites for this frame (`PlayerSim::block_draw`): the engine
    /// stages `$FFC4`/`$FFC5`/`$FFC6` and enqueues them (`$5297`), so they are
    /// extra OAM sprites drawn at the block — through the same player-sprite
    /// path, after Mario (his entries come first, so he wins overlaps).
    bumpers: Vec<(u16, u16, u8)>,
    /// The live fireballs' sprites (`$3261`'s `$2CF4` staging, mapping `$B0`/`$B4`
    /// + the rDiv flicker), `(x, y, mapping)` in the same convention as
    /// `bumpers`.
    fireballs: Vec<(u16, u16, u8)>,
    /// During a boss fight, the boss path's own draw output (`$3A14` -> bank 25
    /// `$66000`, [`sml2_core::objects::ObjectSim::boss_oam`]): the entries are
    /// already in OAM byte space, so they are blitted at `(x - 8, y - 16)` and
    /// their attr byte is used as-is (the pass XORed `sSpriteFlipFlags` in).
    /// Drawn after Mario, like the level objects — and `insts` is empty on
    /// these frames, because the boss slots' state byte carries HP, not a slot
    /// state, and the engine's own draw never runs in the boss path.
    boss_oam: Vec<sml2_core::objects::BossOamCell>,
    /// Distinct phases: Mario subtracts the decremented phase; BG/objects use latched scroll.
    player_pan: u8,
    scroll_pan: u8,
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
            None,
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

/// The BG is a 32x32-tile tilemap = 256 px. The engine loads it with the level's rows and
/// columns around the camera — clamped into the level, because the window has to hold real
/// level rows — and never clamps the camera or the view.
fn window_origin(view: i32, world: i32) -> i32 {
    view.clamp(0, world - 256)
}

/// A screen row/column seen through that window: itself inside the level, the window's own
/// rows again outside it. A 256-px window is a whole number of 16-px metatiles and of 8-px
/// tiles, so the wrap keeps both grids and the fine scroll intact.
fn window_coord(v: i32, origin: i32) -> i32 {
    origin + (v - origin).rem_euclid(256)
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
    // `$2756`'s pause flash: the byte the ROM writes to `sBGPalette` (and to
    // `sOAMPalette1`, which the caller applies to `spr.obp0`) while paused,
    // standing in for the level's BG palette. `None` on every other frame —
    // which is every frame any replay sees, so the gate is unaffected.
    pal_override: Option<u8>,
) -> Result<Vec<u8>, String> {
    // Decode every tile in the composed block once.
    let tiles: Vec<[u8; 64]> = (0..256u16)
        .map(|t| {
            let off = video::bg_tile_offset(t as u8);
            let bytes: [u8; 16] = bg[off..off + 16].try_into().expect("block is $1000 bytes");
            video::tile_colors(&bytes)
        })
        .collect();
    let shades = video::bgp_shades(pal_override.unwrap_or(manifest.header.palettes[0]));
    let world_w = (LAYOUT_WIDTH * 16) as i32;
    let world_h = (LAYOUT_HEIGHT * 16) as i32;
    // The engine never clamps the view to the level: it leaves the camera where it is and
    // slides a 256-px tilemap window under it, so with the camera near an edge the view
    // runs past the level and shows the window's rows again. Clamping the view here (which
    // this used to do) instead shifted the whole background against Mario's OAM — on level
    // 5, whose start camera sits at the bottom of a 768-px world (cam_y 704, so the view
    // top 632 is past 768 - 144), every tile drew 8 px low and Mario looked like he floated
    // above the block row the console puts him on.
    let sx = cam_x as i32 - vw as i32 / 2;
    let sy = cam_y as i32 - vh as i32 / 2 - spr.map_or(0, |s| s.scroll_pan as i32);
    let (ox, oy) = (window_origin(sx, world_w), window_origin(sy, world_h));
    let mut img = vec![shades[0]; vw * vh];
    let mut bg_idx = vec![0u8; vw * vh];
    for py in 0..vh {
        let wy = window_coord(sy + py as i32, oy);
        let (cy, ty) = ((wy as usize) >> 4, (wy as usize) & 15);
        for px in 0..vw {
            let wx = window_coord(sx + px as i32, ox);
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
                    c.y.wrapping_sub(s.player_pan) as i32 - 16,
                    c.tile,
                    c.attr,
                    vw,
                    vh,
                );
            }
        }
        if let Some((sx, sy, sframe)) = s.player_shield {
            let cells =
                sml2_core::player::draw_cells(&s.tables, sframe, sx, sy, cam_x, cam_y, false)
                    .ok_or_else(|| format!("player shield frame {sframe} has no mapping"))?;
            for c in cells {
                blit_obj_cell(
                    &mut img,
                    &bg_idx,
                    &mut taken,
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8,
                    c.y.wrapping_sub(s.player_pan) as i32 - 16,
                    c.tile,
                    c.attr,
                    vw,
                    vh,
                );
            }
        }
        // The live fireballs ($3261's $2CF4 staging, mapping $B0/$B4 + the rDiv
        // flicker), drawn with the bumpers. `$3261` stages them inside `$0F71`,
        // i.e. before the player's own draw call, so their OAM index — and thus
        // the overlap priority against Mario for the frame or two a fresh ball
        // sits inside him — may want to precede his cells; not verified against
        // a console frame yet, and it can only show while they overlap.
        let fireballs = s.fireballs.clone();
        for (fx, fy, ff) in fireballs {
            let cells = sml2_core::player::draw_cells(&s.tables, ff, fx, fy, cam_x, cam_y, false)
                .ok_or_else(|| format!("fireball mapping {ff} has no entry"))?;
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
        // The boss path's own draw (`$3A14` -> bank 25 `$66000`), which replaces
        // the level's `$C000` pass while the fight runs: those entries are
        // already OAM bytes, so the screen pixel is `(x - 8, y - 16)` and the
        // attr byte stands (the pass XORed the flip flags in itself). Drawn last
        // so the fight's parts layer over the level's objects, and after Mario,
        // who keeps OAM's first entries.
        for c in &s.boss_oam {
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
    /// `sMoonPhysics` ($A287) — the level's `LevelPropertiesSpace` byte. The sim
    /// derives it from the manifest (`PlayerSim::seed_moon_physics`), so this is
    /// a CHECK of the derivation, not a source for it (traj19/level 20 → $01).
    #[serde(default)]
    moon: Option<u8>,
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
    /// The `$1CBA` exit-tile arm's observables (`captures/traj17.json`):
    /// `sExitTileFlag` ($A2B4), `sLowTimeFlag` ($A53F), the exit fade's
    /// `sFadeTimerLow`/`High` ($A266/$A267) and `sLevelTally` ($A2C5, BCD).
    /// All optional, so every capture recorded before traj17 still replays.
    #[serde(default)]
    exit: Option<u8>,
    #[serde(default)]
    lowtime: Option<u8>,
    #[serde(default)]
    fadelo: Option<u8>,
    #[serde(default)]
    fadehi: Option<u8>,
    #[serde(default)]
    tally: Option<u8>,
    /// The fade/map/reload gap carried by the first post-respawn record of a
    /// full-death-cycle capture.
    #[serde(default)]
    gap: Option<GapJ>,
    /// The exit-tile arm's writes, carried on the arm's own record
    /// (`captures/traj17.json`) — checked once, outside the per-frame table.
    #[serde(default)]
    exit_arm: Option<ExitArmJ>,
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

/// The `$1CBA` exit-tile arm's facts that a per-frame table cannot express,
/// carried on the arm's own record by `captures/trajectory_capture17.py`: the
/// frame it fired on, the tile the probe read, and the fade span measured from
/// outside the in-level hook (`$05E4` stops firing once `$FF9B` leaves 4, so
/// the capture splices the frames after it in from a `$028D` hook).
#[derive(Deserialize)]
struct ExitArmJ {
    /// The in-level record the arm fired on — the LAST `$05E4` record; every
    /// record after it came from the `$028D` splice.
    frame: usize,
    #[allow(dead_code)]
    tick: u64,
    x: u16,
    y: u16,
    /// `sCollisionTile` at the arm (the `$4A` the probe read).
    col: u8,
    music: u8,
    #[allow(dead_code)]
    exit: u8,
    #[allow(dead_code)]
    lowtime: u8,
    tally_before: u8,
    tally_after: u8,
    #[allow(dead_code)]
    fade_first: u16,
    #[allow(dead_code)]
    fade_last: u16,
    /// `[[$FF9B, ticks], ...]` polled per tick from the arm onwards.
    state_seq: Vec<[u64; 2]>,
}

/// The `MarioContact` both loops hand to the object engine (`$2D7D`'s view of
/// Mario). One constructor, because each loop used to build it by hand and the
/// play loop quietly passed defaults for `star_high`, `invincibility` and
/// `on_object` — a divergence no capture can show, since only `--play` had it
/// (the class queued as `infra_play_parity_harness`).
///
/// `pan` is `sCameraPanTimer` (`$A2B3`, the `$2062` scroll-Y nudge): the replay
/// path takes it from its capture, play mode has no source for it yet and
/// passes 0. `scene_state` and `flutter` are 0 in both paths for the same
/// reason — nothing in the port drives `sSceneState` or `sFlutterTimer` yet.
fn build_contact(sim: &PlayerSim, keys: &PlayerKeys, pan: u8) -> sml2_core::objects::MarioContact {
    sml2_core::objects::MarioContact {
        x: sim.x,
        y: sim.y,
        dead: sim.dead,
        powerup: sim.powerup,
        ducking: keys.down && sim.on_ground,
        dir_right: sim.dir_right,
        spin_jump: sim.spin_jump != 0,
        star_high: sim.star_high,
        invincibility: sim.invincibility,
        flutter: sim.flutter,

        vvel: sim.vvel,
        on_ground: sim.on_ground,
        on_object: sim.on_ground_obj,
        b_held: keys.b,
        a_held: keys.a,
        scene_state: 0,
        moon_physics: sim.moon,
        pan,
        travelling: sim.pipe_dir >= 0x10,
        hvel: sim.hvel,
    }
}

/// Replay a captured trajectory through `sml2_core::player_sim` and report
/// frame-by-frame divergences from the live console state. With `filmstrip`,
/// also renders every 30th frame (sim + camera + player draw) as a 2x PNG.
#[allow(clippy::too_many_arguments)]
fn run_replay(
    path: &Path,
    level_entry: sml2_core::level::LevelEntry,
    manifest: &LevelManifest,
    layout: &LevelLayout,
    bg_base: &[u8],
    anim_rows: &[u8],
    hud: &HudStrip,
    hud_y: u8,
    mut spr: Option<SpriteCtx>,
    filmstrip: Option<&Path>,
    edge_map: Option<Vec<u8>>,
    scene_map: Option<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let frames: Vec<Frame> = serde_json::from_str(&fs::read_to_string(path)?)?;
    if frames.len() < 2 {
        return Err("replay: need at least 2 frames".into());
    }
    // The level's 48-byte sScreenEdgeMap: with it the port derives `$FF98`
    // itself and the capture's byte becomes a check. Without it the captured
    // byte is injected as before (and the run says so, so the gate cannot
    // quietly pass with the port's own edge map never exercised).
    let own_edges = matches!(edge_map.as_ref(), Some(e) if e.len() == 48);
    if !own_edges {
        println!("note: level edge table missing — replaying with the captured $FF98");
    }
    // Same rule for `sSceneMap`: nothing in a capture records scene data, so a
    // missing file means `scene_id` stays 0 and no transition can ever start a
    // scene. The extractor writes one file per level, so this never fires in
    // the gate — it is the flag for the case where it should have.
    if !matches!(scene_map.as_ref(), Some(m) if m.len() == 48) {
        println!("note: level scene map missing — sSceneId will stay 0");
    }
    // The scene scripts (bank 5 `$4125`): one ROM-wide blob, so it is loaded
    // here rather than threaded through `main`. Level 1's scene map is all
    // zeros, so its `sSceneId` never latches and the VM never runs a byte.
    let scripts = load_scene_scripts();
    let mut scene_vm = SceneVm::new();
    let mut ending_entry = EndingEntry::default();
    let mut ending_sequence = sml2_core::ending_sequence::EndingSequenceController::new();
    let mut ending_scene = sml2_core::ending_scene::EndingScene::default();
    let nodes = load_map_nodes();
    let mut sim = PlayerSim::new();
    sim.powerup = 0; // level-0 spawn starts small
    sim.secret_route = level_entry.secret_route;
    sim.level_progress_flags = level_entry.progress_flags;
    sim.level_progress_flags = level_entry.progress_flags;
    seed_level_timer(&mut sim, manifest);
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
    // `$04DD`/`home.asm:661`: `sMoonPhysics` is the level's own property byte,
    // so the sim DERIVES it from the manifest — never from the record (the
    // edge-map rule: an injected value leaves the derivation untested in play).
    // Captures that carry `moon` make it a CHECK instead: traj19 (level 20)
    // records `$01` on all 1309 frames, which is `LevelPropertiesSpace[19]`.
    sim.seed_moon_physics(manifest.properties.space_physics);
    let mut cam = sml2_core::camera::CameraSim::new(
        f0.camx as u16 | ((f0.camxh as u16 & 0x0F) << 8),
        f0.camy as u16 | ((f0.camyh as u16 & 0x0F) << 8),
    );
    cam.pan_timer = f0.pan.unwrap_or(0); // documented initial snapshot only
    cam.update_scroll();
    cam.prev_mario_y_lo = f0.y as u8;
    cam.map_scroll_limit = f0.limit;
    cam.auto_scroll = f0.autoscroll;
    if let Some(e) = edge_map.as_ref() {
        cam.edge_map.copy_from_slice(e);
    }
    // `$0609`: the header's +18 high nibble is the initial `sSceneState`, and
    // the 48-byte scene map is what `$0F0B` latches `sSceneId` from. Level 1's
    // map is all zeros, so this cannot move the gate — it is wired for the
    // scene levels, where a transition under a nonzero cell starts a scene.
    cam.scene_state = manifest.header.scene_state;
    if let Some(m) = scene_map.as_ref() {
        if m.len() == 48 {
            cam.scene_map.clone_from(m);
        }
    }
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
    // $0DAF's capture-less flag is RETIRED (2026-10-14): traj19 replays level 20
    // (`--level 19`, listed in `captures/sweep.sh`) with `camx`/`camy`/`en_r`/
    // `en_l`/`edge` all-zero across 1308 compared frames, so the autoscroll path
    // is gate-verified now. The flag's job was to fail a gate that added such a
    // capture; traj19 is that capture, so it is the check that replaces it.
    if let Some(dir) = filmstrip {
        fs::create_dir_all(dir)?;
    }
    let mut counts: Vec<usize> = Vec::new();
    let mut names: Vec<&'static str> = Vec::new();
    let mut shown = 0usize;
    let mut gap_ok = true;
    let mut prev_a = f0.a;
    let mut prev_b = f0.b;
    // The first snapshot has explicit virtual DIV phase zero; the corpus has
    // no initial counter or within-frame sample timing evidence.
    let mut divider_clock = VirtualDividerClock::new(0);
    for (i, f) in frames.iter().enumerate().skip(1) {
        divider_clock.advance_tick();
        let keys = PlayerKeys {
            right: f.right,
            left: f.left,
            up: f.up,
            down: f.down,
            a: f.a,
            b: f.b,
            // The replay `Frame` carries no START/SELECT: no capture records
            // them, and the pause state is only reachable interactively.
            start: false,
            select: false,
        };
        let a_pressed = f.a && !prev_a;
        prev_a = f.a;
        // `hKeysPressed` bit 1 — the fire button is B, edge-triggered, so
        // holding it fires once.
        let b_pressed = f.b && !prev_b;
        prev_b = f.b;
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
        // `$FF97`'s bits above 2 are not in the capture record, so the full
        // counter is a best-effort free run here (only `$532D`'s spin-jump pose
        // reads bit 3, and no capture spins). The *low* bits still come from the
        // record above, which is what every other reader uses.
        sim.frame_counter = sim.frame_counter.wrapping_add(1);
        // The camera bytes the *player* update reads are the ones written by
        // the previous frame's camera step: the ROM runs `$0F71` before
        // `$0B2A` (home.asm:799/803), so `$0FBF`'s clamp and `$1BFF`'s hazard
        // `sMarioScreenY` both see the camera as of the frame's START. Feed
        // them from the port's own camera object — exactly what `run_play`
        // does — rather than from `f.camx`/`f.camy`, which are the record's
        // POST-frame camera. Injecting the post-frame byte put the clamp one
        // frame ahead of the console and made the sim push Mario +1 early on
        // every frame the autoscroll camera stepped (traj19,
        // `autoscroll_clamp_x_divergence`). `cam.x`/`cam.y` here already equal
        // the record's camera for the PREVIOUS frame — the camx/camy checks
        // below still verify the derivation against the record.
        sim.camx = cam.x;
        sim.camy = cam.y;
        sim.pan_timer = cam.pan_timer;
        // $05E4/$0640: the frame loop's three branches. Dead (`$FF`) runs only
        // the arc + the object engine (the camera freezes); the death fade
        // (`$FF9B = 9`) leaves the level entirely, freezing everything.
        let ending_frame = scene_frame_head(
            &mut scene_vm,
            &mut ending_entry,
            &mut sim,
            &mut cam,
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    ..PlayerKeys::default()
                },
            },
        );
        let gameplay = matches!(ending_frame, Some(EndingFrame::Gameplay(_)));
        let input = match ending_frame {
            Some(EndingFrame::Gameplay(input)) => input,
            _ => EndingInput::default(),
        };
        let keys = input.held;
        let (a_pressed, b_pressed) = (input.pressed.a, input.pressed.b);
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
        //
        // $FF98 is read from the level's edge table at the camera's *current*
        // cell, before the camera moves ($0B39) — and the console's sampled
        // byte still holds that pre-move value on the frame a crossing
        // happens, so it is read here and checked after the step. Recomputing
        // it on a dead/fade frame is right: the camera has not moved, so the
        // cell (and the console's stale $FF98) is unchanged.
        let cam_edge_before = if own_edges { cam.edge_byte() } else { f.edge };
        let fade_at_entry = !gameplay || sim.dead_fade || sim.exit_phase;
        // `$FF9B = $14` is set by the exit fade's *last* frame, so the hand-off
        // body runs on the frame after it — keyed on the state at entry.
        let exit_done_at_entry = sim.exit_completed;
        if !gameplay {
            // The shared frame head dispatched this ending-entry frame.
            if let Some(EndingFrame::SequencePending) = ending_frame {
                let res =
                    ending_sequence.step(&mut sml2_core::ending_sequence::EndingStepContext {
                        scene: &mut ending_scene,
                        rom: None,
                        credits_text: None,
                        object_tables: Some(&tables),
                        decrement_fade: true,
                    });
                if res.sfx_request_0 != 0 {
                    sim.sfx0 = res.sfx_request_0;
                }
                if res.sfx_request_1 != 0 {
                    sim.sfx1 = res.sfx_request_1;
                }
            }
        } else if sim.dead_fade || sim.exit_phase {
            if sim.exit_phase {
                sim.exit_fade_step();
            }
            if exit_done_at_entry {
                let handoff = sim.map_handoff(&nodes);
                note_handoff_screen(&handoff);
            }
        } else if was_dead {
            sim.death_arc(cam.y);
        } else {
            // a screen transition skips the player update — the camera's
            // `$0DED` handler moves Mario instead.
            if cam.transition == 0 {
                // $307D — `$0F71`'s FIRST call: the B+Down/B+Up crouch-look
                // pan runs at the *top* of the player update, before the body,
                // so its `sMarioOnGround`/`sMarioOnObject` gate reads the
                // flags the previous frame settled (`PlayerSim::step` would
                // overwrite them). Still before `cam.step` below, and skipped
                // whenever `$0F71` is — see `CameraSim::crouch_pan`.
                cam.crouch_pan(&sim, &keys);
                sim.step(&keys, a_pressed, b_pressed, layout);
            } else {
                // home.asm ~115: VBlank decrements sFadeTimer unconditionally
                // on every in-level frame, including during camera transitions.
                sim.tick_fade_timer();
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
            // scroll_y/scroll_x latched by $2062 at the end of the previous frame.
            // The object engine ($8000) reads the below-hit flag later this frame,
            // so the pass has to land before `obj.step`; `resolve_bump_coin` is the
            // `$19D2` coin half of the split.
            if let Some((by, bx)) = sim.below_bonk.take() {
                let scroll_y = cam.scroll_y;
                let scroll_x = cam.scroll_x;
                let special = obj.below_bonk(by, bx, scroll_y, scroll_x);
                sim.resolve_bump_coin(special);
            }
            // A fireball's `$06` break reaches the same `$5960`/`$2F72` pair
            // from `$0F71`'s tail: the ball's own cell bytes arm the pass, and
            // only the player's `$19D2` bump owes the coin.
            if let Some((by, bx)) = sim.fire_below_bonk.take() {
                let scroll_y = cam.scroll_y;
                let scroll_x = cam.scroll_x;
                obj.below_bonk(by, bx, scroll_y, scroll_x);
            }
            // `$3261`'s `$2FED`: the ball's staged screen pair against the
            // object slots, before the object engine reads the flags it sets.
            for i in 0..sim.fire_scan.len() {
                let Some((sy, sx)) = sim.fire_scan[i].take() else {
                    continue;
                };
                let hit = obj.fire_hit(sy, sx);
                if hit == FireHit::Die {
                    sim.sfx0 = 2; // `$3007`'s sfx arm
                }
                sim.resolve_fire_hit(i, hit != FireHit::None);
            }
            cam.edge_now = if own_edges { None } else { Some(f.edge) };
            cam.step(&mut sim);
            cam.update_enables(sim.x);
        }
        // $FF98 for this frame. `own_edges` makes it the port's own derivation
        // from the level's 48-byte table (checked below); otherwise the
        // captured byte is what the camera ran with, so the check is a
        // tautology — the missing-table note above is what fails the gate.
        let edge_derived = if own_edges { cam_edge_before } else { f.edge };
        cam.update_player_pan(sim.y, was_dead, !fade_at_entry);
        let mario = build_contact(&sim, &keys, cam.scroll_pan);
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
            // then writes it back.
            sample_level_player_inputs(&mut obj, &sim);
            obj.div = divider_clock.sample();
            // `$0EB0`'s scene branch: taken only for a def whose handlers are
            // all ported (the fight then ends in the level clear, which reseeds
            // `sSceneState` and so clears the park); every other def is
            // reported, because parking with nothing to end the fight would
            // freeze the camera for good.
            if cam.scene_load_pending {
                let def_index = boss_def_index(manifest.level_id, cam.boss_fight_phase);
                if boss_def_portable(def_index) {
                    if let Some((music, level_song)) = cam.begin_scene_load(manifest.level_id) {
                        // `$0ECA`: the boss song goes to BOTH bytes.
                        sim.music_request = music;
                        sim.level_song = level_song;
                        obj.spawn_boss(def_index);
                    }
                } else if cam.take_blocked_scene_load() {
                    obj.note_blocked_scene_load();
                }
            }
            obj.step(
                cam.x,
                cam.y,
                layout,
                &tables,
                &mario,
                cam.boss_mode(), // `$0652`
            );
            // The write-back list lives in the sim so this loop and the play
            // loop cannot drift apart again (`infra_play_parity_harness`).
            sim.apply_object_result(&mut obj, keys.a);
            cam.apply_object_result(&mut obj);
        }
        // $20A4 owns the CPU across its HALT waits; the shared frame head
        // parks gameplay until its END returns to the main loop.
        if gameplay || ending_frame.is_none() {
            scene_vm.step(&mut sim, &mut cam, &scripts);
        }
        // The full-death-cycle gap: after the fade-trigger frame the game
        // leaves the level (fade 192 frames → `$14` lives/save/map setup →
        // the overworld map, which waits for input → re-entry reloads the
        // level). Only the fade and the reload's final state are modelled;
        // the input-dependent map is skipped over the captured tick gap.
        //
        // The map itself IS modelled now — `sml2_core::map_sim` is state `$0C`
        // (its tick, the cursor's walk graph, the gate and the entry), driven
        // by its own unit tests — but this harness cannot drive it: the replay
        // is an in-level ($05E4) record, and a map frame has no in-level state
        // to compare against. So a state-`$0C` stretch in `g.seq` stays inside
        // `g.total` here and is *accounted for*, not simulated.
        if let Some(g) = &f.gap {
            // The trigger record already counted one tick; the gap includes
            // its entire fade/map duration, not just the unmodelled map tail.
            divider_clock.advance_ticks(g.total.saturating_sub(1));
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
            if sim.game_over {
                // $314B → $05: the borrow means the game overruns to the GAME
                // OVER screen instead of reloading the level. No capture
                // reaches zero lives, so this is the modelled-only branch:
                // $27AF's state writes, then $07's fade + START poll (which
                // the caller drives via `game_over_step`).
                sim.game_over_enter();
                // `$27AF` clears `sMoneyBagCount` along with the coins, and that
                // byte lives on the object engine, so the caller clears it here.
                obj.money_bag = 0;
            } else {
                // $0F2A + $04F1 + $EBBB: the reload puts Mario back at the
                // header spawn and clears the object scratch (fff0 persists).
                sim.respawn(level_entry.start.mario_x, level_entry.start.mario_y);
                seed_level_timer(&mut sim, manifest);
                apply_level_entry(&mut sim, &mut cam, level_entry);
                obj.reset_level_start();
            }
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
                    s.player_pan = cam.player_pan;
                    s.scroll_pan = cam.scroll_pan;
                    // The dead path's own draw overwrites the staging pair.
                    s.bumpers = if was_dead {
                        Vec::new()
                    } else {
                        sim.block_draw.clone()
                    };
                    s.fireballs = sim.fire_draw.clone();
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
                    None,
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
                "edge",
                edge_derived == f.edge,
                format!("edge sim=${:02X} exp=${:02X}", edge_derived, f.edge),
            );
            if let Some(pan) = f.pan {
                push("pan", cam.pan_timer == pan,
                     format!("pan sim=${:02X} exp=${:02X}", cam.pan_timer, pan));
            }
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
                obj.stream_ptr() == f.sptr,
                format!(
                    "sptr sim=${:04X} exp=${:04X}",
                    obj.stream_ptr(),
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
            // The exit-tile arm's own state (only when the capture carries
            // it, so the older captures keep their check count).
            if let Some(v) = f.exit {
                push(
                    "exit",
                    sim.exit_tile_flag == v,
                    format!("exit sim=${:02X} exp=${:02X}", sim.exit_tile_flag, v),
                );
            }
            if let Some(v) = f.lowtime {
                push(
                    "lowtime",
                    sim.low_time_flag == v,
                    format!("lowtime sim=${:02X} exp=${:02X}", sim.low_time_flag, v),
                );
            }
            if let (Some(lo), Some(hi)) = (f.fadelo, f.fadehi) {
                let exp = ((hi as u16) << 8) | lo as u16;
                push(
                    "fade",
                    sim.fade_timer == exp,
                    format!("fade sim={} exp={}", sim.fade_timer, exp),
                );
            }
            if let Some(v) = f.tally {
                push(
                    "tally",
                    sim.level_tally == v,
                    format!("tally sim=${:02X} exp=${:02X}", sim.level_tally, v),
                );
            }
            push("objslots", slot_ok, slot_msg);
            push(
                "odd",
                obj.odd == (f.odd != 0),
                format!("odd sim={} exp={}", obj.odd, f.odd),
            );
            // `sMoonPhysics` ($A287) — the level's own property byte, DERIVED
            // from the manifest. traj19 (level 20, `space_physics` 1) is the one
            // capture that can see it: without the seed this reads $00 against
            // the record's $01 on every frame.
            if let Some(v) = f.moon {
                push(
                    "moon",
                    sim.moon == v,
                    format!("moon sim=${:02X} exp=${:02X}", sim.moon, v),
                );
            }
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
    // The exit-tile arm (`$1CBA`): the one-shot facts the per-frame table
    // cannot express (`captures/traj17.json`). `$05E4` is the state-$04 body's
    // entry, so it stops firing the moment `$FF9B` leaves 4 — the capture keeps
    // that hook for the whole in-level run and splices the frames AFTER the arm
    // in from a second hook at `$028D` (one fire per frame in *every* state), so
    // the fade, its `$3547` tally and the exit flag go through the ordinary
    // per-frame checks. What is left for here: the trigger's own geometry, the
    // arm record having that tail, the fade's DURATION (a per-frame table can't
    // count) and the hand-off state the fade's last frame moves to.
    let mut exit_arm_ok = true;
    if let Some(arm) = frames.iter().find_map(|f| f.exit_arm.as_ref()) {
        let mut probs: Vec<String> = Vec::new();
        if arm.frame + 1 > frames.len() - 1 {
            probs.push(format!(
                "arm frame {} has no tail records ({} total)",
                arm.frame,
                frames.len()
            ));
        }
        if !sim.exit_phase {
            probs.push("the port's exit arm did not fire".to_string());
        }
        // The poll starts on the tick AFTER the arm frame — the arm frame's own
        // start is still state 4 (`$05E4` ran and produced the last record),
        // and the switch to 24 happens inside it — so the measured span is the
        // frames after it and the arm frame itself makes up the rest.
        let cap_fade_raw = arm.state_seq.first().map(|s| s[1]).unwrap_or(0);
        let cap_fade = cap_fade_raw + 1;
        if arm.state_seq.first().map(|s| s[0]) != Some(24) {
            probs.push(format!(
                "state_seq does not start in 24: {:?}",
                arm.state_seq
            ));
        }
        if arm.state_seq.last().map(|s| s[0]) != Some(20) {
            probs.push(format!(
                "state_seq does not end in 20 ($14, the hand-off): {:?}",
                arm.state_seq
            ));
        }
        let modeled_fade = 192u64;
        if cap_fade != modeled_fade {
            probs.push(format!(
                "fade length captured={cap_fade} modeled={modeled_fade}"
            ));
        }
        // The tail records stop at the fade's LAST decrement, and the frame that
        // `$3547`'s tail charges the tally on never reaches `$028D` (its
        // gfx/save work spans the whole tick), so run the port's remaining fade
        // steps here and check the charge they produce against the capture's.
        let mut tail_steps = 0u64;
        while sim.fade_timer > 0 {
            sim.exit_fade_step();
            tail_steps += 1;
        }
        if !sim.exit_completed {
            probs.push("the modelled fade never reached its tail ($3547)".to_string());
        }
        if sim.level_tally != arm.tally_after {
            probs.push(format!(
                "tally sim=${:02X} exp=${:02X}",
                sim.level_tally, arm.tally_after
            ));
        }
        println!(
            "  exit arm: record {} of {}, tick {}, x={} y={} col=${:02X}, music={}, \
             fade {cap_fade} captured vs {modeled_fade} modeled \
             ({cap_fade_raw} ticks after the arm frame + the arm frame), \
             tally {:02X}->{:02X}, state_seq {:?}, +{tail_steps} modelled to the tail{}",
            arm.frame,
            frames.len() - 1,
            arm.tick,
            arm.x,
            arm.y,
            arm.col,
            arm.music,
            arm.tally_before,
            arm.tally_after,
            arm.state_seq,
            if probs.is_empty() {
                String::new()
            } else {
                format!("  <- MISMATCH: {}", probs.join("; "))
            }
        );
        exit_arm_ok = probs.is_empty();
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
    if !exit_arm_ok {
        println!("note: the exit-tile arm check FAILED (see the `exit arm:` line above)");
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
    // A scene script opcode the port has no implementation for. The VM steps
    // over it with its real operand length, so the script still ends properly,
    // but the behaviour it was supposed to cause did not happen — and this line
    // is what fails the gate if a scene level is ever added to it.
    if !scene_vm.unported().is_empty() {
        println!(
            "note: scene script opcode(s) with no port implementation: {}",
            scene_vm
                .unported()
                .iter()
                .map(|o| format!("${o:02X}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // A latched scene whose script cursor was never found: the VM stepped
    // nothing at all. This line is what keeps a scene capture from replaying
    // all-zero while the scene never ran — the exact failure the missing
    // `SceneVm::start()` call used to hide in both frame loops.
    if let Some(id) = scene_vm.bad_cursor_scene() {
        println!("note: scene ${id:02X} latched but no script cursor was found - no scene byte ran");
    }
    Ok(())
}

/// A hand-off whose tail selected a zone screen: the ROM's
/// `$3D0FF`/`$3D208`/`$3D371`/`$3D49F`/`$3D61F`/`$3D79A`/`$3D6EB`/`$3D880`
/// loaders are ported into `PlayerSim::map_handoff` and `MapSim::load_screen`.
fn note_handoff_screen(_out: &sml2_core::player_sim::MapHandoffOutcome) {}

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

/// Where a `--keys` run got to: enough to tell "never reached the pipe" from
/// "reached it and nothing happened".
fn print_play_state(
    why: &str,
    frame: u64,
    sim: &PlayerSim,
    cam: &sml2_core::camera::CameraSim,
    obj: &sml2_core::objects::ObjectSim,
) {
    println!(
        "{why} at frame {frame}: mario=({:#06X},{:#06X}) state={:?} dead={} camera=({:#06X},{:#06X}) transition={} scene_state={:#04X} scene_id={:#04X} edge={:#04X} boss_mode={} boss_oam={} first_oam={:?} objects={} unported={:?}",
        sim.x,
        sim.y,
        sim.state(),
        sim.dead,
        cam.x,
        cam.y,
        cam.transition,
        cam.scene_state,
        cam.scene_id,
        cam.edge_byte(),
        cam.boss_mode(),
        obj.boss_oam.len(),
        obj.boss_oam.first(),
        obj.slots
            .iter()
            .filter(|s| s.state() != sml2_core::objects::SlotState::Inactive)
            .count(),
        obj.unported,
    );
}

/// Interactive play mode: the ported player physics + camera drive the
/// viewport (arrows move, Z/Space = A, X/Shift = B, Q/ESC quits). With
/// `--keys` the input comes from the tape instead, so the run is scriptable.
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
    scene_map: Option<Vec<u8>>,
    keys_tape: Option<String>,
    headless_frames: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (vw, vh) = view;
    if let Some(e) = edge_map.as_ref() {
        if e.len() == 48 {
            cam.edge_map.copy_from_slice(e);
        }
    } else {
        // Play mode has no capture to fall back on: with no table every edge
        // bit reads clear, so the vertical follow takes the no-hard-edge
        // branches and drifts downward over a jump.
        println!("note: level edge table missing — the camera will scroll with no edge bits");
    }
    if let Some(m) = scene_map.as_ref() {
        if m.len() == 48 {
            cam.scene_map.clone_from(m);
        }
    } else {
        println!("note: level scene map missing — sSceneId will stay 0");
    }
    let scripts = load_scene_scripts();
    let mut scene_vm = SceneVm::new();
    let mut ending_entry = EndingEntry::default();
    let mut ending_sequence = sml2_core::ending_sequence::EndingSequenceController::new();
    let mut ending_scene = sml2_core::ending_scene::EndingScene::default();
    let nodes = load_map_nodes();
    // The `$6x` opcode's tileset swap rewrites the level's own tile data and
    // metatile definitions mid-level, so play mode owns a mutable copy of the
    // manifest and a live tile block. The per-frame render copies the live
    // block instead of the level's extracted one.
    let mut manifest = manifest.clone();
    let mut bg_live = bg_base.to_vec();
    let tilesets = load_tilesets();
    let arena_layout = load_arena_layout();
    let mut layout_live = layout.clone();
    use minifb::{Key, Window, WindowOptions};
    // One free-running hardware-input origin spans pauses, fades and reloads.
    // A headless run has no hardware to sample, so it uses the replay clock:
    // one explicit tick per frame keeps the run deterministic.
    let native_clock = NativeDividerClock::new(0);
    let mut virtual_clock = VirtualDividerClock::new(0);
    let headless = headless_frames.is_some();
    let mut window = if headless {
        None
    } else {
        Some(
            Window::new(
                &format!(
                    "sml2 level {} — play (arrows move, Z jump, X run, ENTER pause, Q quits)",
                    manifest.level_id
                ),
                vw * scale,
                vh * scale,
                WindowOptions {
                    resize: true,
                    ..WindowOptions::default()
                },
            )
            .map_err(|e| format!("cannot open a window ({e}) — no display found"))?,
        )
    };
    if let Some(w) = window.as_mut() {
        w.set_target_fps(60);
    }
    let mut buf = vec![0u32; vw * scale * vh * scale];
    let mut anim_timer: u32 = 0;
    let mut anim_phase: u8 = 0;
    let mut prev_a = false;
    let mut prev_b = false;
    let mut prev_start = false;
    let mut prev_select = false;
    let mut tape_i = 0usize;
    let mut frame_n: u64 = 0;
    obj.configure(manifest.level_id);
    let obj_tables = spr.as_ref().map(|s| s.tables.clone());
    loop {
        if let Some(w) = window.as_ref() {
            if !w.is_open() || w.is_key_down(Key::Escape) || w.is_key_down(Key::Q) {
                break;
            }
        } else if frame_n >= headless_frames.unwrap_or(0) {
            print_play_state("headless end", frame_n, &sim, &cam, &obj);
            break;
        }
        if headless {
            virtual_clock.advance_tick();
        }
        let held = |k: Key| window.as_ref().is_some_and(|w| w.is_key_down(k));
        let mut a_down = held(Key::Z) || held(Key::Space);
        let mut b_down = held(Key::X) || held(Key::LeftShift);
        let mut start_down = held(Key::Enter);
        let mut select_down = held(Key::Backspace);
        let mut keys = PlayerKeys {
            right: held(Key::Right),
            left: held(Key::Left),
            up: held(Key::Up),
            down: held(Key::Down),
            a: a_down,
            b: b_down,
            start: start_down,
            select: select_down,
        };
        if let Some(tape) = keys_tape.as_deref() {
            let k = match tape.as_bytes().get(tape_i) {
                Some(&k) => k,
                None => {
                    print_play_state("tape end", frame_n, &sim, &cam, &obj);
                    break;
                }
            };
            tape_i += 1;
            let d = k as char;
            keys = PlayerKeys {
                right: d == 'R',
                left: d == 'L',
                up: d == 'U',
                down: d == 'D',
                a: d == 'A',
                b: d == 'B',
                start: d == 'S',
                select: d == 'E',
            };
            a_down = keys.a;
            b_down = keys.b;
            start_down = keys.start;
            select_down = keys.select;
        }
        frame_n += 1;
        let a_pressed = a_down && !prev_a;
        prev_a = a_down;
        let b_pressed = b_down && !prev_b;
        prev_b = b_down;
        // START and SELECT are read as press edges, like `hKeysPressed` — the
        // ROM's pause entry and resume both test the press, not the hold.
        let start_pressed = start_down && !prev_start;
        prev_start = start_down;
        let select_pressed = select_down && !prev_select;
        prev_select = select_down;
        sim.camx = cam.x;
        sim.camy = cam.y;
        sim.pan_timer = cam.pan_timer;
        // $FF97: the engine's frame counter ticks once per game frame. The
        // full byte is kept because the pause flash reads bit 4 of it; `phase`
        // is its low three bits.
        sim.phase = (sim.phase + 1) & 7;
        sim.frame_counter = sim.frame_counter.wrapping_add(1);
        // $05E4/$0640: dead → the arc only; the death fade freezes everything.
        let ending_frame = scene_frame_head(
            &mut scene_vm,
            &mut ending_entry,
            &mut sim,
            &mut cam,
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    start: start_pressed,
                    select: select_pressed,
                    ..PlayerKeys::default()
                },
            },
        );
        let gameplay = matches!(ending_frame, Some(EndingFrame::Gameplay(_)));
        let input = match ending_frame {
            Some(EndingFrame::Gameplay(input)) => input,
            _ => EndingInput::default(),
        };
        let keys = input.held;
        let (a_pressed, b_pressed) = (input.pressed.a, input.pressed.b);
        let (start_pressed, select_pressed) = (input.pressed.start, input.pressed.select);
        let was_dead = sim.dead == 0xFF;
        // The fade-trigger frame still runs the object engine ($0640
        // continues into $0652); the true fade frames do not. `$FF9B`: 9 =
        // the death fade, 24 = the exit fade ($1CBA's bell arm) — both leave
        // the in-level loop, so everything freezes while either is set.
        // VBlank still ticks `sFadeTimer`; the exit fade's last step is
        // `$3547`'s BCD tally + the `$14` hand-off.
        //
        // `$2756` (the pause) replaces the whole state-`$04` body, so a paused
        // frame freezes exactly like a fade — and like the fades the flag is
        // read at entry, so the frame START lands on still runs its object
        // pass (the ROM's `$273E` sits at the frame tail) before the freeze
        // takes effect on the next one.
        let frozen = !gameplay || sim.paused || sim.dead_fade || sim.exit_phase;
        let exit_done_at_entry = sim.exit_completed;
        if !gameplay {
            // The shared frame head dispatched this ending-entry frame.
            if let Some(EndingFrame::SequencePending) = ending_frame {
                let res =
                    ending_sequence.step(&mut sml2_core::ending_sequence::EndingStepContext {
                        scene: &mut ending_scene,
                        rom: None,
                        credits_text: None,
                        object_tables: obj_tables.as_ref(),
                        decrement_fade: true,
                    });
                if res.sfx_request_0 != 0 {
                    sim.sfx0 = res.sfx_request_0;
                }
                if res.sfx_request_1 != 0 {
                    sim.sfx1 = res.sfx_request_1;
                }
            }
        } else if frozen {
            if sim.exit_phase {
                sim.exit_fade_step();
            }
            // See the replay loop: `$FF9B = $14` comes from the fade's last
            // frame, so the hand-off body runs one frame later.
            if exit_done_at_entry {
                let handoff = sim.map_handoff(&nodes);
                note_handoff_screen(&handoff);
            }
        } else if was_dead {
            sim.death_arc(cam.y);
        } else {
            if cam.transition == 0 {
                // $307D — `$0F71`'s first call; see the replay loop above.
                cam.crouch_pan(&sim, &keys);
                sim.step(&keys, a_pressed, b_pressed, &layout_live);
            } else {
                // home.asm ~115: VBlank decrements sFadeTimer unconditionally.
                sim.tick_fade_timer();
            }
            // `$3451` id 4's `sStompChainCount` reset — see the replay loop.
            if sim.take_stomp_chain_reset() {
                obj.stomp_chain = 0;
            }
            // $2F72 — the bump alloc's tail; see the replay loop. Use the camera's
            // latched scroll_y and scroll_x.
            if let Some((by, bx)) = sim.below_bonk.take() {
                let scroll_y = cam.scroll_y;
                let scroll_x = cam.scroll_x;
                let special = obj.below_bonk(by, bx, scroll_y, scroll_x);
                sim.resolve_bump_coin(special);
            }
            // A fireball's `$06` break; see the replay loop above.
            if let Some((by, bx)) = sim.fire_below_bonk.take() {
                let scroll_y = cam.scroll_y;
                let scroll_x = cam.scroll_x;
                obj.below_bonk(by, bx, scroll_y, scroll_x);
            }
            // `$3261`'s `$2FED`; see the replay loop above.
            for i in 0..sim.fire_scan.len() {
                let Some((sy, sx)) = sim.fire_scan[i].take() else {
                    continue;
                };
                let hit = obj.fire_hit(sy, sx);
                if hit == FireHit::Die {
                    sim.sfx0 = 2;
                }
                sim.resolve_fire_hit(i, hit != FireHit::None);
            }
            cam.step(&mut sim);
            cam.update_enables(sim.x);
        }
        cam.update_player_pan(sim.y, was_dead, !frozen);
        if let Some(tables) = &obj_tables {
            if !frozen {
                let mario = build_contact(&sim, &keys, cam.scroll_pan);
                sample_level_player_inputs(&mut obj, &sim);
                obj.div = if headless {
                    virtual_clock.sample()
                } else {
                    native_clock.sample()
                };
                // `$0EB0`'s scene branch; see the replay loop.
                if cam.scene_load_pending {
                    let def_index = boss_def_index(manifest.level_id, cam.boss_fight_phase);
                    if boss_def_portable(def_index) {
                        if let Some((music, level_song)) = cam.begin_scene_load(manifest.level_id) {
                            // `$0ECA`: the boss song goes to BOTH bytes.
                            sim.music_request = music;
                            sim.level_song = level_song;
                            obj.spawn_boss(def_index);
                        }
                    } else if cam.take_blocked_scene_load() {
                        obj.note_blocked_scene_load();
                    }
                }
                obj.step(
                    cam.x,
                    cam.y,
                    &layout_live,
                    tables,
                    &mario,
                    cam.boss_mode(), // `$0652`
                );
                // Same call the replay loop makes. The copy that used to live
                // here was missing `on_ground_cleared` and `on_ground_obj`, so
                // play mode disagreed with every capture on both.
                sim.apply_object_result(&mut obj, keys.a);
                cam.apply_object_result(&mut obj);
            }
        }
        // Resume the blocking $20A4 call after gameplay, or its previous wait.
        let stepped = if gameplay || ending_frame.is_none() {
            scene_vm.step(&mut sim, &mut cam, &scripts)
        } else {
            SceneStep::Idle
        };
        // `$6x` (`$25AF`): the scene's tileset swap. It is a *graphics* change —
        // VRAM tiles plus the metatile definitions — so it has to reach the
        // renderer's inputs, not just the sim's `sLevelGfxIndex`. This is what
        // makes a boss arena look like an arena instead of like the level that
        // contains it.
        if let SceneStep::Ran(op) = stepped {
            if op >> 4 == 0x6 {
                let index = sim.level_gfx_index;
                let obj = spr.as_mut().map(|s| &mut s.objblock);
                let applied = match obj {
                    Some(o) => tilesets.apply(index, &mut bg_live, o, &mut manifest.metatiles),
                    None => false,
                };
                // The reload's other half: the room itself comes from bank 19
                // `$42F0[3]`, not from the level the arena sits in.
                let room = match arena_layout.as_ref() {
                    Some(a) => {
                        layout_live = a.clone();
                        true
                    }
                    None => false,
                };
                println!(
                    "$6x: tileset swap {index:#04X} (map {}) — {}; arena room {}",
                    tilesets.entry(index).map(|e| e.map_index).unwrap_or(0),
                    if applied { "applied" } else { "NOT APPLIED" },
                    if room { "reloaded" } else { "NOT RELOADED" }
                );
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
        if gameplay && !was_dead {
            sim.block_update();
        }
        if let Some(s) = spr.as_mut() {
            s.player = Some((sim.x, sim.y, f));
            s.player_shield = if was_dead {
                None
            } else {
                sim.moon_shield_frame().map(|sf| (sim.x, sim.y, sf))
            };
            s.player_pan = cam.player_pan;
            s.scroll_pan = cam.scroll_pan;
            // $2C82 is skipped while dead, and the dead path's own draw ($5550,
            // frame 107) overwrites the staging pair — so a bump sprite never
            // survives into the arc.
            if was_dead {
                s.bumpers.clear();
            } else {
                s.bumpers = sim.block_draw.clone();
            }
            s.fireballs = sim.fire_draw.clone();
            // The boss path draws itself: while `$0652` selects `$8DA9`, the
            // level's `$8000` engine does not run, so the slots are the fight's
            // parts — and their state byte is HP (the def image seeds it with
            // 6), never a slot state, so the level draw's `state == 2` filter
            // would both miss them and, at 2 HP, double-draw one. So: boss
            // frames take `$66000`'s own output and no instances at all.
            if cam.boss_mode() {
                s.insts.clear();
                s.boss_oam = obj.boss_oam.clone();
            } else {
                s.boss_oam.clear();
                // Draw ACTIVE objects: positions/flip from the engine's staging.
                s.insts = obj
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(i, o)| {
                        // `$C000`/`$C014`: only state 2 slots draw, and never one
                        // whose handler set `sObjectDone` this frame (a defeat).
                        o.state() == sml2_core::objects::SlotState::Active && !obj.slot_done[*i]
                    })
                    .map(|(_, o)| SpriteInst {
                        sprite: o.sprite(),
                        x: o.x(),
                        y: o.y(),
                        flip: o.b[sml2_core::objects::B_DEFEAT]
                            ^ o.b[sml2_core::objects::B_FLIP]
                            ^ o.b[sml2_core::objects::B_FLASH],
                    })
                    .collect();
            }
        }
        // $273E: the START check at the tail of the in-level frame — the ROM
        // calls it from the alive and the dead branch alike. START toggles the
        // pause; the ROM requires `sScreenTransition == 0` and blanks both key
        // bytes while `sFadeTimer != 0`, so those two gates are applied here.
        // While paused, SELECT abandons the level instead when
        // `sLevelProgressFlags` bit 7 says it was already cleared.
        if gameplay && sim.fade_timer == 0 && cam.transition == 0 {
            if start_pressed {
                if sim.paused {
                    sim.pause_exit();
                } else {
                    sim.pause_enter();
                }
            } else if sim.paused && select_pressed {
                sim.pause_select_exit();
            }
        }
        // $2756: a paused frame writes the flash byte to `sBGPalette` *and*
        // `sOAMPalette1`, so BG and OBJ0 flash together; `sOAMPalette2` (OBP1)
        // is never touched. Unpaused frames restore the level's OBJ0 value, so
        // this stays idempotent per frame.
        let flash = if sim.paused {
            Some(sim.pause_flash())
        } else {
            None
        };
        if let Some(s) = spr.as_mut() {
            s.obp0 = flash.unwrap_or(manifest.header.palettes[1]);
        }
        anim_timer += 1;
        if manifest.tile_anim.enabled && anim_timer >= manifest.tile_anim.speed as u32 * 2 {
            anim_timer = 0;
            anim_phase = (anim_phase + 1) & 3;
        }
        // Headless runs skip the whole render path: the frame loop's job in a
        // smoke run is the simulation, and rendering a 160x144 frame per level
        // would dominate the runtime.
        if let Some(w) = window.as_mut() {
            let mut bg = bg_live.clone();
            apply_tile_anim(&mut bg, &manifest.tile_anim, anim_rows, anim_phase);
            // $076B: the status bar's coin digits follow the sim (the picked-up
            // coin shows up immediately, like the console's per-frame patch).
            let hud_live = hud_row_coins(hud, sim.coins_high, sim.coins_low);
            let frame = render_view(
                &manifest,
                &layout_live,
                &sim.layout_writes,
                &bg,
                cam.x,
                cam.y,
                vw,
                vh,
                Some((&hud_live, hud_y)),
                spr.as_ref(),
                flash,
            )?;
            // Integer-scale + letterbox into the current window size.
            let (win_w, win_h) = w.get_size();
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
            w.update_with_buffer(&buf, win_w, win_h)
                .map_err(|e| e.to_string())?;
        }
    }
    println!(
        "play: ended at mario ({}, {}) cam ({}, {})",
        sim.x, sim.y, cam.x, cam.y
    );
    if headless {
        // Anything the port could not run is a failed smoke run, not a note.
        let mut failed = false;
        if !obj.unported.is_empty() {
            println!(
                "note: unported bank-16 handler path(s) hit: {}",
                obj.unported
                    .iter()
                    .map(|a| format!("${a:05X}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            failed = true;
        }
        if !scene_vm.unported().is_empty() {
            println!(
                "note: scene script opcode(s) with no port implementation: {}",
                scene_vm
                    .unported()
                    .iter()
                    .map(|o| format!("${o:02X}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            failed = true;
        }
        if let Some(id) = scene_vm.bad_cursor_scene() {
            println!("note: scene ${id:02X} latched but no script cursor was found - no scene byte ran");
            failed = true;
        }
        if failed {
            return Err("headless run reported unported paths".into());
        }
    }
    Ok(())
}

/// The dispatch boundary shared by live play, replay, and pipe integration tests.
fn scene_frame_head(
    scene: &mut SceneVm,
    ending: &mut EndingEntry,
    sim: &mut PlayerSim,
    cam: &mut sml2_core::camera::CameraSim,
    input: EndingInput,
) -> Option<EndingFrame> {
    if scene.owns_gameplay() {
        sim.tick_fade_timer();
        None
    } else {
        Some(ending_frame_head(ending, sim, cam, input))
    }
}

/// Shared live/replay dispatch boundary. Gameplay keeps its existing fade tick
/// in PlayerSim::step (or the transition arm); ending states tick here instead.
fn ending_frame_head(
    ending: &mut EndingEntry,
    sim: &mut PlayerSim,
    cam: &mut sml2_core::camera::CameraSim,
    input: EndingInput,
) -> EndingFrame {
    if ending.state() == EndingEntryState::Gameplay {
        // $5D6A precedes the scripted-input override at $5E12.
        update_level_timer_at_frame_head(sim);
    } else {
        sim.tick_fade_timer();
    }
    ending.frame_head(
        &mut cam.boss_fight_phase,
        sim.x,
        sim.y,
        &mut sim.fade_timer,
        input,
    )
}

/// Share the level route latch with object handlers, including pose 87.
fn sample_level_player_inputs(obj: &mut sml2_core::objects::ObjectSim, sim: &PlayerSim) {
    obj.sample_player_inputs(sim);
    obj.secret_route = sim.secret_route != 0;
}

/// Apply the resolved `$A800-$A807` block and `$5579` route/progress latches.
fn apply_level_entry(
    sim: &mut PlayerSim,
    cam: &mut sml2_core::camera::CameraSim,
    entry: sml2_core::level::LevelEntry,
) {
    sim.x = entry.start.mario_x;
    sim.y = entry.start.mario_y;
    sim.secret_route = entry.secret_route;
    sim.level_progress_flags = entry.progress_flags;
    cam.reset_level_start(entry.start.camera_x, entry.start.camera_y, sim.y as u8);
}

/// `$04F1`: the header seeds the countdown and the level's music byte on load.
fn seed_level_timer(sim: &mut PlayerSim, manifest: &LevelManifest) {
    sim.level_timer = sml2_core::level_timer::LevelTimer::new(manifest.header.time_limit);
    sim.level_song = manifest.header.music_track;
}

/// `$05E4`: countdown precedes death dispatch, player physics, and camera motion.
fn update_level_timer_at_frame_head(sim: &mut PlayerSim) {
    if !sim.paused && !sim.dead_fade && !sim.exit_phase {
        sim.update_level_timer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_route_entry_consumes_map_progress_and_reloads_stored_start() {
        use sml2_core::map::{MapNodeRecords, MapNodes, MapWalk};
        use sml2_core::map_sim::{MapInput, MapSim};
        use sml2_core::level::LevelStart;
        let (mut manifest, _, _) = level_files("level12.manifest.json", "level12.bgblock.bin");
        manifest.secret_entry = Some(LevelStart::from_bytes([
            0x94, 0x01, 0x31, 0x08, 0xBA, 0x01, 0x52, 0x08,
        ]));
        let mut table = vec![0; MapNodes::BLOB_LEN];
        table[2] = 11;
        table[MapNodes::NODES + 2] = 1;
        let nodes = MapNodes::from_bytes(&table).unwrap();
        for progress in [0, 0x80, 8, 0x88] {
            let mut map = MapSim::new();
            map.screen_id = 1;
            map.node = 2;
            map.level_progress[1] = progress;
            let transition = map.tick(
                MapInput { keys_pressed: 1, ..Default::default() },
                &nodes, &MapWalk::default(), &MapNodeRecords::default(),
            );
            assert_eq!(transition.level, Some(manifest.level_id));
            let entry = manifest.header.entry(transition.progress_flags, manifest.secret_entry);
            let mut sim = PlayerSim::new();
            let mut cam = sml2_core::camera::CameraSim::new(0, 0);
            sim.secret_route = 255; // old pickup/previous level must not leak.
            apply_level_entry(&mut sim, &mut cam, entry);
            let expected = if progress & 8 != 0 {
                [0x94, 0x01, 0x31, 0x08, 0xBA, 0x01, 0x52, 0x08]
            } else {
                manifest.header.entry(0, None).start.to_bytes()
            };
            let observed = |sim: &PlayerSim, cam: &sml2_core::camera::CameraSim| LevelStart {
                mario_y: sim.y, mario_x: sim.x, camera_y: cam.y, camera_x: cam.x,
            }.to_bytes();
            assert_eq!(observed(&sim, &cam), expected, "$A800-$A807 consumers");
            assert_eq!(sim.secret_route, if progress & 8 != 0 { 255 } else { 0 });
            assert_eq!(sim.level_progress_flags, progress);
            let mut obj = sml2_core::objects::ObjectSim::new(Vec::new(), cam.x);
            sample_level_player_inputs(&mut obj, &sim);
            assert_eq!(obj.secret_route, progress & 8 != 0);
            // Item $07 changes only the route latch. Death reload consumes
            // the stored start, rather than choosing from that mutable latch.
            sim.secret_route = 255;
            sim.respawn(entry.start.mario_x, entry.start.mario_y);
            apply_level_entry(&mut sim, &mut cam, entry);
            assert_eq!(observed(&sim, &cam), expected);
            assert_eq!(sim.secret_route, entry.secret_route);
        }
    }

    #[test]
    fn ending_entry_frame_head_keeps_trigger_player_and_objects_then_dispatches() {
        let (_, layout, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
        let tables = pan_sprites(None).tables;
        let mut sim = PlayerSim::new();
        sim.x = 162;
        sim.y = 224;
        sim.fade_timer = 2;
        sim.boss_clear = true;
        sim.level_timer = sml2_core::level_timer::LevelTimer::new(0x03);
        let mut cam = sml2_core::camera::CameraSim::new(80, 224);
        cam.boss_fight_phase = 3;
        let mut obj = sml2_core::objects::ObjectSim::new(Vec::new(), cam.x);
        let mut ending = EndingEntry::default();
        let action = ending_frame_head(
            &mut ending,
            &mut sim,
            &mut cam,
            EndingInput {
                held: PlayerKeys {
                    left: true,
                    start: true,
                    select: true,
                    ..PlayerKeys::default()
                },
                pressed: PlayerKeys {
                    a: true,
                    b: true,
                    start: true,
                    select: true,
                    ..PlayerKeys::default()
                },
            },
        );
        assert_eq!(ending.state() as u8, 35);
        assert_eq!(sim.fade_timer, 2, "player owns trigger-frame fade tick");
        let EndingFrame::Gameplay(input) = action else {
            panic!("trigger frame still runs gameplay");
        };
        assert!(input.held.right && input.held.a);
        assert!(!input.held.left && !input.held.start && !input.held.select);
        assert!(!input.pressed.a && !input.pressed.b && !input.pressed.start && !input.pressed.select);
        sim.step(&input.held, input.pressed.a, input.pressed.b, &layout);
        assert_eq!(sim.fade_timer, 1);
        assert!(
            sim.hvel != 0,
            "scripted Right reaches the real player update"
        );
        cam.step(&mut sim);
        let before_object_phase = obj.phase;
        let contact = build_contact(&sim, &input.held, cam.scroll_pan);
        obj.sample_player_inputs(&sim);
        obj.step(cam.x, cam.y, &layout, &tables, &contact, cam.boss_mode());
        assert_ne!(
            obj.phase, before_object_phase,
            "trigger frame runs the actual object engine"
        );
        sim.apply_object_result(&mut obj, input.held.a);
        cam.apply_object_result(&mut obj);
        assert!(
            !sim.exit_phase && !sim.exit_completed,
            "ending entry never rewards an ordinary level clear"
        );
        let timer_before_fade = sim.level_timer;
        assert!(matches!(
            ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default()),
            EndingFrame::Fade(_)
        ));
        assert_eq!(ending.state() as u8, 18);
        assert_eq!(
            sim.fade_timer, 255,
            "one tick reaches zero, then state35 resets fade"
        );
        assert_eq!(sim.level_timer, timer_before_fade);
        assert_eq!(cam.boss_fight_phase, 3);
        assert!(matches!(
            ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default()),
            EndingFrame::SequenceInitialized
        ));
        assert_eq!(
            (
                ending.state() as u8,
                ending.ending_state(),
                cam.boss_fight_phase,
                sim.fade_timer
            ),
            (19, 0, 0, 254)
        );
        assert!(matches!(
            ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default()),
            EndingFrame::SequencePending
        ));
        assert_eq!(sim.fade_timer, 253);
        assert!(!sim.exit_phase && !sim.exit_completed);
    }

    #[test]
    fn ending_sequence_runs_from_entry_through_all_substates_to_terminal_completion() {
        let mut sim = PlayerSim::new();
        sim.x = 162;
        sim.y = 224;
        sim.fade_timer = 2;
        sim.boss_clear = true;
        let mut cam = sml2_core::camera::CameraSim::new(80, 224);
        cam.boss_fight_phase = 3;
        let mut ending = EndingEntry::default();

        // 1. Scripted trigger frame
        let f1 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(f1, EndingFrame::Gameplay(_)));
        assert_eq!(ending.state() as u8, 35);

        // 2. Fade frame -> sets state 18 when fade reaches zero
        sim.fade_timer = 1;
        let f2 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(f2, EndingFrame::Fade(_)));
        assert_eq!(ending.state() as u8, 18);

        // 3. Initialize frame -> sets state 19
        let f3 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(f3, EndingFrame::SequenceInitialized));
        assert_eq!(ending.state() as u8, 19);

        // 4. State 19 sequence controller takes over
        let mut seq = sml2_core::ending_sequence::EndingSequenceController::new();
        let mut scene = sml2_core::ending_scene::EndingScene::default();
        let dummy_assets =
            sml2_core::ending_scene::EndingAssets::new(vec![0; 0x1800], vec![0; 0x400]).unwrap();
        let mut sound_driver = sml2_core::audio::SoundDriver::new();

        // Substates 0..3: blank frame waits
        for sub in 0..4 {
            let fp = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
            assert!(matches!(fp, EndingFrame::SequencePending));
            let res = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
                scene: &mut scene,
                rom: None,
                credits_text: None,
                object_tables: None,
                decrement_fade: false,
            });
            assert_eq!(res.substate, sub + 1);
        }

        // Substate 4: loads ending scene assets
        scene.load(dummy_assets, &mut 4, &mut 0x100);
        let res4 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res4.substate, 5);

        // Fast-forward substates 5..19 verifying state targets and audio dispatch
        // Jump substate 5 -> 6
        seq.fade_timer = 0;
        let res5 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res5.substate, 6);

        // Jump substate 6 -> 7
        seq.fade_timer = 0;
        let res6 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res6.substate, 7);

        // Jump substate 7 -> 8 (raster_y reached 139)
        seq.raster_y = 138;
        seq.frame = 0;
        let res7 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res7.substate, 8);
        assert!(seq.vram_transfer.is_some());

        // Substate 8 -> 9 (transfer finishes)
        seq.vram_transfer.as_mut().unwrap().active = false;
        let res8 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res8.substate, 9);

        // Substate 9 -> 10 (fade reaches 0)
        seq.fade_timer = 0;
        let res9 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res9.substate, 10);

        // Substate 10 -> 11 (raster_y retracts to 13)
        seq.raster_y = 14;
        seq.frame = 0;
        let res10 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res10.substate, 11);

        // Substate 11 -> 12 (ending tile flag set)
        seq.fade_timer = 0;
        let res11 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res11.substate, 12);
        assert_eq!(seq.ending_tile_flag, 0xFF);

        // Substate 12: burst cycle with SFX1=24 on odd frame
        seq.frame = 0;
        let res12 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res12.sfx_request_1, 24);
        sound_driver.sfx_request_1 = res12.sfx_request_1;
        sound_driver.update_sound();
        assert_eq!(sound_driver.sfx_request_1, 0, "sound driver consumed request");

        // Substate 12 -> 13 (fade reaches 0 -> SFX0=7 burst sound)
        seq.fade_timer = 0;
        let res12_end = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res12_end.substate, 13);
        assert_eq!(res12_end.sfx_request_0, 7);

        // Substate 13 -> 14
        seq.fade_timer = 0;
        let res13 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res13.substate, 14);

        // Substate 14 -> 15 (queues transfer 1)
        let res14 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res14.substate, 15);

        // Substate 15 -> 16 (transfer finished -> SFX0=6 castle transformation)
        seq.vram_transfer.as_mut().unwrap().active = false;
        let res15 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res15.substate, 16);
        assert_eq!(res15.sfx_request_0, 6);

        // Substate 16 -> 17
        seq.fade_timer = 0;
        let res16 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res16.substate, 17);

        // Substate 17 -> 18 (scroll_y reached 24)
        seq.fade_timer = 0;
        let res17 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res17.substate, 18);
        assert_eq!(res17.scroll_y, 24);
        assert!(res17.scanline_split_enabled);

        // Substate 18 -> 19 (credits text finishes)
        seq.credits_text.as_mut().unwrap().done = true;
        let res18 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res18.substate, 19);
        assert!(res18.terminal);
        assert!(res18.scanline_split_enabled);

        // Substate 19 is terminal
        let res19 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res19.substate, 19);
        assert!(res19.terminal);
    }

    #[test]
    fn level_timer_load_and_reload_use_header_bytes() {
        let (mut manifest, _, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
        manifest.header.time_limit = 0x12;
        manifest.header.music_track = 0x23;
        let mut sim = PlayerSim::new();
        sim.low_time_flag = 1;
        for _ in 0..2 {
            seed_level_timer(&mut sim, &manifest);
            assert_eq!(
                sim.level_timer,
                sml2_core::level_timer::LevelTimer::new(0x12)
            );
            assert_eq!(sim.level_song, 0x23);
            assert_eq!(sim.low_time_flag, 1);
            sim.level_timer.low = 0x19;
            sim.level_timer.high = 0;
            sim.level_timer.tick_counter = 1;
            sim.low_time_flag = 1;
            sim.level_song = 0xFF;
        }
    }

    #[test]
    fn level_timer_frame_head_freezes_with_pause_and_fades() {
        let mut sim = PlayerSim::new();
        sim.level_timer = sml2_core::level_timer::LevelTimer::new(0x03);
        let initial = sim.level_timer;
        for flags in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            (sim.paused, sim.dead_fade, sim.exit_phase) = flags;
            update_level_timer_at_frame_head(&mut sim);
            assert_eq!(sim.level_timer, initial);
        }
        (sim.paused, sim.dead_fade, sim.exit_phase) = (false, false, false);
        update_level_timer_at_frame_head(&mut sim);
        assert_eq!(sim.level_timer.tick_counter, 0x2F);
        assert_eq!((sim.level_timer.high, sim.level_timer.low), (3, 0));
    }

    #[test]
    fn level_timer_expiry_precedes_the_death_snapshot() {
        let mut sim = PlayerSim::new();
        sim.level_timer = sml2_core::level_timer::LevelTimer::new(0);
        sim.level_timer.tick_counter = 1;
        update_level_timer_at_frame_head(&mut sim);
        assert_eq!(sim.dead, 0x80);
        assert_eq!((sim.level_timer.high, sim.level_timer.low), (0, 0));
        assert_eq!(sim.sfx0, 0x0F);
    }

    /// Load one level's extracted manifest, layout and composed tile block.
    fn level_files(manifest_name: &str, bg_name: &str) -> (LevelManifest, LevelLayout, Vec<u8>) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../extracted/levels");
        let manifest: LevelManifest =
            serde_json::from_str(&fs::read_to_string(dir.join(manifest_name)).unwrap()).unwrap();
        let layout =
            LevelLayout::from_bytes(&fs::read(dir.join(&manifest.layout.file)).unwrap()).unwrap();
        let bg = fs::read(dir.join(bg_name)).unwrap();
        (manifest, layout, bg)
    }

    /// Top row of the level's solid block row in a rendered frame — the row carrying the
    /// widest run of the palette's darkest shade below row 60 (the block row; the cloud
    /// outlines never cover a whole row).
    fn block_row_top(img: &[u8], vw: usize, darkest: u8) -> usize {
        (60..144)
            .map(|y| (y, (0..vw).filter(|&x| img[y * vw + x] == darkest).count()))
            .max_by_key(|&(_, n)| n)
            .expect("a block row in the frame")
            .0
    }

    /// The view must not be clamped into the level's own bounds. Level 5's start camera sits
    /// at the bottom of a 768-px world (cam_y 704), so the view's top row (632) is past
    /// `768 - 144`: the engine lets the view overhang and the tilemap window's rows come back
    /// around under the status bar. Clamping the view to the level instead shifted the whole
    /// background 8 px down against Mario's OAM, and he looked like he floated above the
    /// block row. The console — PyBoy, entered through the map route in
    /// `captures/probe_level_select.py` — draws that block row's top on row 118.
    #[test]
    fn a_camera_at_the_level_bottom_does_not_shift_the_background() {
        let (manifest, layout, bg) = level_files("level06.manifest.json", "level06.bgblock.bin");
        let darkest = video::bgp_shades(manifest.header.palettes[0])[3];
        let img = render_view(&manifest, &layout, &[], &bg, 80, 704, 160, 144, None, None, None)
            .unwrap();
        assert_eq!(block_row_top(&img, 160, darkest), 118);
    }

    /// The same frame contract mid-level, where nothing is clamped either way: level 1's
    /// start camera (80, 448) draws its block row's top on row 120 on the console.
    #[test]
    fn a_camera_mid_level_lands_where_the_console_draws_it() {
        let (manifest, layout, bg) = level_files("level01.manifest.json", "level01.bgblock.bin");
        let darkest = video::bgp_shades(manifest.header.palettes[0])[3];
        let img = render_view(&manifest, &layout, &[], &bg, 80, 448, 160, 144, None, None, None)
            .unwrap();
        assert_eq!(block_row_top(&img, 160, darkest), 120);
    }

    /// `$04DD`/`home.asm:661`: `sMoonPhysics` is the level's own
    /// `LevelPropertiesSpace` byte (`$1F91`, indexed by `sCurLevel`), which both
    /// frame loops now seed from the manifest. Only three levels are nonzero —
    /// indices 18, 19 and 28, values `$08`, `$01`, `$08` — and level 1 is `$00`
    /// in all three property bytes, which is exactly why the replay gate was
    /// structurally blind to this: every capture ran on `PlayerSim::new()`'s own
    /// default. traj19 (level 20) records the byte, so the gate checks the
    /// derivation now; this test checks the three known levels' values.
    #[test]
    fn the_moon_physics_byte_comes_from_the_manifest_for_the_three_known_levels() {
        for (name, id, byte) in [
            ("level19.manifest.json", 18u8, 0x08u8),
            ("level20.manifest.json", 19, 0x01),
            ("level29.manifest.json", 28, 0x08),
        ] {
            let (manifest, _, _) = level_files(name, "level01.bgblock.bin");
            assert_eq!(manifest.level_id, id, "{name}");
            assert_eq!(
                manifest.properties.space_physics, byte,
                "{name}: the ROM's LevelPropertiesSpace entry"
            );
            let mut sim = PlayerSim::new();
            sim.seed_moon_physics(manifest.properties.space_physics);
            assert_eq!(sim.moon, byte, "{name}: seeded raw, not folded to a bool");
        }
        // The default side: level 1's byte is `$00`, so its play and replay are
        // unchanged by the seed — the reason 19 captures never saw the gap.
        let (lvl1, _, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
        assert_eq!(lvl1.properties.space_physics, 0x00);
        let mut sim = PlayerSim::new();
        sim.seed_moon_physics(lvl1.properties.space_physics);
        assert_eq!(sim.moon, 0x00);
    }

    fn pan_sprites(player: Option<(u16, u16, u8)>) -> SpriteCtx {
        SpriteCtx {
            tables: serde_json::from_str(
                &fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/objects.json"),
                )
                .unwrap(),
            )
            .unwrap(),
            level_id: 0,
            insts: Vec::new(),
            objblock: vec![255; 0x1000],
            obp0: 0xE4,
            obp1: 0xE4,
            player,
            player_shield: None,
            bumpers: Vec::new(),
            fireballs: Vec::new(),
            boss_oam: Vec::new(),
            player_pan: 0,
            scroll_pan: 0,
        }
    }

    #[test]
    fn pan_renderer_moves_bg_by_latched_scroll_phase() {
        let (manifest, layout, bg) = level_files("level01.manifest.json", "level01.bgblock.bin");
        let mut s = pan_sprites(None);
        let baseline = render_view(
            &manifest,
            &layout,
            &[],
            &bg,
            80,
            448,
            160,
            144,
            None,
            Some(&s),
            None,
        )
        .unwrap();
        s.scroll_pan = 3;
        let shifted = render_view(
            &manifest,
            &layout,
            &[],
            &bg,
            80,
            448,
            160,
            144,
            None,
            Some(&s),
            None,
        )
        .unwrap();
        assert_ne!(baseline, shifted);
        assert_eq!(&shifted[3 * 160..144 * 160], &baseline[..141 * 160]);
    }

    #[test]
    fn pan_renderer_moves_only_mario_by_post_decrement_phase() {
        let (manifest, layout, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
        let bg = vec![0; 0x1000];
        let mut s = pan_sprites(Some((34, 468, 32)));
        let baseline = render_view(
            &manifest,
            &layout,
            &[],
            &bg,
            80,
            448,
            160,
            144,
            None,
            Some(&s),
            None,
        )
        .unwrap();
        s.player_pan = 2;
        let shifted = render_view(
            &manifest,
            &layout,
            &[],
            &bg,
            80,
            448,
            160,
            144,
            None,
            Some(&s),
            None,
        )
        .unwrap();
        assert_ne!(baseline, shifted, "the mapping must draw visible pixels");
        assert_eq!(&shifted[..142 * 160], &baseline[2 * 160..144 * 160]);
    }

    #[test]
    fn wario_arena_entries_definitions_6_7_8_through_final_ending_integration() {
        let (_, layout, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
        let tables = pan_sprites(None).tables;
        let mut sim = PlayerSim::new();
        let mut cam = sml2_core::camera::CameraSim::new(80, 72);
        let mut obj = sml2_core::objects::ObjectSim::new(Vec::new(), cam.x);
        obj.level_id = 0x18;

        // =================================================================
        // PHASE 0: Wario Form 1 Entry (Definition 6)
        // =================================================================
        cam.scene_load_pending = true;

        let def_index = boss_def_index(0x18, cam.boss_fight_phase);
        assert_eq!(def_index, 6);
        assert!(boss_def_portable(def_index));

        let (music, level_song) = cam.begin_scene_load(0x18).unwrap();
        assert_eq!((music, level_song), (33, 32));
        sim.music_request = music;
        sim.level_song = level_song;

        obj.spawn_boss(def_index);
        assert_eq!(obj.boss_def_index, 6);
        assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 20); // Wario Form 1
        assert_eq!(obj.slots[1].b[sml2_core::objects::B_DEF], 21); // Child 21
        assert!(cam.boss_mode());
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());

        // Form 1 Defeat and Phase 1 Exit:
        // Set slot 0 to state 8 (exit walk) at threshold screen X
        obj.slots[0].b[sml2_core::objects::B_FSTATE] = 8;
        obj.slots[0].b[sml2_core::objects::B_SCRX] = 191;
        obj.fff0 = 1;
        obj.sample_player_inputs(&sim);
        obj.boss_engine(80, 72, &layout, &tables);

        // Object result processed by player and camera
        sim.apply_object_result(&mut obj, false);
        cam.apply_object_result(&mut obj);

        assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 0, "Form 1 slot blanked");
        assert_eq!(cam.boss_fight_phase, 1, "Phase advanced to 1");
        assert_eq!(cam.scene_state, 0, "Arena unparked for stream door");
        assert!(!cam.boss_mode());
        assert_eq!(obj.records.len(), 1);
        assert_eq!(obj.records[0].raw, [7, 0xC8, 0, 0, 0xB0, 0x1D]);
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());

        // =================================================================
        // PHASE 1: Wario Form 2 Entry (Definition 7)
        // =================================================================
        // Mario walks through the streamed door: transition finishes
        cam.scene_load_pending = true;

        let def_index = boss_def_index(0x18, cam.boss_fight_phase);
        assert_eq!(def_index, 7);
        assert!(boss_def_portable(def_index), "Definition 7 must be portable");

        let (music, level_song) = cam.begin_scene_load(0x18).unwrap();
        assert_eq!((music, level_song), (34, 32));
        sim.music_request = music;
        sim.level_song = level_song;

        obj.spawn_boss(def_index);
        assert_eq!(obj.boss_def_index, 7);
        assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 23); // Wario Form 2
        assert_eq!(obj.slots[1].b[sml2_core::objects::B_DEF], 24); // Child 24
        assert!(cam.boss_mode());
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());

        // Form 2 Defeat and Phase 2 Exit:
        // Set slot 0 to state 8 (exit walk) at threshold screen X
        obj.slots[0].b[sml2_core::objects::B_FSTATE] = 8;
        obj.slots[0].b[sml2_core::objects::B_SCRX] = 190;
        obj.sample_player_inputs(&sim);
        obj.boss_engine(80, 72, &layout, &tables);

        sim.apply_object_result(&mut obj, false);
        cam.apply_object_result(&mut obj);

        assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 0, "Form 2 slot blanked");
        assert_eq!(cam.boss_fight_phase, 2, "Phase advanced to 2");
        assert_eq!(cam.scene_state, 0, "Arena unparked for stream door");
        assert!(!cam.boss_mode());
        assert_eq!(obj.records.len(), 1);
        assert_eq!(obj.records[0].raw, [8, 0xC8, 0, 0, 0xB0, 27]);
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());

        // =================================================================
        // PHASE 2: Wario Form 3 Entry (Definition 8)
        // =================================================================
        // Mario walks through the streamed door: transition finishes
        cam.scene_load_pending = true;

        let def_index = boss_def_index(0x18, cam.boss_fight_phase);
        assert_eq!(def_index, 8);
        assert!(boss_def_portable(def_index), "Definition 8 must be portable");

        let (music, level_song) = cam.begin_scene_load(0x18).unwrap();
        assert_eq!((music, level_song), (35, 32));
        sim.music_request = music;
        sim.level_song = level_song;

        obj.spawn_boss(def_index);
        assert_eq!(obj.boss_def_index, 8);
        assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 26); // Wario Form 3
        assert!(cam.boss_mode());
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());

        // Form 3 Defeat and Phase 3 Exit:
        // Set slot 0 to state 10 (defeat exit) with expired timer at threshold screen X
        obj.slots[0].b[sml2_core::objects::B_FSTATE] = 10;
        obj.slots[0].b[sml2_core::objects::B_T28] = 0;
        obj.slots[0].b[sml2_core::objects::B_SCRX] = 191;
        obj.fff0 = 1;
        obj.sample_player_inputs(&sim);
        obj.boss_engine(80, 72, &layout, &tables);

        sim.apply_object_result(&mut obj, false);
        cam.apply_object_result(&mut obj);

        assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 0, "Form 3 slot blanked");
        assert_eq!(cam.boss_fight_phase, 3, "Phase advanced to 3 (Ending Entry)");
        assert_eq!(cam.scene_state, 0, "Arena unparked");
        assert!(!cam.boss_mode());
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());

        // =================================================================
        // PHASE 3: Ending Entry through Sequence Completion
        // =================================================================
        let mut ending = EndingEntry::default();
        sim.x = 162;
        sim.y = 224;
        sim.fade_timer = 2;
        sim.boss_clear = true;

        // Trigger frame
        let fp0 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(fp0, EndingFrame::Gameplay(_)));
        assert_eq!(ending.state(), EndingEntryState::Fade);

        // Fade frames
        let fp1 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(fp1, EndingFrame::Fade(_)));
        let fp2 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(fp2, EndingFrame::Fade(_)));
        assert_eq!(ending.state(), EndingEntryState::Initialize);

        // Initialize frame
        let fp3 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(fp3, EndingFrame::SequenceInitialized));
        assert_eq!(ending.state(), EndingEntryState::Sequence);

        // Sequence begins
        let fp4 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(fp4, EndingFrame::SequencePending));

        // Step EndingSequenceController to completion
        let mut seq = sml2_core::ending_sequence::EndingSequenceController::new();
        let mut scene = sml2_core::ending_scene::EndingScene::default();
        let dummy_assets =
            sml2_core::ending_scene::EndingAssets::new(vec![0; 0x1800], vec![0; 0x400]).unwrap();
        scene.load(dummy_assets, &mut 4, &mut 0x100);

        // Fast-forward substates to terminal completion
        seq.substate = 19;
        let final_step = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(final_step.substate, 19);
        assert!(final_step.terminal);

        // Verify clean diagnostics throughout
        assert!(!cam.take_blocked_scene_load());
        assert!(obj.unported.is_empty());
    }
}
