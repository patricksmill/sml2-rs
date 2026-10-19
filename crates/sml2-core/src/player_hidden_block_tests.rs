//! Hidden-block ceiling contact and collision-visible foothold, `$15B6/$1993/$5B3C`.
use crate::{
    level::{LevelLayout, LAYOUT_SIZE},
    player_sim::{PlayerKeys, PlayerSim},
};

#[test]
fn hidden_block_bonk_produces_a_solid_foothold_for_the_floor_probe() {
    let mut cells = vec![0x60; LAYOUT_SIZE];
    cells[30 * 256..31 * 256].fill(0x18);
    let block = 26 * 256 + 5;
    cells[block] = 0x7E;
    let layout = LevelLayout::from_bytes(&cells).unwrap();
    let mut jumper = PlayerSim::new();
    jumper.x = 72;
    jumper.y = 452;
    jumper.camx = 80;
    jumper.seed_moon_physics(8);
    for frame in 0..90 {
        jumper.frame_counter = frame;
        jumper.phase = frame & 7;
        jumper.step(
            &PlayerKeys {
                a: true,
                up: true,
                ..Default::default()
            },
            frame == 0,
            false,
            &layout,
        );
        jumper.block_update();
    }
    assert_eq!(layout.cells[block], 0x7E, "source layout stays immutable");
    assert!(
        jumper.layout_writes.contains(&(block as u32, 0x07)),
        "$5B3C used tile"
    );
    assert_eq!(jumper.coins_low, 0, "hidden blocks do not award a coin");

    // A falling player reads the produced overlay through the actual floor
    // probe. The original hidden tile alone cannot support this landing.
    for revealed in [false, true] {
        let mut falling = PlayerSim::new();
        falling.x = 72;
        falling.y = 372;
        falling.camx = 80;
        falling.on_ground = false;
        falling.vvel = 24;
        if revealed {
            falling.layout_writes = jumper.layout_writes.clone();
        }
        for _ in 0..30 {
            falling.step(&PlayerKeys::default(), false, false, &layout);
        }
        if revealed {
            assert!(falling.on_ground);
            assert_eq!((falling.y, falling.collision_tile), (388, 7));
        } else {
            assert!(
                falling.y > 388,
                "unrevealed $7E remains passable from above"
            );
        }
    }
}
