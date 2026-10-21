use super::*;
use sml2_core::{level::LAYOUT_SIZE, objects::*};

#[test]
fn disabled_tile_animation_preserves_vram_and_cannot_overwrite_arena_fire() {
    let mut player = PlayerSim::new();
    // Level 9: `$35EA` = $4B, stored as $4C at level load.
    player.tile_anim_config = 0x4C;
    let mut bg = vec![0xA5; 0x1000];
    let mut rows = vec![0; 16 * 0x100];
    rows[0xB40..0xB80].fill(0x39);
    let mut timer = 7;
    let mut phase = 0;
    update_tile_animation(&player, false, &mut bg, &rows, &mut timer, &mut phase);
    assert_eq!((timer, phase), (0, 1));
    assert_eq!(&bg[0xA00..0xA40], &[0x39; 64]);

    // `$6x` precedes `$2x` in the arena intro. `$01BF` skips animation
    // throughout that scene, even before `$2x` clears the config.
    bg[0xA00..0xA40].fill(0x6D);
    timer = 7;
    update_tile_animation(&player, true, &mut bg, &rows, &mut timer, &mut phase);
    assert_eq!((timer, phase), (8, 1));
    assert_eq!(&bg[0xA00..0xA40], &[0x6D; 64]);
    bg[0xA00..0xA40].fill(0x39);

    // `$20EB` clears the config. No later write, including the render pass,
    // may revert the last frame or paste the old level's row into arena art.
    player.tile_anim_config = 0;
    for _ in 0..16 {
        update_tile_animation(&player, false, &mut bg, &rows, &mut timer, &mut phase);
    }
    assert_eq!(&bg[0xA00..0xA40], &[0x39; 64]);
    bg[0xA00..0xA40].fill(0x6D); // `$25AF` reloads the arena's fire tiles.
    for _ in 0..32 {
        update_tile_animation(&player, false, &mut bg, &rows, &mut timer, &mut phase);
    }
    assert_eq!(&bg[0xA00..0xA40], &[0x6D; 64]);
    assert_eq!(phase, 1);
}

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
