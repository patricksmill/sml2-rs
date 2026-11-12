//! Ordered gameplay phases shared by native hosts and capture replay.

mod live;
pub(crate) use live::{
    LiveLevelFrame, begin_live_level_frame, step_live_level_frame, step_two_player_live_level_frame,
};

use sml2_core::{
    camera::CameraSim,
    ending_entry::{EndingFrame, EndingInput},
    ending_scene::EndingScene,
    ending_sequence::{EndingSequenceController, EndingStepContext},
    level::LevelLayout,
    object::ObjectTables,
    objects::{FireHit, MarioContact, ObjectSim, SlotState},
    player_sim::{PlayerKeys, PlayerSim},
    scene::{SceneScripts, SceneStep, SceneVm, boss_def_index, boss_def_portable},
};

pub(crate) struct FloatStep {
    pub timer_before: u8,
    pub timer_after: u8,
    pub vvel_before: u8,
    pub vvel_after: u8,
}

/// `$0F71`'s cross-engine tail precedes camera scrolling and the object pass.
pub(crate) fn resolve_player_events(sim: &mut PlayerSim, cam: &CameraSim, obj: &mut ObjectSim) {
    if sim.take_stomp_chain_reset() {
        obj.stomp_chain = 0;
    }
    // `$2F72` uses the previous frame's latched `$2062` scroll bytes.
    if let Some(point) = sim.below_bonk.take() {
        let special = obj.below_bonk(point, cam.scroll_y, cam.scroll_x);
        sim.resolve_bump_coin(special);
    }
    if let Some(point) = sim.fire_below_bonk.take() {
        obj.below_bonk(point, cam.scroll_y, cam.scroll_x);
    }
    for i in 0..sim.fire_scan.len() {
        let Some((sy, sx)) = sim.fire_scan[i].take() else {
            continue;
        };
        let hit = obj.fire_hit(sy, sx, sim.fire_position(i));
        if hit == FireHit::Die {
            sim.sfx0 = 2;
        }
        sim.resolve_fire_hit(i, hit != FireHit::None);
    }
}

/// Alive, unfrozen `$0F71`. Hosts own death/fade dispatch.
pub(crate) fn step_player(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    obj: &mut ObjectSim,
    layout: &LevelLayout,
    input: EndingInput,
) -> Option<FloatStep> {
    let mut float_use = None;
    if cam.transition == 0 {
        // Crouch-look reads ground flags from the preceding frame.
        cam.crouch_pan(sim, &input.held);
        let float_before =
            (!sim.on_ground && sim.moon & 0x08 != 0 && input.held.a && sim.moon_float_timer != 0)
                .then_some((sim.moon_float_timer, sim.vvel));
        sim.step(&input.held, input.pressed.a, input.pressed.b, layout);
        if let Some((timer, vvel)) = float_before {
            if sim.moon_float_timer == timer - 1 {
                float_use = Some(FloatStep {
                    timer_before: timer,
                    timer_after: sim.moon_float_timer,
                    vvel_before: vvel,
                    vvel_after: sim.vvel,
                });
            }
        }
    } else {
        sim.tick_fade_timer();
    }
    resolve_player_events(sim, cam, obj);
    float_use
}

/// `$0B2A` follows player events; replay installs its captured edge in between.
pub(crate) fn step_camera(sim: &mut PlayerSim, cam: &mut CameraSim) -> bool {
    let transition_before = cam.transition;
    cam.step(sim);
    cam.update_enables(sim.x);
    transition_before != 0 && cam.transition == 0
}

/// Keep a live player inside the shared presentation vertically. The native
/// view is the camera-centred 160x144 rect: its top edge is the ROM's
/// `y - cam + 80 = 16`, and the bottom kill line is the death arc's
/// `y - cam + 98 = $B0`. Enlarged views use their own world rect. A player
/// may rise [`ABOVE_TOP`] past the top, as NSMBW lets players leave the top
/// of the screen; that reach is also what an up-lock transition needs
/// (`sMarioScreenY < $1B`). Crossing the bottom is the normal death line
/// (bubble in co-op), so pits stay lethal.
pub(crate) fn clamp_shared_vertical(sim: &mut PlayerSim, cam: &CameraSim) {
    if sim.dead == 0xFF || sim.exit_phase || sim.exit_completed || sim.pipe_dir >= 0x10 {
        return;
    }
    let (top, bottom) = match sim.view {
        Some(view) => (view.y as u16, view.h),
        // 64 above the camera centre to the 6-px-below-bottom death line.
        None => (cam.y.wrapping_sub(64), 142),
    };
    let rel = sim.y.wrapping_sub(top) as i16 as i32;
    if rel < -ABOVE_TOP {
        sim.y = top.wrapping_sub(ABOVE_TOP as u16);
        sim.vvel = 24;
        sim.on_ground = false;
        return;
    }
    if rel >= bottom {
        sim.dead = 0xFF;
        sim.dead_fade = false;
        sim.fade_timer = 0;
        sim.powerup = 0;
        sim.invincibility = 0;
    }
}

/// Pixels a player may rise above the shared view's top edge.
const ABOVE_TOP: i32 = 32;

/// One player's contact with the shared screen's side edges this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScreenEdge {
    pub pressing_left: bool,
    pub pressing_right: bool,
    /// Left behind more than 16 px past an edge: NSMBW bubbles the player
    /// (`daPlBase_c::checkDisplayOutDead`, multiplayer offset 16).
    pub off_screen: bool,
}

/// How far past a side edge a pinned player may fall behind before bubbling.
const OFF_SCREEN_MARGIN: i32 = 16;

/// NSMBW's `daPlBase_c::checkDispSideLemit` on the frame-start view: a player
/// past a side edge is put back on it and loses speed into it, unless a wall
/// stands on the player's inward side, in which case the edge does not push
/// the player into the wall and the player may drift off-screen. The span is
/// the ROM's `$10..=$A0` screen X on the native view, or the enlarged view
/// rect. Pressing reports walking into an edge, which starts the room
/// transition at a scroll lock.
pub(crate) fn apply_screen_edges(
    sim: &mut PlayerSim,
    layout: &LevelLayout,
    held: &PlayerKeys,
) -> ScreenEdge {
    let mut out = ScreenEdge::default();
    if sim.dead != 0 || sim.exit_phase || sim.exit_completed || sim.pipe_dir >= 0x10 {
        return out;
    }
    let (left, width) = match sim.view {
        Some(view) => (view.x as u16, view.w - 16),
        None => (sim.camx.wrapping_sub(96).wrapping_add(0x10), 0xA0 - 0x10),
    };
    let rel = i32::from(sim.x.wrapping_sub(left) as i16);
    if rel <= 0 {
        out.pressing_left = held.left;
        if rel < 0 {
            if sim.wall_beside(layout, true) {
                out.off_screen = rel < -OFF_SCREEN_MARGIN;
            } else {
                sim.x = left;
                if !sim.dir_right {
                    sim.hvel = 0;
                }
            }
        }
    } else if rel >= width {
        out.pressing_right = held.right;
        if rel > width {
            if sim.wall_beside(layout, false) {
                out.off_screen = rel > width + OFF_SCREEN_MARGIN;
            } else {
                sim.x = left.wrapping_add(width as u16);
                if sim.dir_right {
                    sim.hvel = 0;
                }
            }
        }
    }
    out
}

/// Sample the player after the host's observation checkpoint, then load a boss.
/// Hosts apply their load-duration compensation before the object pass.
pub(crate) fn prepare_object_step(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    obj: &mut ObjectSim,
    level_id: u8,
    div: u8,
) -> Option<u8> {
    super::sample_level_player_inputs(obj, sim);
    obj.div = div;
    if !cam.scene_load_pending {
        return None;
    }
    let definition = boss_def_index(level_id, cam.boss_fight_phase);
    if boss_def_portable(definition) {
        if let Some((music, level_song)) = cam.begin_scene_load(level_id) {
            sim.music_request = music;
            sim.level_song = level_song;
            obj.spawn_boss(definition);
            return Some(definition);
        }
    } else if cam.take_blocked_scene_load() {
        obj.note_blocked_scene_load();
    }
    None
}

/// `$0652`: use the contact captured before boss loading (and replay's pose
/// checkpoint), then write the object engine's results to both state owners.
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_objects(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    obj: &mut ObjectSim,
    layout: &LevelLayout,
    tables: &ObjectTables,
    contact: &MarioContact,
    a_held: bool,
) -> bool {
    obj.sample_camera_inputs(cam);
    obj.step(cam.x, cam.y, layout, tables, contact, cam.boss_mode());
    let active = obj
        .slots
        .iter()
        .any(|slot| slot.state() == SlotState::Active);
    sim.apply_object_result(obj, a_held);
    cam.apply_object_result(obj);
    active
}

pub(crate) fn step_ending_sequence(
    sim: &mut PlayerSim,
    sequence: &mut EndingSequenceController,
    scene: &mut EndingScene,
    credits_text: Option<&[u8]>,
    object_tables: Option<&ObjectTables>,
) {
    let result = sequence.step(&mut EndingStepContext {
        scene,
        rom: None,
        credits_text,
        object_tables,
        decrement_fade: true,
    });
    if result.sfx_request_0 != 0 {
        sim.sfx0 = result.sfx_request_0;
    }
    if result.sfx_request_1 != 0 {
        sim.sfx1 = result.sfx_request_1;
    }
}

/// Resume `$20A4` and publish its DMA and pipe background in the same phase.
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_level_scene(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    scene: &mut SceneVm,
    scripts: &SceneScripts,
    ending_frame: Option<EndingFrame>,
    bg: &mut [u8],
    mut sprites: Option<&mut crate::render::SpriteCtx>,
) -> SceneStep {
    let scene_camera = (cam.x, cam.y);
    let stepped =
        if matches!(ending_frame, Some(EndingFrame::Gameplay(_))) || ending_frame.is_none() {
            scene.step(sim, cam, scripts)
        } else {
            SceneStep::Idle
        };
    super::apply_scene_transfer(scene, bg, sprites.as_deref_mut());
    if let Some(sprites) = sprites {
        crate::render::PipeBackground::update(
            &mut sprites.pipe_background,
            scene_camera,
            cam,
            stepped,
        );
    }
    stepped
}
