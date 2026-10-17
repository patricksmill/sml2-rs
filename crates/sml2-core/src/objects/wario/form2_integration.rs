//! Wario 23's live timer input and camera-impact producer/consumer paths.

use crate::objects::{
    LevelLayout, ObjectSim, ObjectTables, Staging, B_DEF, B_FLAGS, B_FLIP, B_FSTATE, B_PHASE,
    B_SCRX, B_SCRY, B_SPRITE, B_T28, B_TYPE,
};
use crate::{
    camera::CameraSim,
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
    obj.slots[0].b[B_DEF] = 23;
    obj.slots[0].b[B_FSTATE] = state;
    obj.slots[0].b[B_SPRITE] = 0x4F;
    obj
}

#[test]
fn handler_0x0ab8b_impact_events_drive_next_frame_camera_and_player_guards() {
    let tables = tables();
    assert_eq!(tables.boss_anim_handler(23), Some(0xAB8B));
    for (state, phase, seed, wait) in [(10, 2, 0x38, 0x30)] {
        let mut obj = wario(state);
        obj.slots[0].b[B_T28] = 0;
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
fn handler_0x0ab8b_spawn_child_fall_latch_parent_intro_and_reset() {
    let tables = tables();
    let mut obj = wario(0);
    obj.spawn_attr = 0x93;
    obj.slots[0].b[B_SCRX] = 80;
    obj.slots[0].b[B_SCRY] = 138;
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!((obj.slots[0].b[B_FSTATE], obj.slots[0].b[B_T28]), (7, 32));
    // Driver snapshots its count: newly appended child starts next frame.
    assert_eq!(obj.boss_def_count, 1);
    assert_eq!(
        (
            obj.slots[1].b[B_DEF],
            obj.slots[1].b[B_SPRITE],
            obj.slots[1].b[B_FLIP]
        ),
        (22, 14, 0x93)
    );
    assert_eq!(
        (obj.spawn_anim, obj.spawn_sprite, obj.spawn_attr),
        (0, 0, 0)
    );
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!((obj.slots[1].b[B_FSTATE], obj.slots[1].b[B_SCRY]), (1, 16));
    obj.slots[1].b[B_SCRY] = 0x5f;
    obj.fff0 = 3;
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!(obj.slots[1].b, [0; 32]);
    assert_eq!(obj.boss_delay, 1);
    assert_eq!(obj.boss_def_count, 0);
    assert_eq!(obj.slots[0].b[B_T28], 32);
    for _ in 0..32 {
        obj.boss_engine(80, 72, &layout(), &tables);
    }
    assert_eq!(
        (
            obj.slots[0].b[B_FSTATE],
            obj.slots[0].b[B_SPRITE],
            obj.boss_delay
        ),
        (11, 114, 0)
    );
    obj.fade_timer = 97;
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!((obj.ending_door_active, obj.sfx1), (1, 1));
    obj.fade_timer = 0;
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!(
        (
            obj.slots[0].b[B_FSTATE],
            obj.slots[0].b[B_TYPE],
            obj.ending_door_active
        ),
        (3, 65, 0)
    );
    assert!(obj.unported.is_empty());
    obj.boss_transition = 1;
    obj.reset_level_start();
    assert_eq!(
        (obj.boss_transition, obj.ending_door_active, obj.boss_delay),
        (0, 0, 0)
    );
}
#[test]
fn handler_0x0ab8b_real_hit_recovery_defeat_and_dead_freeze() {
    let tables = tables();
    for (flags, hits, remaining, sprite, timer) in
        [(1, 4, 2, 82, 17), (2, 4, 3, 79, 9), (1, 1, 255, 82, 17)]
    {
        let mut obj = wario(5);
        obj.boss_hits_remaining = hits;
        obj.boss_timer_reload = 17;
        obj.slots[0].b[B_FLAGS] = flags;
        obj.slots[0].b[B_T28] = 9;
        obj.slots[0].b[B_TYPE] = 65;
        obj.boss_engine(80, 72, &layout(), &tables);
        assert_eq!(obj.boss_hits_remaining, remaining);
        assert_eq!(
            (obj.saved_frame_state, obj.saved_type, obj.spawn_sprite),
            (5, 65, 79)
        );
        assert_eq!(
            (
                obj.slots[0].b[B_FSTATE],
                obj.slots[0].b[B_SPRITE],
                obj.slots[0].b[B_T28]
            ),
            (1, sprite, timer)
        );
        obj.slots[0].b[B_FLAGS] = 0;
        obj.slots[0].b[B_T28] = 1;
        obj.boss_engine(80, 72, &layout(), &tables);
        assert_eq!(obj.slots[0].b[B_FSTATE], if hits == 1 { 2 } else { 5 });
        if hits != 1 {
            assert_eq!((obj.slots[0].b[B_SPRITE], obj.slots[0].b[B_TYPE]), (79, 65));
        }
        assert!(obj.unported.is_empty());
    }
    let mut obj = wario(10);
    obj.mario.dead = 255;
    let mut before = obj.slots[0].b;
    // The frozen handler still passes through the ROM sprite79 hitbox refresh.
    before[19..23].copy_from_slice(&[0xe3, 0xfd, 0xf6, 0x0a]);
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!(obj.slots[0].b, before);
}
#[test]
fn handler_0x0ab8b_phase_exit_actual_stream_and_scene_handoff() {
    let tables = tables();
    for (easy, animation) in [(false, 27), (true, 29)] {
        let mut obj = wario(8);
        obj.easy_mode = easy;
        obj.slots[0].b[B_SCRX] = 190;
        let mut player = PlayerSim::new();
        player.fade_timer = 0x1255;
        let mut camera = CameraSim::new(80, 72);
        camera.scene_state = 0xf0;
        obj.sample_player_inputs(&player);
        obj.boss_engine(80, 72, &layout(), &tables);
        player.apply_object_result(&mut obj, false);
        camera.apply_object_result(&mut obj);
        assert_eq!(player.fade_timer, 0x1205);
        assert_eq!(obj.slots[0].b, [0; 32]);
        assert_eq!(obj.records[0].raw, [8, 0xc8, 0, 0, 0xb0, animation]);
        assert_eq!((camera.scene_state, camera.boss_fight_phase), (0, 2));
        assert!(!camera.boss_mode());
        assert!(obj.unported.is_empty());
    }
}
#[test]
fn handler_0x0ab8b_live_projected_player_screen_coordinate_controls_facing_and_proximity() {
    let tables = tables();
    for (world_x, cam_x, screen, facing, state) in [
        (64, 80, 80, 1, 5),
        (63, 80, 79, 255, 4),
        (0, 353, 255, 1, 5),
    ] {
        let mut obj = wario(3);
        obj.slots[0].b[B_SCRX] = 80;
        obj.slots[0].b[B_PHASE] = 35;
        obj.mario.x = world_x;
        obj.boss_engine(cam_x, 72, &layout(), &tables);
        assert_eq!(
            (obj.player_screen_x, obj.facing, obj.slots[0].b[B_FSTATE]),
            (screen, facing, state)
        );
    }
    let mut obj = wario(4);
    obj.slots[0].b[B_SCRX] = 112;
    obj.slots[0].b[B_SCRY] = 100;
    obj.mario.x = 64;
    obj.fff0 = 7;
    obj.timer_low = 0;
    obj.boss_engine(80, 72, &layout(), &tables);
    assert_eq!((obj.player_screen_x, obj.slots[0].b[B_FSTATE]), (80, 10));
}
#[test]
fn handler_0x0ab8b_adapter_reports_unsupported_state_and_missing_profile() {
    let mut tables = tables();
    let mut obj = wario(12);
    let mut staging = Staging {
        b: obj.slots[0].b,
        ..Default::default()
    };
    obj.handler_wario23(&mut staging, &tables);
    assert_eq!(obj.unported, vec![0xAB8B]);
    obj.unported.clear();
    staging.b[B_FSTATE] = 10;
    staging.b[B_T28] = 0;
    tables.boss_picker_profile.clear();
    obj.handler_wario23(&mut staging, &tables);
    assert_eq!(obj.unported, vec![0xAB8B]);
}
