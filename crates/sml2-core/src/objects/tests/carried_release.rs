use super::*;
use crate::player_sim::{PlayerKeys, PlayerSim};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn release(objects: &mut ObjectSim, layout: &LevelLayout, tables: &ObjectTables) -> Staging {
    objects.configure(10);
    objects.fff0 = 1; // `$5B607` does not advance the sprite on this update.
    objects.action_latch = 1;
    objects.mario = MarioContact {
        x: 100,
        y: 200,
        powerup: 2,
        dir_right: true,
        ..Default::default()
    };
    objects.mario_ref_x = 108;
    let mut stage = Staging::default();
    stage.b = std::array::from_fn(|i| (i as u8).wrapping_mul(13));
    stage.b[B_DEF] = 4;
    stage.b[B_RECHI] = 0xAB;
    stage.b[B_STATE] = 2;
    stage.b[B_FSTATE] = 1;
    stage.b[B_TYPE] = 0;
    stage.b[B_FLIP] = 0x20;
    let mut expected = stage.b;
    expected[0..2].copy_from_slice(&[0, 124]); // Mario X + 8 + 16.
    expected[3..5].copy_from_slice(&[0, 205]); // Big Mario Y + 5.
    expected[B_DEF] = 3;
    expected[B_SPRITE] = 7; // `$4BC1[3]`.
    expected[14..31].fill(0); // `$8240`, including state, contact, and timers.
    objects.handler_carried(&mut stage, layout, tables);
    assert_eq!(stage.b, expected);
    assert_eq!((objects.action_latch, objects.done), (0, 1));
    stage
}

#[test]
fn carried_release_resets_the_slot_before_initializing_a_live_kick() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut objects = ObjectSim::new(Vec::new(), 0);
    let mut stage = release(&mut objects, &layout, &tables());
    objects.handler_kicked(&mut stage, &layout);
    assert_eq!(stage.x().to_be_bytes(), [0, 124]);
    assert_eq!(stage.b[B_FSTATE], 1);
    assert_eq!(stage.b[B_TYPE], 0x5F);
    assert_eq!(stage.b[B_FLIP], 0);
}

#[test]
fn released_shell_contact_removes_the_bunny_powerup_on_the_next_player_update() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut objects = ObjectSim::new(Vec::new(), 0);
    let mut stage = release(&mut objects, &layout, &tables());
    objects.handler_kicked(&mut stage, &layout);
    // Project at camera (80,192), with sprite 7's ROM hitbox.
    stage.b[B_SCRX] = 140;
    stage.b[B_SCRY] = 111;
    stage.b[19..23].copy_from_slice(&[0xF3, 0xFD, 0xFA, 6]);
    objects.slots[0].b = stage.b;
    objects.mario = MarioContact {
        x: 120,
        y: 200,
        powerup: 2,
        vvel: 20,
        ..Default::default()
    };
    objects.collide(80, 192, &layout);
    assert_eq!(objects.mario_dead, 15);
    assert_eq!(objects.slots[0].b[B_FLAGS], 0x10);
    let mut player = PlayerSim::new();
    player.x = 120;
    player.y = 200;
    player.powerup = 2;
    player.vvel = 20;
    player.apply_object_result(&mut objects, false);
    assert_eq!((player.powerup, player.dead), (2, 15));
    player.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(
        (player.powerup, player.dead, player.invincibility),
        (1, 0, 128)
    );
}
