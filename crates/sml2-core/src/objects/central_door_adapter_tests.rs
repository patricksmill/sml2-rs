use crate::level::LevelLayout;
use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0x98c8_dispatch_runs_full_cycle_and_carries_rider() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.boss_anim_handler(8), Some(0x98C8));
    assert_eq!(
        &tables.boss_picker_profile[..22],
        &[0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3]
    );
    let mut sim = ObjectSim::new(vec![], 0);
    sim.door_sequence_active = 1;
    sim.door_phase = 2;
    sim.slots[0].b[B_DEF] = 8;
    sim.slots[0].b[B_SCRY] = 116;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(sim.slots[0].b[B_FSTATE], 1);
    assert_eq!(sim.slots[0].b[B_T24], 64);
    assert_eq!(sim.slots[0].b[B_T28], 64);
    for _ in 0..64 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(sim.slots[0].b[B_FSTATE], 2);
    assert_eq!(sim.slots[0].b[B_SPRITE], 19);
    assert_eq!(sim.sfx2, 5);
    sim.mario.y = 1;
    for _ in 0..32 {
        sim.slots[0].b[B_FLAGS] = 8;
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(sim.mario_y_adjust, Some(0xFFC1));
    assert_eq!(sim.slots[0].b[B_SCRY], 52);
    assert_eq!(sim.slots[0].b[B_FSTATE], 3);
    assert_eq!(sim.slots[0].b[B_T28], 32);
    for _ in 0..32 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(sim.door_sequence_active, 0);
    assert_eq!(sim.slots[0].b[B_FSTATE], 4);
    for _ in 0..64 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(sim.slots[0].b[B_SCRY], 116);
    assert_eq!(sim.slots[0].b[B_FSTATE], 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x98c8_reward_morph_reaches_existing_anim4_consumer() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.remove_mode = 2;
    sim.slots[0].b[B_DEF] = 8;
    sim.slots[0].b[B_FSTATE] = 255;
    sim.slots[0].b[B_PHASE] = 255;
    sim.boss_engine(0, 0, &layout, &tables);
    let raw = sim.slots[0].b;
    assert_eq!(
        (raw[B_DEF], raw[B_SCRY], raw[B_SCRX], raw[B_SPRITE]),
        (4, 116, 88, 9)
    );
    assert_eq!((raw[B_FSTATE], raw[B_PHASE], raw[B_TYPE]), (0, 0, 0));
    sim.boss_engine(0, 0, &layout, &tables);
    assert!(sim.unported.is_empty());
    assert_eq!(sim.slots[0].b[B_DEF], 4);
}

#[test]
fn handler_0x98c8_adapter_reports_invalid_profile_and_preserves_staging_scratch() {
    let mut tables = tables();
    tables.boss_picker_profile.clear();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut staging = Staging {
        px: 0x1234,
        py: 0x5678,
        ..Default::default()
    };
    staging.b[B_FSTATE] = 4;
    let before = staging.b;
    sim.handler_central_door(&mut staging, &tables);
    assert_eq!(staging.b, before);
    assert_eq!((staging.px, staging.py), (0x1234, 0x5678));
    assert_eq!(sim.unported, vec![0x98C8]);
}
