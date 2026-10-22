//! Shared native frame-ordering helpers used by sessions and parity adapters.

use sml2_core::ending_entry::{EndingEntry, EndingEntryState, EndingFrame, EndingInput};
use sml2_core::level::LevelManifest;
use sml2_core::map::MapNodes;
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_core::scene::SceneVm;

/// `$EBBB` clears the level's slots and scratch; `$FFF0` survives the loader.
/// Cold startup (`$022A`) supplies zero, while session re-entry supplies its
/// persistent counter. Replay fixtures retain their own explicit seeds.
pub(crate) fn initialize_level_objects(
    objects: &mut sml2_core::objects::ObjectSim,
    persistent_counter: u8,
) {
    objects.fff0 = persistent_counter;
    // `$EC40` also clears the previous camera ($AF24/$AF25). Seeding it
    // from the new camera suppresses the first spawn scan at checkpoints,
    // shifting later actors into different slots and update phases.
    objects.reset_level_start();
}

/// The `MarioContact` both loops hand to the object engine (`$2D7D`'s view of
/// Mario). One constructor, because each loop used to build it by hand and the
/// play loop quietly passed defaults for `star_high`, `invincibility` and
/// `on_object` — a divergence no capture can show, since only `--play` had it
/// (the class queued as `infra_play_parity_harness`).
///
/// `pan` is `sCameraPanTimer` (`$A2B3`, the `$2062` scroll-Y nudge): the replay
/// path takes it from its capture, play mode has no source for it yet and
/// passes 0. `scene_state` is `sSceneState` (`CameraSim::scene_state`, which
/// the scene VM's `$2x` arms); `$2E5C`'s boss-scene bounce depends on it.
pub(crate) fn build_contact(
    sim: &PlayerSim,
    keys: &PlayerKeys,
    pan: u8,
    scene_state: u8,
) -> sml2_core::objects::MarioContact {
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
        scene_state,
        moon_physics: sim.moon,
        pan,
        travelling: sim.pipe_dir >= 0x10,
        hvel: sim.hvel,
    }
}

/// A hand-off whose tail selected a zone screen: the ROM's
/// `$3D0FF`/`$3D208`/`$3D371`/`$3D49F`/`$3D61F`/`$3D79A`/`$3D6EB`/`$3D880`
/// loaders are ported into `PlayerSim::map_handoff` and `MapSim::load_screen`.
pub(crate) fn note_handoff_screen(_out: &sml2_core::player_sim::MapHandoffOutcome) {}

#[allow(dead_code)]
pub(crate) struct BellArm {
    pub(crate) frame: u64,
    pub(crate) position: (u16, u16),
    pub(crate) fade_timer: u16,
    pub(crate) music_request: u8,
    pub(crate) exit_tile_flag: u8,
    pub(crate) dead: u8,
}

#[allow(dead_code)]
pub(crate) struct TimedFloatUse {
    pub(crate) frame: u64,
    pub(crate) timer_before: u8,
    pub(crate) timer_after: u8,
    pub(crate) vvel_before: u8,
    pub(crate) vvel_after: u8,
}

/// The dispatch boundary shared by live play, replay, and pipe integration tests.
pub(crate) fn scene_frame_head(
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
pub(crate) fn ending_frame_head(
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
pub(crate) fn sample_level_player_inputs(obj: &mut sml2_core::objects::ObjectSim, sim: &PlayerSim) {
    obj.sample_player_inputs(sim);
    obj.secret_route = sim.secret_route != 0;
}

/// Apply the resolved `$A800-$A807` block and `$5579` route/progress latches.
pub(crate) fn apply_level_entry(
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

/// Recover a direct level selection's node only when the source table is
/// unambiguous. Castle id 24 has no node, and id 17 occurs at nodes 5 and 26;
/// those entries retain their caller's node until overworld entry supplies it.
pub(crate) fn seed_unique_level_map_node(
    sim: &mut PlayerSim,
    level: u8,
    nodes: &MapNodes,
) -> Option<u8> {
    let mut matches =
        (0..MapNodes::NODES as u8).filter(|&node| nodes.level_for(node) == Some(level));
    let node = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    sim.map_node = node;
    Some(node)
}

/// `$04F1`: the header seeds the countdown and the level's music byte on load.
pub(crate) fn seed_level_timer(sim: &mut PlayerSim, manifest: &LevelManifest) {
    sim.level_timer = sml2_core::level_timer::LevelTimer::new(manifest.header.time_limit);
    sim.level_song = manifest.header.music_track;
    sim.tile_anim_config = manifest.tile_anim.table_value.wrapping_add(1);
}

/// `$3873`: animate live VRAM only while `sTileAnimConfig` is nonzero.
/// Disabling animation preserves the last write until a tileset reload.
pub(crate) fn update_tile_animation(
    sim: &PlayerSim,
    scene_active: bool,
    bg: &mut [u8],
    rows: &[u8],
    clock: &mut sml2_core::tile_animation::TileAnimationClock,
) {
    // `$01BF` is bypassed while `sSceneId != 0` (`$26C3` handles the
    // scene instead). In particular, `$6x` must not be overwritten before
    // the following `$2x` opcode disables animation permanently.
    if let Some(anim) = clock.tick(sim.tile_anim_config, scene_active) {
        crate::render::apply_tile_anim(bg, &anim, rows, clock.phase);
    }
}

/// `$05E4`: countdown precedes death dispatch, player physics, and camera motion.
/// The star tick is `$5D44`, ahead of `$5D6A`'s dead/goal guard, so it keeps
/// running through a death fade or a level exit; pause (`$FF9B` = 8) skips the
/// whole `$05D5` path.
pub(crate) fn update_level_timer_at_frame_head(sim: &mut PlayerSim) {
    if sim.paused {
        return;
    }
    sim.tick_star_timer();
    if !sim.dead_fade && !sim.exit_phase {
        sim.update_level_timer();
    }
}
