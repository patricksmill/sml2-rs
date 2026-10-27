use super::*;
use crate::objects::{
    B_ANIMFRAME, B_DEF, B_FLASH, B_MARKER, B_SCRX, B_SPRITE, B_STATE, B_XHI, B_XLO, B_YHI, B_YLO,
};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn original_record() -> SpawnRecord {
    SpawnRecord {
        x: 0x128,
        y: 0x90,
        anim_index: 3,
        mode_flag: 1,
        sprite_id: 0x40,
        hitbox: [-1, 2, -3, 4],
        raw: [1, 0x28, 1, 0, 0x90, 3],
    }
}

#[test]
fn helper_0x0ac55_native_cadence_cycle_flash_and_wrapping_bytes() {
    // Independently calculated from $B356's FE 01 FF 02, $B40E's XOR $10,
    // and $AC60's byte addition. Every non-fourth frame skips both helpers.
    for frame in 0..=255u8 {
        for (phase, sprite, next_phase, next_sprite) in [
            (0, 0xFF, 1, 0),
            (1, 0, 2, 0xFF),
            (2, 0xFF, 3, 1),
            (3, 0, 0, 0xFE),
        ] {
            for x in [0xBDu8, 0xFE, 0xFF] {
                let mut bytes = [0xA5; 32];
                bytes[B_ANIMFRAME] = phase;
                bytes[B_SPRITE] = sprite;
                bytes[B_SCRX] = x;
                let mut expected = bytes;
                expected[B_SCRX] = x.wrapping_add(2);
                if frame & 3 == 0 {
                    expected[B_ANIMFRAME] = next_phase;
                    expected[B_SPRITE] = next_sprite;
                    expected[B_FLASH] = 0xB5;
                }
                let mut actor = Actor::from_raw_slot(&bytes);
                assert_eq!(
                    update_exit(&mut actor, FrameTick::new(u64::from(frame)), 0).unwrap(),
                    ExitEffects {
                        replacement_animation: None
                    }
                );
                assert_eq!(
                    actor.to_raw_slot(),
                    expected,
                    "frame {frame}, phase {phase}, X {x}"
                );
            }
        }
    }
}

#[test]
fn helper_0x0ac55_native_clear_boundary_and_difficulty_bit_zero() {
    for flags in [0, 1, 2, 0x80, 0xFE, 0xFF] {
        for x in [0xBE, 0xBF, 0xC0, 0xFD] {
            let mut bytes = [0xA5; 32];
            bytes[B_ANIMFRAME] = 3;
            bytes[B_SCRX] = x;
            let mut actor = Actor::from_raw_slot(&bytes);
            let effects = update_exit(&mut actor, FrameTick::new(4), flags).unwrap();
            assert_eq!(actor.to_raw_slot(), [0; 32]);
            assert_eq!(
                effects.replacement_animation,
                Some(if flags & 1 == 0 { 27 } else { 29 })
            );
        }
    }
}

#[test]
fn helper_0x0ac55_cycle_precedes_flash_and_movement() {
    let mut bytes = [0xA5; 32];
    bytes[B_ANIMFRAME] = 4; // The selected table has no entry for advanced phase 5.
    bytes[B_SCRX] = 0xBE;
    let mut actor = Actor::from_raw_slot(&bytes);
    assert_eq!(
        update_exit(&mut actor, FrameTick::new(0), 0),
        Err(AnimationFrameError { frame: 5 })
    );
    bytes[B_ANIMFRAME] = 5;
    assert_eq!(actor.to_raw_slot(), bytes);
}

#[test]
fn helper_0x0ac55_adapter_no_exit_preserves_stream_count_and_other_state() {
    let tables = tables();
    let original = original_record();
    for x in [0xBDu8, 0xFE, 0xFF] {
        let mut sim = ObjectSim::new(vec![original, original], 0);
        sim.stream_index = 6;
        sim.spawned = vec![true, true];
        sim.sprite_map_index = 9;
        sim.scene_phase_set = Some(7);
        sim.boss_def_count = 3;
        sim.fade_timer = 0xABCD;
        sim.fff0 = 3;
        let mut st = Staging::default();
        st.b = [0xA5; 32];
        st.b[B_ANIMFRAME] = 0;
        st.b[B_SCRX] = x;
        let mut expected = st.b;
        expected[B_SCRX] = x.wrapping_add(2);
        sim.wario_phase2_exit(&mut st, &tables);
        assert_eq!(st.b, expected);
        assert_eq!((sim.fade_timer, sim.fade_timer_set), (0xAB05, Some(0xAB05)));
        assert_eq!(
            (
                sim.done,
                sim.boss_def_count,
                sim.sprite_map_index,
                sim.scene_phase_set,
                sim.stream_index
            ),
            (0, 3, 9, Some(7), 6)
        );
        assert_eq!(sim.records, vec![original, original]);
        assert_eq!(sim.spawned, vec![true, true]);
    }
}

#[test]
fn helper_0x0ac55_adapter_exact_stream_and_real_consumers_then_reload() {
    let tables = tables();
    for (easy, animation) in [(false, 27), (true, 29)] {
        let original = original_record();
        let mut sim = ObjectSim::new(vec![original, original], 0);
        sim.level_id = 0x18;
        sim.stream_index = 6;
        sim.spawned = vec![true, true];
        sim.boss_def_count = 0x80;
        sim.fade_timer = 0x3401;
        let mut camera = crate::camera::CameraSim::new(0x0880, 0);
        camera.scene_state = 0xF0;
        camera.boss_fight_phase = 1;
        sim.easy_mode = easy;
        let mut st = Staging::default();
        st.b = [0xA5; 32];
        st.b[B_ANIMFRAME] = 3;
        st.b[B_SCRX] = 0xBE;
        sim.wario_phase2_exit(&mut st, &tables);
        assert_eq!(st.b, [0; 32]);
        assert_eq!(
            (
                sim.done,
                sim.boss_def_count,
                sim.sprite_map_index,
                sim.stream_index
            ),
            (1, 0x80, 1, 0)
        );
        assert_eq!((sim.fade_timer, sim.fade_timer_set), (0x3405, Some(0x3405)));
        assert_eq!(sim.scene_phase_set, Some(2));
        assert_eq!(sim.spawned, vec![false]);
        assert_eq!(sim.records.len(), 1);
        let record = sim.records[0];
        assert_eq!(
            (record.x, record.y, record.anim_index, record.mode_flag),
            (2248, 176, animation, 0)
        );
        assert_eq!(record.raw, [8, 0xC8, 0, 0, 0xB0, animation]);
        let sprite = tables.sprite_for(0x0A, animation).unwrap();
        assert_eq!(
            (record.sprite_id, record.hitbox),
            (sprite, tables.hitbox_for(0x0A, sprite).unwrap())
        );
        camera.apply_object_result(&mut sim);
        assert_eq!((camera.scene_state, camera.boss_fight_phase), (0, 2));
        assert!(!camera.boss_mode());
        assert_eq!(sim.scene_phase_set, None);
        camera.scene_state = 0xA0;
        camera.boss_fight_phase = 9;
        camera.apply_object_result(&mut sim);
        assert_eq!((camera.scene_state, camera.boss_fight_phase), (0xA0, 9));
        sim.scan_dir = 1;
        sim.spawn_x = 0x08C8;
        sim.scan(&tables);
        let bytes = &sim.slots[0].b;
        assert_eq!(
            [
                bytes[B_XHI],
                bytes[B_XLO],
                bytes[B_MARKER],
                bytes[B_YHI],
                bytes[B_YLO],
                bytes[B_DEF]
            ],
            [8, 0xC8, 0, 0, 0xB0, animation]
        );
        assert_eq!(bytes[B_STATE], 1);
        assert_eq!(sim.spawned, vec![true]);
        assert!(sim.unported.is_empty());
        sim.scene_phase_set = Some(2);
        sim.reset_level_start();
        assert_eq!(sim.records, vec![original, original]);
        assert_eq!(sim.spawned, vec![false, false]);
        assert_eq!(sim.stream_index, 0);
        assert_eq!(sim.scene_phase_set, None);
    }
}
