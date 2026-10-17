//! Wario 20's live timer input and camera-impact producer/consumer paths.

use crate::objects::{
    LevelLayout, ObjectSim, ObjectTables, B_DEF, B_FSTATE, B_PHASE, B_SCRX, B_SCRY, B_SPRITE,
    B_T24, B_T28,
};
use crate::{
    camera::CameraSim,
    level_timer::LevelTimer,
    player_sim::{PlayerKeys, PlayerSim},
};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn layout() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}

fn wario(state: u8) -> ObjectSim {
    let mut obj = ObjectSim::new(vec![], 0);
    obj.boss_def_index = 6;
    obj.level_id = 0x18;
    obj.slots[0].b[B_DEF] = 20;
    obj.slots[0].b[B_FSTATE] = state;
    obj.slots[0].b[B_SPRITE] = 0x4F;
    obj
}

#[test]
fn handler_0x0a8ab_impact_events_drive_next_frame_camera_and_player_guards() {
    let tables = tables();
    assert_eq!(tables.boss_anim_handler(20), Some(0xA8AB));
    for (state, phase, seed, wait) in [(3, 2, 0x34, 0x30), (5, 20, 0x38, 0x38)] {
        let mut obj = wario(state);
        obj.slots[0].b[B_T24] = 1;
        obj.slots[0].b[B_PHASE] = phase;
        obj.slots[0].b[B_SCRY] = 0x89;
        let mut sim = PlayerSim::new();
        sim.x = 100;
        sim.y = 200;
        let mut cam = CameraSim::new(80, 72);
        cam.update_player_pan(72, false, true);
        let creation_phases = (cam.scroll_pan, cam.player_pan, cam.scroll_y);
        obj.sample_player_inputs(&sim);
        obj.boss_engine(80, 72, &layout(), &tables);
        assert_eq!(
            (
                obj.slots[0].b[B_FSTATE],
                obj.slots[0].b[B_T28],
                obj.boss_transition,
                obj.sfx0
            ),
            (9, wait, 1, 0x18)
        );
        assert_eq!(obj.camera_pan_set, Some(seed));
        sim.apply_object_result(&mut obj, false);
        cam.apply_object_result(&mut obj);
        assert_eq!(sim.sfx0, 0x18);
        assert_eq!(cam.pan_timer, seed);
        assert_eq!(
            (cam.scroll_pan, cam.player_pan, cam.scroll_y),
            creation_phases
        );
        assert_eq!(obj.camera_pan_set, None);
        // Runtime snapshots this new timer before the next frame's player step.
        sim.pan_timer = cam.pan_timer;
        sim.on_ground_obj = true;
        sim.dir_right = false;
        sim.step(
            &PlayerKeys {
                a: true,
                right: true,
                b: true,
                ..Default::default()
            },
            true,
            false,
            &layout(),
        );
        assert_eq!((sim.hvel, sim.vvel, sim.pan_timer), (0x80, 24, seed));
        assert!(sim.on_ground);
        assert!(!sim.jump_flag);
        assert!(!sim.dir_right);
        cam.update_player_pan(sim.y, false, true);
        assert_eq!(
            (cam.scroll_pan, cam.player_pan, cam.pan_timer),
            (seed & 3, seed.wrapping_sub(1) & 3, seed - 1)
        );
        assert!(obj.unported.is_empty());
    }
}

#[test]
fn handler_0x0a8ab_samples_live_bcd_timer_before_walking_branch() {
    let tables = tables();
    for (before, low, state, x) in [(0x11, 0x10, 5, 0x40), (0x12, 0x11, 4, 0x3E)] {
        let mut sim = PlayerSim::new();
        sim.level_timer = LevelTimer {
            high: 0,
            low: before,
            tick_counter: 1,
        };
        sim.update_level_timer();
        assert_eq!(sim.level_timer.low, low);
        let mut obj = wario(4);
        obj.slots[0].b[B_SCRX] = 0x40;
        obj.sample_player_inputs(&sim);
        assert_eq!(obj.timer_low, low);
        obj.boss_engine(80, 72, &layout(), &tables);
        assert_eq!(
            (obj.slots[0].b[B_FSTATE], obj.slots[0].b[B_SCRX]),
            (state, x)
        );
        assert!(obj.unported.is_empty());
    }
}

#[test]
fn handler_0x0a8ab_samples_player_owned_bytes_and_clears_boss_latches_on_reload() {
    let mut sim = PlayerSim::new();
    sim.dead = 0x80;
    sim.fade_timer = 0x1234;
    sim.level_timer.low = 0x57;
    let mut obj = wario(0);
    obj.sample_player_inputs(&sim);
    assert_eq!(
        (obj.mario_dead, obj.fade_timer, obj.timer_low),
        (0x80, 0x1234, 0x57)
    );
    obj.boss_transition = 1;
    obj.ending_door_active = 1;
    obj.reset_level_start();
    assert_eq!((obj.boss_transition, obj.ending_door_active), (0, 0));
}

#[test]
fn handler_0x0a8ab_phase_exit_releases_scene_through_actual_dispatch() {
    let tables = tables();
    let mut obj = wario(8);
    obj.slots[0].b[B_SCRX] = 0xBE;
    let mut sim = PlayerSim::new();
    sim.fade_timer = 0x1255;
    let mut cam = CameraSim::new(80, 72);
    cam.scene_state = 0xF0;
    cam.boss_fight_phase = 0;
    obj.sample_player_inputs(&sim);
    obj.boss_engine(80, 72, &layout(), &tables);
    sim.apply_object_result(&mut obj, false);
    cam.apply_object_result(&mut obj);
    assert_eq!(sim.fade_timer, 0x1205);
    assert_eq!((cam.scene_state, cam.boss_fight_phase), (0, 1));
    assert!(!cam.boss_mode());
    assert_eq!(obj.records[0].raw, [7, 0xC8, 0, 0, 0xB0, 0x1D]);
    assert_eq!(obj.slots[0].b, [0; 32]);
    assert!(obj.unported.is_empty());
}
