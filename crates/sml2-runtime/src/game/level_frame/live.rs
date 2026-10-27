//! The native frame driver; hosts retain departure and presentation ownership.
use super::{
    prepare_object_step, step_camera, step_ending_sequence, step_level_scene, step_objects,
    step_player, FloatStep,
};
use crate::render::{SpriteCtx, SpriteInst};
use crate::session::BOSS_LOAD_VBLANKS;
use sml2_core::{
    camera::CameraSim,
    ending_entry::{EndingEntry, EndingFrame, EndingInput},
    ending_scene::EndingScene,
    ending_sequence::EndingSequenceController,
    level::{LevelLayout, LevelManifest},
    objects::ObjectSim,
    player_sim::PlayerSim,
    scene::{SceneScripts, SceneStep, SceneVm},
    tile_animation::TileAnimationClock,
    tileset::Tilesets,
};

/// Freeze/death flags are sampled before this frame can trigger a departure.
#[derive(Clone, Copy)]
pub(crate) struct LiveFrameHead {
    ending_frame: Option<EndingFrame>,
    input: EndingInput,
    pub gameplay: bool,
    was_dead: bool,
    pub frozen: bool,
    pub exit_done_at_entry: bool,
}

pub(crate) fn begin_live_level_frame(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    scene_vm: &mut SceneVm,
    ending_entry: &mut EndingEntry,
    input: EndingInput,
) -> LiveFrameHead {
    sim.camx = cam.x;
    sim.camy = cam.y;
    sim.pan_timer = cam.pan_timer;
    sim.phase = (sim.phase + 1) & 7;
    sim.frame_counter = sim.frame_counter.wrapping_add(1);
    // `$05D5` runs after this frame's VBlank decrement and before `$20A4`'s
    // scene, and clears `hKeysHeld`/`hKeysPressed` while `sFadeTimer` is
    // nonzero, so the arena intro holds Mario still.
    let fade_locks_input = sim.fade_timer > 1;
    // `$5E12` (inside `$2C61`) writes its scripted keys after that clear,
    // so the lock masks only the joypad the ending head may replace.
    let joypad = if fade_locks_input {
        EndingInput::default()
    } else {
        input
    };

    let ending_frame = super::super::scene_frame_head(scene_vm, ending_entry, sim, cam, joypad);
    let gameplay = matches!(ending_frame, Some(EndingFrame::Gameplay(_)));
    let input = match ending_frame {
        Some(EndingFrame::Gameplay(input)) => input,
        _ => EndingInput::default(),
    };
    let was_dead = sim.dead == 0xFF;
    let frozen = !gameplay || sim.paused || sim.dead_fade || sim.exit_phase;
    let exit_done_at_entry = sim.exit_completed;

    LiveFrameHead {
        ending_frame,
        input,
        gameplay,
        was_dead,
        frozen,
        exit_done_at_entry,
    }
}

/// A borrowed view of existing state; custom diagnostic seeds stay in place.
pub(crate) struct LiveLevelFrame<'a> {
    pub sim: &'a mut PlayerSim,
    pub cam: &'a mut CameraSim,
    pub obj: &'a mut ObjectSim,
    pub scene_vm: &'a mut SceneVm,
    pub ending_sequence: &'a mut EndingSequenceController,
    pub ending_scene: &'a mut EndingScene,
    pub credits_text: Option<&'a [u8]>,
    pub manifest: &'a mut LevelManifest,
    pub layout_live: &'a mut LevelLayout,
    pub bg_live: &'a mut Vec<u8>,
    pub spr: &'a mut Option<SpriteCtx>,
    pub scripts: &'a SceneScripts,
    pub tilesets: &'a Tilesets,
    pub arena_layout: Option<&'a LevelLayout>,
    pub anim_rows: &'a [u8],
    pub tile_animation: &'a mut TileAnimationClock,
}

pub(crate) struct ArenaReload {
    pub index: u8,
    pub applied: bool,
    pub room_reloaded: bool,
}

#[derive(Default)]
pub(crate) struct LiveFrameResult {
    pub float_use: Option<FloatStep>,
    pub transition_finished: bool,
    pub boss_definition: Option<u8>,
    pub active_objects: bool,
    pub arena_reload: Option<ArenaReload>,
    pub flash: Option<u8>,
    pub tile_animation_copied: bool,
}

/// Run the native frame after the host's death/exit-fade policy. Those policies
/// differ between diagnostic play and a session that can leave the level.
pub(crate) fn step_live_level_frame(
    state: LiveLevelFrame<'_>,
    head: LiveFrameHead,
    div: u8,
) -> LiveFrameResult {
    let LiveLevelFrame {
        sim,
        cam,
        obj,
        scene_vm,
        ending_sequence,
        ending_scene,
        credits_text,
        manifest,
        layout_live,
        bg_live,
        spr,
        scripts,
        tilesets,
        arena_layout,
        anim_rows,
        tile_animation,
    } = state;
    let LiveFrameHead {
        ending_frame,
        input,
        gameplay,
        was_dead,
        frozen,
        ..
    } = head;
    let keys = input.held;
    let mut result = LiveFrameResult::default();
    let obj_tables = spr.as_ref().map(|s| &s.tables);

    if !gameplay {
        if let Some(EndingFrame::SequencePending) = ending_frame {
            step_ending_sequence(sim, ending_sequence, ending_scene, credits_text, obj_tables);
        }
    } else if !frozen {
        // The host has already handled its frozen-frame departure policy.
        if was_dead {
            sim.death_arc(cam.y);
        } else {
            result.float_use = step_player(sim, cam, obj, layout_live, input);
            result.transition_finished = step_camera(sim, cam);
        }
    }
    cam.update_player_pan(sim.y, was_dead, !frozen);

    if let Some(tables) = obj_tables {
        if !frozen {
            let mario = super::super::build_contact(sim, &keys, cam.scroll_pan, cam.scene_state);
            if let Some(definition) = prepare_object_step(sim, cam, obj, manifest.level_id, div) {
                result.boss_definition = Some(definition);
                // `$3C94` runs across an extra VBlank before returning.
                sim.fade_timer = sim.fade_timer.saturating_sub(BOSS_LOAD_VBLANKS);
            }
            if step_objects(sim, cam, obj, layout_live, tables, &mario, keys.a) {
                result.active_objects = true;
            }
        }
    }

    let stepped = step_level_scene(
        sim,
        cam,
        scene_vm,
        scripts,
        ending_frame,
        bg_live,
        spr.as_mut(),
    );
    if let SceneStep::Ran(op) = stepped {
        if op >> 4 == 0x6 {
            let index = sim.level_gfx_index;
            let applied = match spr.as_mut() {
                Some(s) => tilesets.apply(index, bg_live, &mut s.objblock, &mut manifest.metatiles),
                None => false,
            };
            let room_reloaded = if let Some(arena) = arena_layout {
                *layout_live = arena.clone();
                // `$25AF` replaces the map, including prior block edits.
                sim.layout_writes.clear();
                true
            } else {
                false
            };
            result.arena_reload = Some(ArenaReload {
                index,
                applied,
                room_reloaded,
            });
        }
    }

    let f = if was_dead {
        107
    } else {
        // `$530D` only runs on simulated frames. Its walk request must not
        // leak out of a paused, dead-fade or exit-fade frame — the tone
        // would sustain for as long as Mario's last speed byte survives.
        let held_sfx0 = sim.sfx0;
        let f = sim.frame_index(sim.walk_timer);
        if frozen || held_sfx0 != 0 {
            sim.sfx0 = held_sfx0;
        }
        f
    };
    if gameplay && !was_dead {
        sim.block_update();
    }
    let streak = if gameplay && !was_dead {
        obj.tick_stomp_bounce_timer()
    } else {
        None
    };

    if let Some(s) = spr.as_mut() {
        let y = if was_dead { sim.death_draw_y(cam.y) } else { sim.y };
        s.player = Some((sim.x, y, f));
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
        if was_dead {
            s.bumpers.clear();
        } else {
            s.bumpers = sim.block_draw.clone();
        }
        s.fireballs = sim.fire_draw.clone();
        if cam.boss_mode() {
            s.insts.clear();
            s.boss_oam = obj.boss_oam.clone();
        } else {
            s.boss_oam.clear();
            s.insts = obj
                .slots
                .iter()
                .enumerate()
                .filter(|(i, o)| {
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

    if gameplay && sim.fade_timer == 0 && cam.transition == 0 {
        if input.pressed.start {
            if sim.paused {
                sim.pause_exit();
            } else {
                sim.pause_enter();
            }
        } else if sim.paused && input.pressed.select {
            sim.pause_select_exit();
        }
    }

    result.flash = sim.paused.then(|| sim.pause_flash());
    if let Some(s) = spr.as_mut() {
        s.obp0 = result.flash.unwrap_or(manifest.header.palettes[1]);
    }
    result.tile_animation_copied = super::super::update_tile_animation(
        sim,
        cam.scene_id != 0,
        bg_live,
        anim_rows,
        tile_animation,
    );
    result
}
