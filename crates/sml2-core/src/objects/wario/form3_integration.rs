//! Integration tests for Wario Form 3 (anim 26) through ObjectSim and CameraSim.

use crate::camera::CameraSim;
use crate::objects::{
    LevelLayout, ObjectSim, ObjectTables, B_DEF, B_FSTATE, B_SCRX, B_SPRITE, B_T28,
};
use crate::player_sim::PlayerSim;

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

#[test]
fn final_wario_anim_handlers_table_wiring() {
    let tables = tables();
    assert_eq!(
        tables.boss_anim_handler(26),
        Some(0xAD93),
        "Wario form 3 boss main body"
    );
    assert_eq!(
        tables.boss_anim_handler(27),
        Some(0xAFE0),
        "Wario trajectory child"
    );
    assert_eq!(
        tables.boss_anim_handler(28),
        Some(0xB0C2),
        "Wario drop child"
    );
}

#[test]
fn form3_spawn_and_defeat_ending_cascade_integration() {
    let tables = tables();
    let mut obj = ObjectSim::new(vec![], 0);
    obj.boss_def_index = 6;
    obj.level_id = 0x18;
    obj.slots[0].b[B_DEF] = 26;
    obj.slots[0].b[B_FSTATE] = 0; // initial spawn
    obj.slots[0].b[B_SPRITE] = 92;

    let sim = PlayerSim::new();
    let mut cam = CameraSim::new(80, 72);
    cam.scene_state = 0xF0; // parked arena

    obj.sample_player_inputs(&sim);
    obj.boss_engine(80, 72, &layout(), &tables);

    // After state 0 run: slot 0 moved to state 12, slot 1 has spawned child 25
    assert_eq!(obj.slots[0].b[B_FSTATE], 12);
    assert_eq!(obj.slots[1].b[B_DEF], 25, "spawned child 25");

    // Test defeat ending offscreen exit (state 10, x >= 192)
    obj.slots[0].b[B_FSTATE] = 10;
    obj.slots[0].b[B_T28] = 0; // timer expired
    obj.slots[0].b[B_SCRX] = 191;
    obj.fff0 = 1; // boss_engine increments to 2 (even frame) -> moves x by 3 to 194

    obj.boss_engine(80, 72, &layout(), &tables);

    // Slot 0 should be blanked (done)
    assert_eq!(obj.slots[0].b[B_DEF], 0, "slot 0 def cleared on exit");
    assert_eq!(obj.slots[0].b[B_SCRX], 0, "slot 0 screen x cleared on exit");
    assert_eq!(obj.scene_phase_set, Some(3), "scene_phase_set = Some(3)");

    // Camera consumes object result
    cam.apply_object_result(&mut obj);
    assert_eq!(cam.boss_fight_phase, 3, "boss fight phase latched to 3");
    assert_eq!(cam.scene_state, 0, "arena park cleared (scene_state = 0)");
}

#[test]
fn source_regression_jump_profile_is_saved_at_walk_transition() {
    use crate::objects::{Staging, B_PHASE};
    let tables = tables();
    // AF70..AF75: literal powerups 0/1/2/3 store FE/FF/00/01 in AFD1.
    for (power, saved, changed, terminal) in [
        (0, 0xFE, 2, 14),
        (1, 0xFF, 2, 14),
        (2, 0x00, 0, 26),
        (3, 0x01, 2, 14),
    ] {
        for walk_state in [4, 5] {
            let mut obj = ObjectSim::new(vec![], 0);
            assert_eq!(obj.saved_power_state, 0);
            obj.saved_power_state = 0x42;
            obj.fff0 = 1;
            obj.player_screen_x = 80;
            obj.mario.powerup = power;
            let mut st = Staging::default();
            st.b[B_FSTATE] = walk_state;
            st.b[B_SCRX] = 80;
            obj.handler_wario26(&mut st, &tables);
            assert_eq!(
                (st.b[B_FSTATE], obj.saved_power_state),
                (walk_state + 2, saved)
            );
            obj.mario.powerup = changed;
            for leg in [0, 1] {
                st.b[B_PHASE] = terminal;
                obj.handler_wario26(&mut st, &tables);
                assert_eq!((st.b[B_PHASE], st.b[24]), (0, leg ^ 1));
                assert_eq!(
                    obj.saved_power_state, saved,
                    "AFD1 persists across jump frames"
                );
                assert!(obj.unported.is_empty());
            }
            assert_eq!((st.b[B_FSTATE], st.b[B_SPRITE]), (3, 88));
        }
    }
}

#[test]
fn form3_dirty_ending_adapter_preserves_count_and_consumes_combined_event_once() {
    use crate::objects::{Staging, B_FLAGS};
    let mut obj = ObjectSim::new(vec![], 0);
    obj.boss_def_count = 7;
    obj.saved_power_state = 0xFE;
    obj.fff0 = 2;
    let mut st = Staging::default();
    st.b = [0xA5; 32];
    st.b[B_FSTATE] = 10;
    st.b[B_FLAGS] = 0; // no damage interaction; all unrelated slot cells stay dirty.
    st.b[B_SCRX] = 191;
    st.b[B_T28] = 0;
    obj.handler_wario26(&mut st, &tables());
    assert_eq!(st.b, [0; 32], "B15B blanks all 32 staged bytes");
    assert_eq!(
        (obj.done, obj.boss_def_count, obj.saved_power_state),
        (1, 7, 0xFE)
    );
    assert_eq!(obj.scene_phase_set, Some(3));
    let mut cam = CameraSim::new(80, 72);
    cam.scene_state = 0xF0;
    cam.boss_fight_phase = 2;
    cam.apply_object_result(&mut obj);
    assert_eq!((cam.boss_fight_phase, cam.scene_state), (3, 0));
    assert_eq!(obj.scene_phase_set, None);
    cam.scene_state = 9;
    cam.boss_fight_phase = 8;
    cam.apply_object_result(&mut obj);
    assert_eq!((cam.boss_fight_phase, cam.scene_state), (8, 9));
}

#[test]
fn form3_jump_tests_saved_byte_for_zero_only() {
    use crate::objects::{Staging, B_PHASE, B_T24};
    let tables = tables();
    for saved in [0, 1, 2, 3] {
        for leg in [0, 1] {
            let mut obj = ObjectSim::new(vec![], 0);
            obj.saved_power_state = saved;
            obj.mario.powerup = 2; // deliberately disagrees with nonzero saved values.
            let mut st = Staging::default();
            st.b[B_FSTATE] = 6;
            st.b[B_SCRX] = 80;
            st.b[B_PHASE] = 14;
            st.b[B_T24] = leg;
            obj.handler_wario26(&mut st, &tables);
            let expected = if saved == 0 { (15, leg) } else { (0, leg ^ 1) };
            assert_eq!((st.b[B_PHASE], st.b[B_T24]), expected);
            assert_eq!(obj.saved_power_state, saved);
            assert!(obj.unported.is_empty());
        }
    }
}
