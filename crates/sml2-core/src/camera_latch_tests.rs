//! Source `$54A7` publishes screen X after `$0B2A`; edge checks read that
//! previous draw (`$0B5E`, `$0BE5`, `$0DAF`), not this update's player X.
use super::CameraSim;
use crate::player_sim::PlayerSim;

#[test]
fn right_edge_consumes_previous_draw_before_latching_scene_17() {
    let mut camera = CameraSim::new(0x1B0, 0xC3);
    camera.prev_mario_y_lo = 0xA4;
    camera.edge_now = Some(1);
    camera.scene_map = vec![0; 48];
    camera.scene_map[1] = 17;
    let mut player = PlayerSim::new();
    player.x = 495;
    player.y = 164;
    camera.update_enables(player.x);
    assert_eq!(camera.mario_screen_x, 159);

    player.x = 497;
    camera.step(&mut player);
    assert_eq!(
        (camera.transition, camera.scene_id, camera.map_event_flag),
        (0, 0, 0)
    );
    assert_eq!(
        (player.x, player.y, camera.x, camera.y),
        (497, 164, 432, 195)
    );
    camera.update_enables(player.x);
    assert_eq!(camera.mario_screen_x, 161);

    player.x = 499;
    camera.step(&mut player);
    assert_eq!(
        (camera.transition, camera.scene_id, camera.map_event_flag),
        (1, 17, 255)
    );
    assert_eq!(
        (player.x, player.y, camera.x, camera.y),
        (499, 164, 432, 195)
    );
}

#[test]
fn left_edge_waits_for_previous_draw_below_0f_before_warping() {
    let mut camera = CameraSim::new(0x50, 0x48);
    camera.prev_mario_y_lo = 0x48;
    camera.edge_now = Some(2);
    let mut player = PlayerSim::new();
    player.x = 0x1FF;
    player.y = 0x48;
    camera.update_enables(player.x);
    assert_eq!(camera.mario_screen_x, 0x0F);
    player.x -= 1;
    camera.step(&mut player);
    assert_eq!((camera.transition, player.x), (0, 0x1FE));
    camera.update_enables(player.x);
    assert_eq!(camera.mario_screen_x, 0x0E);
    player.x -= 1;
    camera.step(&mut player);
    assert_eq!((camera.transition, player.x), (2, 0x200));
}

#[test]
fn autoscroll_edge_uses_previous_draw_and_level_reset_seeds_40() {
    let mut camera = CameraSim::new(0x1B0, 0x48);
    assert_eq!(camera.mario_screen_x, 0x40);
    camera.auto_scroll = 1;
    camera.prev_mario_y_lo = 0x48;
    camera.edge_now = Some(1);
    let mut player = PlayerSim::new();
    player.x = 496;
    player.y = 0x48;
    camera.update_enables(player.x);
    assert_eq!(camera.mario_screen_x, 0xA0);
    player.x += 1;
    camera.step(&mut player);
    assert_eq!((camera.transition, camera.x), (0, 432));
    camera.update_enables(player.x);
    camera.step(&mut player);
    assert_eq!((camera.transition, camera.x), (1, 432));
    camera.reset_level_start(0x1B0, 0x48, 0x48);
    assert_eq!(camera.mario_screen_x, 0x40);
}
