//! Existing single-level diagnostic play host and input-tape results.

use crate::content::{load_arena_layout, load_map_nodes, load_scene_scripts, load_tilesets};
use crate::divider_clock::VirtualDividerClock;
use crate::game::{
    build_contact, note_handoff_screen, scene_frame_head, update_tile_animation, BellArm,
    TimedFloatUse,
};
use crate::play_tape::PlayTape;
use crate::present::letterbox_gray;
use crate::render::{hud_row_status, render_view, HudDigits, SpriteCtx, SpriteInst};
use sml2_core::ending_entry::{EndingEntry, EndingFrame, EndingInput};
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_core::scene::{SceneStep, SceneVm};

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
        sim.camx = cam.x;
        sim.camy = cam.y;
        sim.pan_timer = cam.pan_timer;
        // $FF97: the engine's frame counter ticks once per game frame. The
        // full byte is kept because the pause flash reads bit 4 of it; `phase`
        // is its low three bits.
        sim.phase = (sim.phase + 1) & 7;
        sim.frame_counter = sim.frame_counter.wrapping_add(1);
        // `$05D5` clears the held and pressed keys while `sFadeTimer` is
        // nonzero, so the arena intro holds Mario still; see the session.
        let fade_locks_input = sim.fade_timer > 1;
        // `$5E12` (inside `$2C61`) writes its scripted keys after that clear,
        // so the lock masks only the joypad the ending head may replace.
        let joypad = if fade_locks_input {
            EndingInput::default()
        } else {
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    start: start_pressed,
                    select: select_pressed,
                    ..PlayerKeys::default()
                },
            }
        };
        // $05E4/$0640: dead → the arc only; the death fade freezes everything.
        let ending_frame =
            scene_frame_head(&mut scene_vm, &mut ending_entry, &mut sim, &mut cam, joypad);
        let gameplay = matches!(ending_frame, Some(EndingFrame::Gameplay(_)));
        let input = match ending_frame {
            Some(EndingFrame::Gameplay(input)) => input,
            _ => EndingInput::default(),
        };
        let keys = input.held;
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
            // The session leaves the level the frame after the exit completes,
            // so a boss clear's leftover fade (and its tally) never runs.
            if sim.exit_phase && !exit_done_at_entry {
                sim.exit_fade_step();
            }
            // See the replay loop: `$FF9B = $14` comes from the fade's last
            // frame, so the hand-off body runs one frame later.
            if exit_done_at_entry {
                let handoff = sim.map_handoff(&nodes);
                note_handoff_screen(&handoff);
                if map_handoff.is_none() {
                    map_handoff = Some(handoff);
                }
            }
        } else if was_dead {
            sim.death_arc(cam.y);
        } else {
            let player = crate::game::level_frame::step_player_camera(
                &mut sim,
                &mut cam,
                &mut obj,
                &layout_live,
                input,
            );
            if let Some(sample) = player.float_use {
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
        }
        cam.update_player_pan(sim.y, was_dead, !frozen);
        if let Some(tables) = &obj_tables {
            if !frozen {
                let mario = build_contact(&sim, &keys, cam.scroll_pan, cam.scene_state);
                if crate::game::level_frame::prepare_object_step(
                    &mut sim,
                    &mut cam,
                    &mut obj,
                    manifest.level_id,
                    virtual_clock.sample(),
                )
                .is_some()
                {
                    // `$3C94` outlasts its frame; replay owns different capture timing.
                    sim.fade_timer = sim
                        .fade_timer
                        .saturating_sub(crate::session::BOSS_LOAD_VBLANKS);
                }
                if crate::game::level_frame::step_objects(
                    &mut sim,
                    &mut cam,
                    &mut obj,
                    &layout_live,
                    tables,
                    &mario,
                    keys.a,
                ) {
                    active_object_frames += 1;
                }
            }
        }
        // Resume the blocking $20A4 call after gameplay, or its previous wait.
        let scene_camera = (cam.x, cam.y);
        let stepped = if gameplay || ending_frame.is_none() {
            scene_vm.step(&mut sim, &mut cam, &scripts)
        } else {
            SceneStep::Idle
        };
        crate::game::apply_scene_transfer(&mut scene_vm, &mut bg_live, spr.as_mut());
        if let Some(s) = spr.as_mut() {
            crate::render::PipeBackground::update(
                &mut s.pipe_background,
                scene_camera,
                &cam,
                stepped,
            );
        }
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
                        sim.layout_writes.clear();
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
            let held_sfx0 = sim.sfx0;
            let f = sim.frame_index(sim.walk_timer);
            if frozen || held_sfx0 != 0 {
                sim.sfx0 = held_sfx0;
            }
            f
        };
        // $2C82: tick the bump slots. The bump stages its own sprite at the
        // block's screen coords (`block_draw`), which the renderer draws as an
        // extra OAM sprite while the player sprite keeps Mario's frame.
        if gameplay && !was_dead {
            sim.block_update();
        }
        let streak = if gameplay && !was_dead {
            obj.tick_stomp_bounce_timer()
        } else {
            None
        };
        if let Some(s) = spr.as_mut() {
            s.player = Some((sim.x, sim.y, f));
            s.streak_counter = streak;
            // `$54C8`: `$FFC7` from the star countdown; the dead draw (`$5550`)
            // does not go through it.
            s.player_star_flash = !was_dead && sim.star_flash();
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
                        // whose handler set `sObjectDone` this frame (a defeat)
                        // or whose type is `$80` (an item inside its block).
                        o.state() == sml2_core::objects::SlotState::Active && !obj.slot_done[*i]
                    })
                    .map(|(i, o)| SpriteInst {
                        sprite: o.sprite(),
                        x: o.x(),
                        y: o.y(),
                        flip: o.b[sml2_core::objects::B_DEFEAT]
                            ^ o.b[sml2_core::objects::B_FLIP]
                            ^ o.b[sml2_core::objects::B_FLASH],
                        slot: i as u8,
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
        if gameplay && !frozen && sim.dead == 0xFF {
            gameplay_death_frames += 1;
        }
        if sim.exit_completed && exit_completed_frame.is_none() {
            exit_completed_frame = Some(frame_n);
        }
        if sim.map_handoff_done && map_handoff_frame.is_none() {
            map_handoff_frame = Some(frame_n);
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
        update_tile_animation(
            &sim,
            cam.scene_id != 0,
            &mut bg_live,
            anim_rows,
            &mut tile_animation,
        );
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
