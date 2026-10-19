//! Existing command-line interface and audio export commands.

use crate::content::{extracted_dir, load_map_nodes};
use crate::game::{apply_level_entry, seed_level_timer, seed_unique_level_map_node};
use crate::image_output::write_png_scaled;
use crate::play::run_play;
use crate::play_tape::PlayTape;
use crate::render::{apply_tile_anim, hud_row, render_view, SpriteCtx, SpriteInst};
use crate::replay::run_replay;
use crate::viewer::{run_window, Viewer};
use crate::{session, SCREEN_H, SCREEN_W};
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest, LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::object::ObjectTables;
use sml2_core::player_sim::PlayerSim;
use sml2_core::video;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Native GB screen; the HUD strip and the parity checks are anchored to it.
fn export_song(
    song_id: u8,
    seconds: f32,
    out_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if song_id == 0 || song_id > 42 {
        return Err(format!("invalid song ID {song_id} (must be 1..=42)").into());
    }
    let mut driver = sml2_core::audio::SoundDriver::new();
    let mut apu = sml2_core::audio::Apu::new();

    driver.music_request = song_id;

    let total_frames = (seconds * 60.0).max(60.0) as usize;
    let mut samples = Vec::with_capacity(total_frames * 1470);

    for _ in 0..total_frames {
        driver.update_sound();
        apu.ingest_driver(&driver);
        let frame_samples = apu.frame_sample_count(44100);
        apu.generate_samples(44100, frame_samples, &mut samples);
    }

    sml2_core::audio::write_wav_file(out_path, &samples, 44100)?;
    println!(
        "Exported Song {song_id} to {} ({:.1}s, {} samples)",
        out_path.display(),
        seconds,
        samples.len() / 2
    );
    Ok(())
}

fn export_sfx(slot: u8, sfx_id: u8, out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut driver = sml2_core::audio::SoundDriver::new();
    let mut apu = sml2_core::audio::Apu::new();

    match slot {
        0 => {
            if sfx_id == 0 || sfx_id > 24 {
                return Err(format!("invalid Slot 0 SFX ID {sfx_id} (must be 1..=24)").into());
            }
            driver.sfx_request_0 = sfx_id;
        }
        1 => {
            if sfx_id == 0 || sfx_id > 24 {
                return Err(format!("invalid Slot 1 SFX ID {sfx_id} (must be 1..=24)").into());
            }
            driver.sfx_request_1 = sfx_id;
        }
        2 => {
            if sfx_id == 0 || sfx_id > 11 {
                return Err(format!("invalid Slot 2 SFX ID {sfx_id} (must be 1..=11)").into());
            }
            driver.sfx_request_2 = sfx_id;
        }
        other => return Err(format!("invalid SFX slot {other} (must be 0, 1, or 2)").into()),
    }

    let total_frames = 120;
    let mut samples = Vec::with_capacity(total_frames * 1470);

    for _ in 0..total_frames {
        driver.update_sound();
        apu.ingest_driver(&driver);
        let frame_samples = apu.frame_sample_count(44100);
        apu.generate_samples(44100, frame_samples, &mut samples);
    }

    sml2_core::audio::write_wav_file(out_path, &samples, 44100)?;
    println!(
        "Exported SFX slot {slot} id {sfx_id} to {} ({} samples)",
        out_path.display(),
        samples.len() / 2
    );
    Ok(())
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut map_mode = false;
    let mut save_path = PathBuf::from("sml2.sav");
    let mut level_progress = 0u8;
    let mut headless_frames: Option<u64> = None;
    let mut keys_tape: Option<String> = None;
    let mut audio_out: Option<PathBuf> = None;
    let mut export_song_id: Option<u8> = None;
    let mut export_sfx_pair: Option<(u8, u8)> = None;
    let mut export_seconds: f32 = 15.0;
    let mut view = (SCREEN_W, SCREEN_H);
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--audio-out" => {
                audio_out = Some(PathBuf::from(
                    args.next().ok_or("--audio-out needs a path")?,
                ));
            }
            "--export-song" => {
                export_song_id = Some(
                    args.next()
                        .ok_or("--export-song needs a song ID (1..=42)")?
                        .parse()?,
                );
            }
            "--export-sfx" => {
                let slot = args
                    .next()
                    .ok_or("--export-sfx needs SLOT (0..=2)")?
                    .parse::<u8>()?;
                let id = args
                    .next()
                    .ok_or("--export-sfx needs ID (1..=24)")?
                    .parse::<u8>()?;
                export_sfx_pair = Some((slot, id));
            }
            "--seconds" => {
                export_seconds = args
                    .next()
                    .ok_or("--seconds needs a duration in seconds")?
                    .parse::<f32>()?;
            }
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
            "--map" => map_mode = true,
            "--save" => {
                save_path = PathBuf::from(args.next().ok_or("--save needs a path")?);
            }
            // The map's `$A27D` entry byte (decimal), consumed by `$5579`.
            "--level-progress" => {
                level_progress = args
                    .next()
                    .ok_or("--level-progress needs a byte")?
                    .parse()?;
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
    let resolve_out = |default_name: String| -> PathBuf {
        if let Some(p) = audio_out.clone() {
            p
        } else if out.to_string_lossy() != "frame.png" {
            out.clone()
        } else {
            PathBuf::from(default_name)
        }
    };
    if let Some(song_id) = export_song_id {
        let out_path = resolve_out(format!("song{song_id:02}.wav"));
        return export_song(song_id, export_seconds, &out_path);
    }
    if let Some((slot, sfx_id)) = export_sfx_pair {
        let out_path = resolve_out(format!("sfx_slot{slot}_id{sfx_id}.wav"));
        return export_sfx(slot, sfx_id, &out_path);
    }
    let tape = keys_tape.clone().map(PlayTape::Ascii);
    if map_mode || level.is_none() {
        let assets = session::SessionAssets::load()?;
        let mut runner =
            session::SessionRunner::new(assets, save_path, scale, view, headless_frames.is_some());
        runner.audio_out_path = audio_out;
        if let Some(n) = headless_frames {
            runner.run_headless(n, tape)?;
            return Ok(());
        }
        runner.run_window(tape)?;
        return Ok(());
    }
    let level = level.unwrap();
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
                player_behind_bg: false,
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
        camera.map(|c| c.0).unwrap_or(level_entry.start.camera_x),
        camera.map(|c| c.1).unwrap_or(level_entry.start.camera_y),
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
        // Direct level selection can recover a unique overworld origin. The
        // castle and level id 17 need route context from the overworld itself.
        seed_unique_level_map_node(&mut sim, manifest.level_id, &load_map_nodes());
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
            keys_tape.map(PlayTape::Ascii),
            headless_frames,
        )
        .map(|_| ());
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
