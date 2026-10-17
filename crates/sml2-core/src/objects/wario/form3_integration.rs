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
    assert_eq!(tables.boss_anim_handler(26), Some(0xAD93), "Wario form 3 boss main body");
    assert_eq!(tables.boss_anim_handler(27), Some(0xAFE0), "Wario trajectory child");
    assert_eq!(tables.boss_anim_handler(28), Some(0xB0C2), "Wario drop child");
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
