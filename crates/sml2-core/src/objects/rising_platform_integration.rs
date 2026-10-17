use super::*;
use crate::{
    camera::CameraSim,
    level::LevelLayout,
    object::ObjectTables,
    objects::{MarioContact, Staging},
    player_sim::PlayerSim,
};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn rising_platform_source_slot_bytes_and_equality_only_bottom() {
    let mut world = ObjectSim::new(vec![], 0);
    let mut staging = Staging::default();
    staging.b = [0xA5; 32];
    staging.b[15] = 0;
    let mut expected = staging.b;
    expected[15] = 3;
    expected[18] = 32;
    world.handler_rising_platform(&mut staging);
    assert_eq!(staging.b, expected);
    staging.b[23] = 8;
    expected[23] = 8;
    expected[15] = 1;
    expected[24] = 64;
    expected[25] = 64;
    world.handler_rising_platform(&mut staging);
    assert_eq!(staging.b, expected);
    assert_eq!(world.mario_y_adjust, None);

    staging.b[3..5].copy_from_slice(&0u16.to_be_bytes());
    expected = staging.b;
    expected[3..5].copy_from_slice(&0xFFFFu16.to_be_bytes());
    expected[24] = 63;
    world.mario.y = 0;
    world.handler_rising_platform(&mut staging);
    assert_eq!(staging.b, expected);
    assert_eq!(world.mario_y_adjust, Some(0xFFFF));
    world.handler_rising_platform(&mut staging);
    assert_eq!(world.mario_y_adjust, Some(0xFFFE));

    for (budget, expected_budget, expected_y) in [(64, 64, 0xFFFFu16), (65, 66, 0), (255, 0, 0)] {
        staging.b[15] = 2;
        staging.b[23] = 0;
        staging.b[24] = budget;
        staging.b[25] = 64;
        staging.b[28] = 1;
        staging.b[3..5].copy_from_slice(&0xFFFFu16.to_be_bytes());
        expected = staging.b;
        expected[24] = expected_budget;
        expected[28] = 0;
        expected[3..5].copy_from_slice(&expected_y.to_be_bytes());
        world.handler_rising_platform(&mut staging);
        assert_eq!(staging.b, expected, "source equality, budget {budget}");
        assert_eq!(world.mario_y_adjust, Some(0xFFFE));
    }
    for state in [4, 127, 255] {
        staging.b[15] = state;
        expected = staging.b;
        world.handler_rising_platform(&mut staging);
        assert_eq!(staging.b, expected);
    }
    assert_eq!(world.unported, [0x58E12]);
}

#[test]
fn rising_platform_real_slot_dispatch_all_profiles_and_death_gate() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    for (level, anim) in [(0, 47), (16, 47), (25, 97)] {
        assert_eq!(tables.profile_for(level, anim).unwrap().handler, 0x58E12);
        for dead in [0, 255] {
            let mut world = ObjectSim::new(vec![], 0);
            world.level_id = level;
            world.fff0 = 3;
            world.phase = false;
            world.mario_dead = dead;
            let mut actor = Actor::default();
            actor.position.x = 100;
            actor.position.y = 100;
            let mut slot = actor.to_raw_slot();
            slot[8] = 2;
            slot[5] = anim;
            world.slots[1].b = slot;
            let mario = MarioContact {
                travelling: true,
                ..Default::default()
            };
            world.step(100, 100, &layout, &tables, &mario, false);
            assert_eq!(world.slots[1].b[15], if dead == 0 { 3 } else { 0 });
            assert_eq!(world.slots[1].b[18], if dead == 0 { 32 } else { 0 });
            assert!(world.unported.is_empty());
        }
    }
}

#[test]
fn rising_platform_dispatch_player_and_next_camera_consume_carry() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let tables = tables();
    let mut world = ObjectSim::new(vec![], 0);
    world.mario.y = 320;
    let mut staging = Staging::default();
    staging.b[15] = 1;
    staging.b[23] = 8;
    staging.b[24] = 1;
    staging.b[25] = 64;
    staging.b[3..5].copy_from_slice(&0u16.to_be_bytes());
    world.dispatch_handler(0x58E12, &mut staging, &layout, &tables);
    assert_eq!(staging.b[3..5], [255, 255]);
    assert_eq!(world.mario_y_adjust, Some(319));
    let mut player = PlayerSim::new();
    player.y = 320;
    player.apply_object_result(&mut world, false);
    assert_eq!(player.y, 319);
    let mut camera = CameraSim::new(0, 400);
    camera.prev_mario_y_lo = 64;
    camera.step(&mut player);
    assert_eq!(camera.y, 399);
    assert_eq!(camera.prev_mario_y_lo, 63);
    assert!(world.unported.is_empty());
}
