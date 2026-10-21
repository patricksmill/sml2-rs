use super::*;
use sml2_core::{level::LAYOUT_SIZE, objects::*};

/// `$2E5C` sets the scene bounce's no-cut latch before `$2D7D` writes
/// the final 48/75 jump index. Exercise contact production, collision,
/// write-back, and `$14FA` on the following frame with A released.
#[test]
fn boss_stomp_preserves_the_high_jump_after_releasing_a() {
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    for (scene_state, a_held, initial_index, next_index, next_y) in [
        (0xF0, true, 75, 74, 105),
        (0xF0, false, 48, 47, 105),
        (0, true, 75, 32, 106),
    ] {
        let mut player = PlayerSim::new();
        player.x = 258;
        player.y = 120;
        player.vvel = 10;
        player.on_ground = false;
        let mut objects = ObjectSim::new(Vec::new(), 336);
        // Isolate a stompable contact from handler movement: the empty
        // definition skips the subsequent engine update. Zero hitbox offsets
        // put Mario six pixels inside the top of the collision rectangle.
        objects.slots[0].b[B_TYPE] = 0x45;
        objects.slots[0].b[B_SCRX] = 18;
        objects.slots[0].b[B_SCRY] = 144;
        let contact = build_contact(
            &player,
            &PlayerKeys {
                a: a_held,
                ..Default::default()
            },
            0,
            scene_state,
        );
        objects.step(336, 88, &layout, &tables, &contact, false);
        assert!(objects.stomp_jump);
        assert_eq!(objects.stomp_bounce, scene_state != 0);
        assert_eq!(objects.slots[0].b[B_FLAGS], 1);
        player.apply_object_result(&mut objects, a_held);
        assert_eq!(player.y.to_le_bytes(), [108, 0]);
        assert_eq!(player.vvel, initial_index);
        assert_eq!(player.no_jump_cut, scene_state != 0);
        player.step(&PlayerKeys::default(), false, false, &layout);
        assert_eq!(player.vvel, next_index);
        assert_eq!(player.y.to_le_bytes(), [next_y, 0]);
    }
}
