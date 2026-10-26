//! Ordered gameplay phases shared by native hosts and capture replay.

use sml2_core::{
    camera::CameraSim,
    ending_entry::EndingInput,
    level::LevelLayout,
    object::ObjectTables,
    objects::{FireHit, MarioContact, ObjectSim, SlotState},
    player_sim::PlayerSim,
    scene::{boss_def_index, boss_def_portable},
};

pub(crate) struct FloatStep {
    pub timer_before: u8,
    pub timer_after: u8,
    pub vvel_before: u8,
    pub vvel_after: u8,
}

pub(crate) struct PlayerStep {
    pub float_use: Option<FloatStep>,
    pub transition_finished: bool,
}

/// `$0F71`'s cross-engine tail precedes camera scrolling and the object pass.
pub(crate) fn resolve_player_events(sim: &mut PlayerSim, cam: &CameraSim, obj: &mut ObjectSim) {
    if sim.take_stomp_chain_reset() {
        obj.stomp_chain = 0;
    }
    // `$2F72` uses the previous frame's latched `$2062` scroll bytes.
    if let Some((by, bx)) = sim.below_bonk.take() {
        let special = obj.below_bonk(by, bx, cam.scroll_y, cam.scroll_x);
        sim.resolve_bump_coin(special);
    }
    if let Some((by, bx)) = sim.fire_below_bonk.take() {
        obj.below_bonk(by, bx, cam.scroll_y, cam.scroll_x);
    }
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
}

/// Alive, unfrozen `$0F71` -> `$0B2A`. Hosts own death/fade dispatch.
pub(crate) fn step_player_camera(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    obj: &mut ObjectSim,
    layout: &LevelLayout,
    input: EndingInput,
) -> PlayerStep {
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
    let transition_before = cam.transition;
    cam.step(sim);
    cam.update_enables(sim.x);
    PlayerStep {
        float_use,
        transition_finished: transition_before != 0 && cam.transition == 0,
    }
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
