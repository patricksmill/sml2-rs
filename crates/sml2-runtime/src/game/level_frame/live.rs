//! The native frame driver; hosts retain departure and presentation ownership.
use super::{
    FloatStep, ScreenEdge, apply_screen_edges, clamp_shared_vertical, prepare_object_step,
    resolve_player_events, step_camera, step_ending_sequence, step_level_scene, step_objects,
    step_player,
};
use crate::render::{SpriteCtx, SpriteInst};
use crate::session::{BOSS_LOAD_VBLANKS, player_behind_bg};
use sml2_core::{
    camera::{CameraSim, CoopPlayer},
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
    pub rom_tables: Option<&'a crate::content::RomTables>,
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
    /// Co-op: players to put in a bubble this frame, NSMBW-style — left
    /// behind past a screen edge, or not taking the pipe the other player
    /// took (`dAcPy_c::setBalloonInNextgoto`).
    pub bubble: [bool; 2],
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
        rom_tables,
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
                crate::session::enter_layout_world(sim, cam, obj, layout_live, rom_tables);
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
        let y = if was_dead {
            sim.death_draw_y(cam.y)
        } else {
            sim.y
        };
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

/// The cooperative counterpart of [`step_live_level_frame`]: player 1 and
/// player 2 each run `$0F71`, the object engine ticks once with the alive
/// player as the primary contact, the other player gets a contact-only pass
/// (no second world tick), and the camera follows their midpoint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_two_player_live_level_frame(
    state: LiveLevelFrame<'_>,
    head: LiveFrameHead,
    div: u8,
    second: &mut PlayerSim,
    second_input: EndingInput,
    first_bubbled: bool,
    second_bubbled: bool,
    first_bubble: Option<(u16, u16, u8)>,
    second_bubble: Option<(u16, u16, u8)>,
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
        rom_tables,
        anim_rows,
        tile_animation,
    } = state;
    let LiveFrameHead {
        ending_frame,
        input,
        gameplay,
        frozen: head_frozen,
        ..
    } = head;
    let mut result = LiveFrameResult::default();
    let obj_tables = spr.as_ref().map(|s| &s.tables);
    // A bubbled player 1 is only out of play; its dead byte must not freeze
    // the survivor's dispatch. Pause and the shared exit still do.
    let frozen = if first_bubbled {
        !gameplay || sim.paused || sim.exit_phase
    } else {
        head_frozen
    };
    let second_frozen = !gameplay || sim.paused || sim.exit_phase || second_bubbled;

    if !gameplay {
        if let Some(EndingFrame::SequencePending) = ending_frame {
            step_ending_sequence(sim, ending_sequence, ending_scene, credits_text, obj_tables);
        }
    } else if !frozen {
        if !first_bubbled {
            result.float_use = step_player(sim, cam, obj, layout_live, input);
        }
        if !second_bubbled {
            // Shared counters: player 2 starts from player 1's post-update
            // totals, so its pickups are added once to the party's.
            second.lives = sim.lives;
            second.coins_low = sim.coins_low;
            second.coins_high = sim.coins_high;
            second.level_tally = sim.level_tally;
            second.secret_route = sim.secret_route;
            // The level map is shared: player 2 updates against player 1's
            // edit list (the one the renderer and the object pass read), so
            // the coins and blocks it takes are gone for everyone at once.
            std::mem::swap(&mut sim.layout_writes, &mut second.layout_writes);
            // No second `cam.crouch_pan`: the camera pan is shared and
            // player 1's update already owns it this frame.
            if cam.transition == 0 {
                second.step(
                    &second_input.held,
                    second_input.pressed.a,
                    second_input.pressed.b,
                    layout_live,
                );
            } else {
                second.tick_fade_timer();
            }
            std::mem::swap(&mut sim.layout_writes, &mut second.layout_writes);
            second.layout_writes.clone_from(&sim.layout_writes);
            resolve_player_events(second, cam, obj);
            sim.lives = second.lives;
            sim.coins_low = second.coins_low;
            sim.coins_high = second.coins_high;
            sim.level_tally = second.level_tally;
            // A secret-exit marker either player touched is the party's.
            sim.secret_route = second.secret_route;
        }
        // NSMBW order: each player meets the screen edges of the view it
        // moved in, then the camera follows the party.
        let mut edges = [ScreenEdge::default(); 2];
        if cam.transition == 0 {
            if !first_bubbled {
                edges[0] = apply_screen_edges(sim, layout_live, &input.held);
            }
            if !second_bubbled {
                edges[1] = apply_screen_edges(second, layout_live, &second_input.held);
            }
        }
        let role = |bubbled: bool, p: &PlayerSim, edge: ScreenEdge| CoopPlayer {
            live: !bubbled && p.dead != 0xFF,
            pressing_left: edge.pressing_left,
            pressing_right: edge.pressing_right,
        };
        let roles = [
            role(first_bubbled, sim, edges[0]),
            role(second_bubbled, second, edges[1]),
        ];
        let step = cam.step_coop([&mut *sim, &mut *second], roles);
        result.transition_finished = step.finished;
        if let Some(leader) = step.started_by {
            let other = 1 - leader;
            if roles[other].live {
                let (lead, rest) = if leader == 0 {
                    (&*sim, &mut *second)
                } else {
                    (&*second, &mut *sim)
                };
                if lead.pipe_dir >= 0x10 {
                    result.bubble[other] = true;
                } else {
                    // The room slides away under the other player; NSMBW
                    // moves the party with the area change, so it follows
                    // just behind the leader.
                    let behind: i16 = if cam.transition & 2 != 0 { 12 } else { -12 };
                    rest.x = lead.x.wrapping_add_signed(behind);
                    rest.y = lead.y;
                    rest.hvel = 0;
                    rest.vvel = 0;
                }
            }
        }
        for i in 0..2 {
            if edges[i].off_screen && roles[1 - i].live {
                result.bubble[i] = true;
            }
        }
    }
    let pan_y = if first_bubbled && !second_bubbled {
        second.y
    } else if !first_bubbled && second_bubbled {
        sim.y
    } else {
        ((u32::from(sim.y) + u32::from(second.y)) / 2) as u16
    };
    cam.update_player_pan(
        pan_y,
        first_bubbled && second_bubbled,
        !frozen && !sim.paused && !sim.exit_phase,
    );

    let first_out = first_bubbled || sim.dead == 0xFF;
    let second_out = second_bubbled || second.dead == 0xFF;
    if let Some(tables) = obj_tables {
        if !frozen && !(first_out && second_out) {
            let primary = if first_out { 1 } else { 0 };
            let primary_sim = if primary == 1 {
                &mut *second
            } else {
                &mut *sim
            };
            if let Some(definition) =
                prepare_object_step(primary_sim, cam, obj, manifest.level_id, div)
            {
                result.boss_definition = Some(definition);
                primary_sim.fade_timer = primary_sim.fade_timer.saturating_sub(BOSS_LOAD_VBLANKS);
            }
            let first_contact = if first_out || sim.dead != 0 {
                None
            } else {
                let contact =
                    super::super::build_contact(sim, &input.held, cam.scroll_pan, cam.scene_state);
                (!contact.travelling).then_some(contact)
            };
            let second_contact = if second_out || second.dead != 0 {
                None
            } else {
                let contact = super::super::build_contact(
                    second,
                    &second_input.held,
                    cam.scroll_pan,
                    cam.scene_state,
                );
                (!contact.travelling).then_some(contact)
            };
            second.layout_writes.clone_from(&sim.layout_writes);
            obj.step_two_players(
                cam.x,
                cam.y,
                layout_live,
                tables,
                [&mut *sim, &mut *second],
                [first_contact, second_contact],
                [input.held.a, second_input.held.a],
                cam.boss_mode(),
                primary,
            );
            cam.apply_object_result(obj);
            result.active_objects = obj
                .slots
                .iter()
                .any(|slot| slot.state() == sml2_core::objects::SlotState::Active);
        }
    }
    // Constrain both live players to the shared presentation vertically, just
    // as `$0FBF` constrains X. Crossing the bottom is the ROM's death line.
    if !frozen {
        if !first_bubbled {
            clamp_shared_vertical(sim, cam);
        }
        if !second_bubbled {
            clamp_shared_vertical(second, cam);
        }
    }
    let scene_is_second = first_out && !second_bubbled;
    let (stepped, scene_dx, scene_dy, scene_index) = {
        let scene_sim = if scene_is_second {
            &mut *second
        } else {
            &mut *sim
        };
        let scene_start = (scene_sim.x, scene_sim.y);
        let stepped = step_level_scene(
            scene_sim,
            cam,
            scene_vm,
            scripts,
            ending_frame,
            bg_live,
            spr.as_mut(),
        );
        (
            stepped,
            scene_sim.x.wrapping_sub(scene_start.0),
            scene_sim.y.wrapping_sub(scene_start.1),
            scene_sim.level_gfx_index,
        )
    };
    // Scene scripts move the primary player directly (`$20A4` page warps).
    // Apply the same delta to the other live player so a scene transition
    // carries both.
    if scene_dx != 0 || scene_dy != 0 {
        if scene_is_second {
            if !first_bubbled {
                sim.x = sim.x.wrapping_add(scene_dx);
                sim.y = sim.y.wrapping_add(scene_dy);
            }
        } else if !second_bubbled {
            second.x = second.x.wrapping_add(scene_dx);
            second.y = second.y.wrapping_add(scene_dy);
        }
    }
    if let SceneStep::Ran(op) = stepped {
        if op >> 4 == 0x6 {
            let index = scene_index;
            let applied = match spr.as_mut() {
                Some(s) => tilesets.apply(index, bg_live, &mut s.objblock, &mut manifest.metatiles),
                None => false,
            };
            let room_reloaded = if let Some(arena) = arena_layout {
                *layout_live = arena.clone();
                let scene_sim = if scene_is_second {
                    &mut *second
                } else {
                    &mut *sim
                };
                crate::session::enter_layout_world(scene_sim, cam, obj, layout_live, rom_tables);
                sim.layout_writes.clear();
                second.layout_writes.clear();
                sim.world = layout_live.world;
                second.world = layout_live.world;
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

    let first_frame = if first_bubbled {
        0
    } else {
        let held_sfx0 = sim.sfx0;
        let f = sim.frame_index(sim.walk_timer);
        if frozen || held_sfx0 != 0 {
            sim.sfx0 = held_sfx0;
        }
        f
    };
    let second_frame = if second_bubbled {
        0
    } else {
        let held_sfx0 = second.sfx0;
        let f = second.frame_index(second.walk_timer);
        if second_frozen || held_sfx0 != 0 {
            second.sfx0 = held_sfx0;
        }
        f
    };
    if gameplay && !first_bubbled {
        sim.block_update();
    }
    if gameplay && !second_bubbled {
        std::mem::swap(&mut sim.layout_writes, &mut second.layout_writes);
        second.block_update();
        std::mem::swap(&mut sim.layout_writes, &mut second.layout_writes);
    }
    second.layout_writes.clone_from(&sim.layout_writes);
    let streak = if gameplay && (!first_bubbled || !second_bubbled) {
        obj.tick_stomp_bounce_timer()
    } else {
        None
    };

    if let Some(s) = spr.as_mut() {
        s.player = (!first_bubbled).then_some((sim.x, sim.y, first_frame));
        s.player_bubble = first_bubble;
        s.player_two = (!second_bubbled).then_some((second.x, second.y, second_frame));
        s.player_two_bubble = second_bubble;
        s.player_two_behind_bg = player_behind_bg(second.pipe_dir);
        s.streak_counter = streak;
        s.player_star_flash = !first_bubbled && sim.star_flash();
        s.player_two_star_flash = !second_bubbled && second.star_flash();
        s.player_shield = if first_bubbled {
            None
        } else {
            sim.moon_shield_frame().map(|sf| (sim.x, sim.y, sf))
        };
        s.player_two_shield = if second_bubbled {
            None
        } else {
            second
                .moon_shield_frame()
                .map(|sf| (second.x, second.y, sf))
        };
        s.player_pan = cam.player_pan;
        s.scroll_pan = cam.scroll_pan;
        s.bumpers.clear();
        if !first_bubbled {
            s.bumpers.extend_from_slice(&sim.block_draw);
        }
        if !second_bubbled {
            s.bumpers.extend_from_slice(&second.block_draw);
        }
        s.fireballs.clear();
        if !first_bubbled {
            s.fireballs.extend_from_slice(&sim.fire_draw);
        }
        if !second_bubbled {
            s.fireballs.extend_from_slice(&second.fire_draw);
        }
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
        if input.pressed.start || second_input.pressed.start {
            if sim.paused {
                sim.pause_exit();
            } else {
                sim.pause_enter();
            }
        } else if sim.paused && (input.pressed.select || second_input.pressed.select) {
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
