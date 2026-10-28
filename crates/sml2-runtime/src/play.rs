//! Existing single-level diagnostic play host and input-tape results.

use crate::content::{load_arena_layout, load_map_nodes, load_scene_scripts, load_tilesets};
use crate::divider_clock::VirtualDividerClock;
use crate::game::level_frame::{begin_live_level_frame, step_live_level_frame, LiveLevelFrame};
use crate::game::{note_handoff_screen, BellArm, TimedFloatUse};
use crate::play_tape::PlayTape;
use crate::present::letterbox_gray;
use crate::render::{hud_row_status, render_view, HudDigits, SpriteCtx};
use sml2_core::ending_entry::{EndingEntry, EndingInput};
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_core::scene::SceneVm;

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

/// Final native play state, also used to accept input tapes through this exact
/// dispatch loop rather than a test's copy of the gameplay frame ordering.
#[allow(dead_code)]
pub(crate) struct PlayOutcome {
    pub(crate) sim: PlayerSim,
    pub(crate) cam: sml2_core::camera::CameraSim,
    pub(crate) obj: sml2_core::objects::ObjectSim,
    pub(crate) frames: u64,
    pub(crate) bell_arm: Option<BellArm>,
    pub(crate) exit_completed_frame: Option<u64>,
    pub(crate) map_handoff_frame: Option<u64>,
    pub(crate) map_handoff: Option<sml2_core::player_sim::MapHandoffOutcome>,
    pub(crate) active_object_frames: u64,
    pub(crate) timed_float_use: Option<TimedFloatUse>,
    pub(crate) timed_float_frames: u64,
    pub(crate) gameplay_death_frames: u64,
}

/// Interactive play mode: the ported player physics + camera drive the
/// viewport (arrows move, Z/Space = A, X/Shift = B, Q/ESC quits). With
/// `--keys` the input comes from the tape instead, so the run is scriptable.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_play(
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
    keys_tape: Option<PlayTape>,
    headless_frames: Option<u64>,
    mut virtual_clock: VirtualDividerClock,
) -> Result<PlayOutcome, Box<dyn std::error::Error>> {
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
    // The caller's clock is already advanced by whatever ran before the level
    // (a map entry, say); one console frame per tick, windowed or headless.
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
    let mut tile_animation = sml2_core::tile_animation::TileAnimationClock::default();
    let mut prev_a = false;
    let mut prev_b = false;
    let mut prev_start = false;
    let mut prev_select = false;
    let mut tape_i = 0usize;
    let mut frame_n: u64 = 0;
    let mut bell_arm = None;
    let mut exit_completed_frame = None;
    let mut map_handoff_frame = None;
    let mut map_handoff = None;
    let mut active_object_frames = 0;
    let mut timed_float_use = None;
    let mut timed_float_frames = 0;
    let mut gameplay_death_frames = 0;
    obj.configure(manifest.level_id);
    loop {
        if let Some(w) = window.as_ref() {
            if !w.is_open() || w.is_key_down(Key::Escape) || w.is_key_down(Key::Q) {
                break;
            }
        } else if frame_n >= headless_frames.unwrap_or(0) {
            print_play_state("headless end", frame_n, &sim, &cam, &obj);
            break;
        }
        virtual_clock.advance_tick();
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
        if let Some(tape) = keys_tape.as_ref() {
            keys = match tape.frame(tape_i) {
                Some(keys) => keys,
                None => {
                    print_play_state("tape end", frame_n, &sim, &cam, &obj);
                    break;
                }
            };
            tape_i += 1;
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
        let head = begin_live_level_frame(
            &mut sim,
            &mut cam,
            &mut scene_vm,
            &mut ending_entry,
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    start: start_pressed,
                    select: select_pressed,
                    ..Default::default()
                },
            },
        );
        if head.gameplay && head.frozen {
            // The session leaves the level the frame after the exit completes,
            // so a boss clear's leftover fade (and its tally) never runs.
            if sim.exit_phase && !head.exit_done_at_entry {
                sim.exit_fade_step();
            }
            // See the replay loop: `$FF9B = $14` comes from the fade's last
            // frame, so the hand-off body runs one frame later.
            if head.exit_done_at_entry {
                let handoff = sim.map_handoff(&nodes);
                note_handoff_screen(&handoff);
                if map_handoff.is_none() {
                    map_handoff = Some(handoff);
                }
            }
        }
        let result = step_live_level_frame(
            LiveLevelFrame {
                sim: &mut sim,
                cam: &mut cam,
                obj: &mut obj,
                scene_vm: &mut scene_vm,
                ending_sequence: &mut ending_sequence,
                ending_scene: &mut ending_scene,
                credits_text: None,
                manifest: &mut manifest,
                layout_live: &mut layout_live,
                bg_live: &mut bg_live,
                spr: &mut spr,
                scripts: &scripts,
                tilesets: &tilesets,
                arena_layout: arena_layout.as_ref(),
                rom_tables: None,
                anim_rows,
                tile_animation: &mut tile_animation,
            },
            head,
            virtual_clock.sample(),
        );
        if result.active_objects {
            active_object_frames += 1;
        }
        if let Some(sample) = result.float_use {
            timed_float_frames += 1;
            if timed_float_use.is_none() {
                timed_float_use = Some(TimedFloatUse {
                    frame: frame_n,
                    timer_before: sample.timer_before,
                    timer_after: sample.timer_after,
                    vvel_before: sample.vvel_before,
                    vvel_after: sample.vvel_after,
                });
            }
        }
        if let Some(reload) = result.arena_reload {
            let index = reload.index;
            println!(
                "$6x: tileset swap {index:#04X} (map {}) — {}; arena room {}",
                tilesets.entry(index).map(|e| e.map_index).unwrap_or(0),
                if reload.applied {
                    "applied"
                } else {
                    "NOT APPLIED"
                },
                if reload.room_reloaded {
                    "reloaded"
                } else {
                    "NOT RELOADED"
                }
            );
        }
        if sim.exit_phase && bell_arm.is_none() {
            bell_arm = Some(BellArm {
                frame: frame_n,
                position: (sim.x, sim.y),
                fade_timer: sim.fade_timer,
                music_request: sim.music_request,
                exit_tile_flag: sim.exit_tile_flag,
                dead: sim.dead,
            });
        }
        if head.gameplay && !head.frozen && sim.dead == 0xFF {
            gameplay_death_frames += 1;
        }
        if sim.exit_completed && exit_completed_frame.is_none() {
            exit_completed_frame = Some(frame_n);
        }
        if sim.map_handoff_done && map_handoff_frame.is_none() {
            map_handoff_frame = Some(frame_n);
        }
        let flash = result.flash;
        // Headless runs skip the whole render path: the frame loop's job in a
        // smoke run is the simulation, and rendering a 160x144 frame per level
        // would dominate the runtime.
        if let Some(w) = window.as_mut() {
            // $076B: lives, coins, money bags, and the countdown, every frame.
            let hud_live = hud_row_status(
                hud,
                HudDigits {
                    lives: sim.lives,
                    coins_high: sim.coins_high,
                    coins_low: sim.coins_low,
                    money_bag: obj.money_bag,
                    timer_high: sim.level_timer.high,
                    timer_low: sim.level_timer.low,
                },
            );
            let frame = render_view(
                &manifest,
                &layout_live,
                &sim.layout_writes,
                &bg_live,
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
            letterbox_gray(&frame, (vw, vh), (win_w, win_h), &mut buf);
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
            println!(
                "note: scene ${id:02X} latched but no script cursor was found - no scene byte ran"
            );
            failed = true;
        }
        if failed {
            return Err("headless run reported unported paths".into());
        }
    }
    Ok(PlayOutcome {
        sim,
        cam,
        obj,
        frames: frame_n,
        bell_arm,
        exit_completed_frame,
        map_handoff_frame,
        map_handoff,
        active_object_frames,
        timed_float_use,
        timed_float_frames,
        gameplay_death_frames,
    })
}
